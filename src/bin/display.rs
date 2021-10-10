#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::rate::*;
use log::info;
use pico::hal;
use pico::hal::pac;
use pico::hal::prelude::*;
use rlane_picosystem_games as rpsg;
use rpsg::usb_logger;

use display_interface_spi::SPIInterface;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_hal::spi::MODE_0;
use hal::gpio::FunctionSpi;
use hal::spi::Spi;
use st7789::{Orientation, ST7789};

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = hal::watchdog::Watchdog::new(pac.WATCHDOG);

    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());
    delay.delay_ms(200);

    usb_logger::init(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        &mut pac.RESETS,
        clocks.usb_clock,
    );

    let sio = hal::sio::Sio::new(pac.SIO);
    let pins = pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut blue_led_pin = pins.gpio15.into_push_pull_output();
    let mut backlight_pin = pins.gpio12.into_push_pull_output();
    let mut lcd_cs_pin = pins.gpio5.into_push_pull_output();
    let _lcd_sck_pin = pins.gpio6.into_mode::<FunctionSpi>();
    let _lcd_mosi_pin = pins.gpio7.into_mode::<FunctionSpi>();
    let mut lcd_vsync_pin = pins.gpio8.into_push_pull_output();
    let mut lcd_dc_pin = pins.gpio9.into_push_pull_output();
    let mut lcd_reset_pin = pins.gpio4.into_push_pull_output();

    let spi = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        125_000_000u32.Hz(),
        16_000_000u32.Hz(),
        &MODE_0,
    );

    info!("Initializing display");
    let di = SPIInterface::new(spi, lcd_dc_pin, lcd_cs_pin);
    let mut display = ST7789::new(di, lcd_reset_pin, 240, 240);
    display.init(&mut delay).unwrap();

    info!("Drawing");
    let circle1 =
        Circle::new(Point::new(128, 64), 64).into_styled(PrimitiveStyle::with_fill(Rgb565::RED));
    let circle2 = Circle::new(Point::new(64, 64), 64)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1));
    display.clear(Rgb565::BLACK).unwrap();
    circle1.draw(&mut display).unwrap();
    circle2.draw(&mut display).unwrap();

    backlight_pin.set_high().unwrap();

    info!("Finished initialization");
    let mut i = 0;
    loop {
        info!("Info log {}", i);
        log::error!("Error log");
        blue_led_pin.set_high().unwrap();
        delay.delay_ms(500);
        blue_led_pin.set_low().unwrap();
        delay.delay_ms(500);
        i += 1;
    }
}
