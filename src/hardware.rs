use crate::display::Display;
use crate::usb_logger;
use embedded_time::rate::*;
use pico::hal;
use pico::hal::pac;
use pico::hal::prelude::*;
use rp2040_hal::gpio::dynpin::DynPin;
use rp2040_hal::gpio::Pins;

pub struct Hardware {
    pub display: Display,
    pub blue_led_pin: DynPin,
    pub delay: cortex_m::delay::Delay,
}

impl Hardware {
    pub fn new() -> Self {
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

        let mut delay =
            cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

        usb_logger::init(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            &mut pac.RESETS,
            clocks.usb_clock,
        );
        delay.delay_ms(1500);
        log::info!("Logging initialized");

        let sio = hal::sio::Sio::new(pac.SIO);
        let pins = Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        let blue_led_pin = pins.gpio15.into_push_pull_output();

        let display = Display::new(
            /*backlight_pin=*/ pins.gpio12.into(),
            /*lcd_dc_pin=*/ pins.gpio9.into(),
            /*lcd_cs_pin=*/ pins.gpio5.into(),
            /*lcd_sck_pin=*/ pins.gpio6.into(),
            /*lcd_mosi_pin=*/ pins.gpio7.into(),
            /*lcd_vsync_pin=*/ pins.gpio8.into(),
            /*lcd_reset_pin=*/ pins.gpio4.into(),
            /*spi_device=*/ pac.SPI0,
            /*resets=*/ &mut pac.RESETS,
            /*delay_source=*/ &mut delay,
        );

        Hardware {
            display,
            blue_led_pin: blue_led_pin.into(),
            delay,
        }
    }
}
