use core::convert::TryInto;
use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::{
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::*,
};
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::MODE_3;
use embedded_time::rate::*;
use hal::pac;
use hal::spi::Spi;
use log::info;
use rp2040_hal as hal;
use rp2040_hal::gpio::dynpin::DynFunction;
use rp2040_hal::gpio::dynpin::DynPin;
use rp2040_hal::gpio::dynpin::DynPinMode;
use st7789::ST7789;

const WIDTH: usize = 240;
const HEIGHT: usize = 240;

pub type RealDisplay =
    ST7789<SPIInterfaceNoCS<Spi<hal::spi::Enabled, pac::SPI0, 8>, DynPin>, DynPin>;

pub struct Display {
    st7789: RealDisplay,
    backlight_pin: DynPin,
    framebuffer: [u16; WIDTH * HEIGHT],
}

impl Display {
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
            125_000_000u32.Hz(),
            62_500_000u32.Hz(),
            &MODE_3,
        );
        let di = SPIInterfaceNoCS::new(spi, lcd_dc_pin);
        let mut st7789 = ST7789::new(di, lcd_reset_pin, WIDTH as u16, HEIGHT as u16);
        st7789.init(delay_source).unwrap();
        let mut display = Display {
            st7789,
            backlight_pin,
            framebuffer: [0; WIDTH * HEIGHT],
        };
        let colors =
            core::iter::repeat(RawU16::from(Rgb565::BLACK).into_inner()).take(WIDTH * HEIGHT);
        display
            .st7789
            .set_pixels(0, 0, (WIDTH - 1) as u16, (HEIGHT - 1) as u16, colors)
            .unwrap();
        display.enable_backlight();
        display
    }

    pub fn flush(&mut self) {
        unsafe {
            let dma_base = 0x50000000 as *mut u32;
            let spi0_tx = 0x4003c000 + 8;
            let ch0_read_addr = dma_base.offset(0);
            let ch0_write_addr = dma_base.offset(1);
            let ch0_trans_count = dma_base.offset(2);
            let ch0_ctrl_trig = dma_base.offset(3);
            ch0_read_addr.write_volatile(self.framebuffer.as_ptr() as u32);
            ch0_write_addr.write_volatile(spi0_tx);
            ch0_trans_count.write_volatile((WIDTH * HEIGHT * 2) as u32);
            ch0_ctrl_trig.write_volatile((1 << 0) | (0 << 2) | (1 << 4) | (16 << 15));
            while ch0_trans_count.read_volatile() > 0 {}
        }
    }

    pub fn enable_backlight(&mut self) {
        self.backlight_pin.set_high().unwrap();
    }

    pub fn disable_backlight(&mut self) {
        self.backlight_pin.set_low().unwrap();
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
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok((x @ 0..=M, y @ 0..=M)) = coord.try_into() {
                let index: u32 = x + y * WIDTH as u32;
                let color = RawU16::from(color).into_inner();
                self.framebuffer[index as usize] = color.to_be();
            }
        }

        Ok(())
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}
