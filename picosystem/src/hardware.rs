use crate::display::Display;
use crate::{audio, dma, input, usb_logger};
use embedded_hal::digital::v2::OutputPin;
use embedded_time::rate::*;
use pico::hal;
use pico::hal::pac;
use pico::hal::prelude::*;
use rp2040_hal::gpio::dynpin::DynPin;
use rp2040_hal::gpio::Pins;

pub struct Hardware {
    pub display: Display,
    pub red_led_pin: DynPin,
    pub green_led_pin: DynPin,
    pub blue_led_pin: DynPin,
    pub delay: cortex_m::delay::Delay,
    pub input: input::Input,
    pub audio: audio::Audio,
}

impl Hardware {
    #[allow(clippy::new_without_default)]
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

        // Wait for USB to be ready.
        delay.delay_ms(500);
        if usb_logger::connected() {
            // Wait for serial logger.
            delay.delay_ms(1000);
        }

        log::info!("Logging initialized");

        let sio = hal::sio::Sio::new(pac.SIO);
        let pins = Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        let mut red_led_pin = pins.gpio14.into_push_pull_output();
        let mut green_led_pin = pins.gpio13.into_push_pull_output();
        let mut blue_led_pin = pins.gpio15.into_push_pull_output();

        red_led_pin.set_low().unwrap();
        green_led_pin.set_low().unwrap();
        blue_led_pin.set_low().unwrap();

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
            /*dma_channel=*/ unsafe { dma::DmaChannel::new(0) },
        );

        pac.RESETS.reset.modify(|_, w| w.dma().clear_bit());
        while pac.RESETS.reset_done.read().dma().bit_is_clear() {}

        let input = input::Input::new(
            pins.gpio22.into(),
            pins.gpio21.into(),
            pins.gpio23.into(),
            pins.gpio20.into(),
            pins.gpio17.into(),
            pins.gpio16.into(),
            pins.gpio18.into(),
            pins.gpio19.into(),
        );

        let audio = audio::Audio::new(pins.gpio11.into());

        Hardware {
            display,
            red_led_pin: red_led_pin.into(),
            green_led_pin: green_led_pin.into(),
            blue_led_pin: blue_led_pin.into(),
            delay,
            input,
            audio,
        }
    }
}
