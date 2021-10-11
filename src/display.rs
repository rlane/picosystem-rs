use core::convert::TryInto;
use display_interface_spi::SPIInterface;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::{
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::*,
};
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::MODE_0;
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
    ST7789<SPIInterface<Spi<hal::spi::Enabled, pac::SPI0, 8>, DynPin, DynPin>, DynPin>;

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
            16_000_000u32.Hz(),
            &MODE_0,
        );
        let di = SPIInterface::new(spi, lcd_dc_pin, lcd_cs_pin);
        let mut st7789 = ST7789::new(di, lcd_reset_pin, WIDTH as u16, HEIGHT as u16);
        st7789.init(delay_source).unwrap();
        st7789.clear(Rgb565::BLACK).unwrap();
        let mut display = Display {
            st7789,
            backlight_pin,
            framebuffer: [0; WIDTH * HEIGHT],
        };
        display.enable_backlight();
        display
    }

    pub fn flush(&mut self) {
        self.st7789
            .set_pixels(0, 0, WIDTH as u16, HEIGHT as u16, self.framebuffer)
            .unwrap();
    }

    pub fn enable_backlight(&mut self) {
        self.backlight_pin.set_high().unwrap();
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
                self.framebuffer[index as usize] = color;
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
