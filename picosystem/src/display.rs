use crate::dma::{self, DmaChannel};
use crate::time;
use core::convert::TryInto;
use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::{
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::*,
    primitives::Rectangle,
};
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal::spi::MODE_3;
use hal::pac;
use hal::spi::Spi;
use log::info;
use rp2040_hal as hal;
use rp2040_hal::gpio::dynpin::DynFunction;
use rp2040_hal::gpio::dynpin::DynPin;
use rp2040_hal::gpio::dynpin::DynPinMode;
use st7789::{TearingEffect, ST7789};
use fugit::RateExtU32;

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 240;

static mut FRAMEBUFFER: [u16; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

pub fn framebuffer() -> &'static mut [u16; WIDTH * HEIGHT] {
    unsafe { &mut FRAMEBUFFER }
}

pub type RealDisplay = st7789::ST7789<SPIInterfaceNoCS<Spi<hal::spi::Enabled, pac::SPI0, 8>, DynPin>, DynPin, DynPin>;

pub struct Display {
    st7789: RealDisplay,
    lcd_vsync_pin: DynPin,
    dma_channel: DmaChannel,
    last_vsync_time: u32,
}


/*
    let spi_screen =
        Spi::<_, _, 8>::new(hw.SPI0).init( p.RESETS, 125u32.MHz(), 16u32.MHz(), &MODE_0);
    let spii_screen = SPIInterface::new(spi_screen, hw.lcd_dc_pin, hw.lcd_cs_pin);
    let mut display = mipidsi::Builder::st7789(spii_screen)
        .with_display_size(240, 240)
        .with_framebuffer_size(240, 240)
        .init(&mut delay, Some(DummyPin))
        .unwrap();

*/

impl Display {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut backlight_pin: DynPin,
        mut lcd_dc_pin: DynPin,
        mut lcd_cs_pin: DynPin,
        mut lcd_sck_pin: DynPin,
        mut lcd_mosi_pin: DynPin,
        mut lcd_vsync_pin: DynPin,
        mut lcd_reset_pin: DynPin,
        spi_device: pac::SPI0,
        resets: &mut pac::RESETS,
        delay_source: &mut impl DelayUs<u32>,
        dma_channel: DmaChannel,
    ) -> Display {
        info!("Initializing display");
        backlight_pin.into_push_pull_output();
        lcd_dc_pin.into_push_pull_output();
        lcd_cs_pin.into_push_pull_output();
        lcd_cs_pin.set_low().unwrap();
        lcd_sck_pin
            .try_into_mode(DynPinMode::Function(DynFunction::Spi))
            .unwrap();
        lcd_mosi_pin
            .try_into_mode(DynPinMode::Function(DynFunction::Spi))
            .unwrap();
        lcd_vsync_pin.into_floating_input();
        lcd_reset_pin.into_push_pull_output();
        let spi = Spi::<_, _, 8>::new(spi_device).init(
            resets,
            125.MHz(),
            62_500_000u32.Hz(),
            &MODE_3,
        );
        let di = SPIInterfaceNoCS::new(spi, lcd_dc_pin);
        let mut st7789 = ST7789::new(di, Some(lcd_reset_pin), Some(backlight_pin), WIDTH as u16, HEIGHT as u16);
        st7789.init(delay_source).unwrap();
        st7789.set_tearing_effect(TearingEffect::Vertical).unwrap();
        let mut display = Display {
            st7789,
            dma_channel,
            lcd_vsync_pin,
            last_vsync_time: 0,
        };
        // A single clear occasionally fails to clear the screen.
        for _ in 0..2 {
            // let colors =
                // core::iter::repeat(RawU16::from(Rgb565::BLACK).into_inner()).take(WIDTH * HEIGHT);
            let colors = core::iter::repeat(Rgb565::BLACK.into_storage()).take(WIDTH * HEIGHT);
            display
                .st7789
                .set_pixels(0, 0, (WIDTH - 1) as u16, (HEIGHT - 1) as u16, colors)
                .unwrap();
        }
        display.enable_backlight(delay_source);
        display
    }

    fn start_flush(&mut self) {
        unsafe {
            dma::start_copy_to_spi(
                &mut self.dma_channel,
                framebuffer().as_ptr() as u32,
                (*pac::SPI0::PTR).sspdr.as_ptr() as u32,
                1,
                (WIDTH * HEIGHT * 2) as u32,
            );
        }
    }

    fn wait_for_flush(&mut self) {
        self.dma_channel.wait();
    }

    pub fn flush(&mut self) {
        self.wait_for_vsync();
        self.start_flush();
        self.wait_for_flush();
    }

    pub fn draw(&mut self, func: impl FnOnce(&mut Self)) {
        self.wait_for_flush();
        func(self);
        self.wait_for_vsync();
        self.start_flush();
    }

    pub fn enable_backlight(&mut self, delay_source: &mut impl DelayUs<u32>) {
        self.st7789.set_backlight(st7789::BacklightState::On, delay_source).unwrap();
    }
    
    pub fn disable_backlight(&mut self, delay_source: &mut impl DelayUs<u32>) {
        self.st7789.set_backlight(st7789::BacklightState::Off, delay_source).unwrap();
    }

    pub fn wait_for_vsync(&mut self) {
/*         if self.last_vsync_time != 0 && time::time_us() - self.last_vsync_time > 16_000 {
            log::info!("Missed vsync");
        } */
        // log::info!("frametime {0}",time::time_us() - self.last_vsync_time);
        while self.lcd_vsync_pin.is_high().unwrap() {}
        while self.lcd_vsync_pin.is_low().unwrap() {}
        self.last_vsync_time = time::time_us();
    }

    pub fn flush_progress(&self) -> usize {
        if self.dma_channel.get_count() == 0 {
            return WIDTH * HEIGHT;
        }
        (self.dma_channel.get_src() as usize - framebuffer().as_ptr() as usize) / 2
    }
}

impl DrawTarget for Display {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        const M: u32 = WIDTH as u32 - 1;
        const N: u32 = HEIGHT as u32 - 1;
        let fb = framebuffer();
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok((x @ 0..=M, y @ 0..=N)) = coord.try_into() {
                let index: u32 = x + y * WIDTH as u32;
                let color = RawU16::from(color).into_inner();
                fb[index as usize] = color.to_be();
            }
        }
        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let clipped_area = area.intersection(&self.bounding_box());
        if area.bottom_right().is_none() || clipped_area.bottom_right().is_none() {
            return Ok(());
        }

        let skip_top_left = clipped_area.top_left - area.top_left;
        let skip_bottom_right = area.bottom_right().unwrap() - clipped_area.bottom_right().unwrap();

        let fb = framebuffer();
        let mut colors = colors.into_iter();

        for _ in 0..skip_top_left.y {
            for _ in 0..area.size.width {
                colors.next();
            }
        }

        for y in 0..clipped_area.size.height as i32 {
            for _ in 0..skip_top_left.x {
                colors.next();
            }

            let mut index = clipped_area.top_left.x + (clipped_area.top_left.y + y) * WIDTH as i32;
            for _ in 0..clipped_area.size.width {
                let color = colors.next().unwrap_or(Rgb565::RED);
                let color = RawU16::from(color).into_inner();
                fb[index as usize] = color.to_be();
                index += 1;
            }

            for _ in 0..skip_bottom_right.x {
                colors.next();
            }
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let color = RawU16::from(color).into_inner().to_be();
        unsafe {
            dma::set_mem(
                &mut self.dma_channel,
                &color as *const u16 as u32,
                framebuffer().as_ptr() as u32,
                2,
                (WIDTH * HEIGHT) as u32,
            );
        }
        if framebuffer()[0] != color {
            log::info!(
                "incorrect framebuffer[0], expected {} got {}",
                color,
                framebuffer()[0]
            );
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.fill_contiguous(area, core::iter::repeat(color))
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

pub struct XorDisplay<'a> {
    display: &'a mut Display,
}

impl<'a> XorDisplay<'a> {
    pub fn new(display: &'a mut Display) -> XorDisplay {
        XorDisplay { display }
    }
}

impl<'a> DrawTarget for XorDisplay<'a> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        const M: u32 = WIDTH as u32 - 1;
        let fb = framebuffer();
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok((x @ 0..=M, y @ 0..=M)) = coord.try_into() {
                let index: u32 = x + y * WIDTH as u32;
                let color = RawU16::from(color).into_inner();
                fb[index as usize] ^= color.to_be();
            }
        }

        Ok(())
    }
}

impl<'a> OriginDimensions for XorDisplay<'a> {
    fn size(&self) -> Size {
        self.display.size()
    }
}
