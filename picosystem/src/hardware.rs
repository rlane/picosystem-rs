use crate::display::Display;
use crate::{audio, dma, idle, input, usb_logger};
use embedded_hal::adc::OneShot;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::rate::*;
use pico::hal;
use pico::hal::pac;
use rp2040_hal::gpio::dynpin::DynPin;
use rp2040_hal::gpio::pin::bank0::Gpio26;
use rp2040_hal::gpio::pin::{FloatingInput, Pin};
use rp2040_hal::gpio::Pins;

use rp2040_hal::{
    clocks::{Clock, ClocksManager, InitError},
    pll::{common_configs::PLL_USB_48MHZ, setup_pll_blocking, PLLConfig},
    watchdog::Watchdog,
    xosc::setup_xosc_blocking,
};

pub struct Hardware {
    pub display: Display,
    pub red_led_pin: DynPin,
    pub green_led_pin: DynPin,
    pub blue_led_pin: DynPin,
    pub battery_pin: Pin<Gpio26, FloatingInput>,
    pub delay: cortex_m::delay::Delay,
    pub adc: hal::adc::Adc,
    pub input: input::Input,
    pub audio: audio::Audio,
    pub idle: idle::Idle,
}

impl Hardware {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut pac = pac::Peripherals::take().unwrap();
        let core = pac::CorePeripherals::take().unwrap();
        let mut watchdog = hal::watchdog::Watchdog::new(pac.WATCHDOG);

        // The default is to generate a 125 MHz system clock
        let clocks = Self::init_clocks_and_plls(
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

        #[cfg(feature = "wait-for-serial")]
        {
            // Wait for USB to be ready.
            delay.delay_ms(500);
            if usb_logger::connected() {
                // Wait for serial logger.
                delay.delay_ms(1000);
            }
        }

        log::info!("Logging initialized");

        log::info!("System clock: {}", clocks.system_clock.freq());

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

        let battery_pin = pins.gpio26.into_floating_input();
        let adc = hal::adc::Adc::new(pac.ADC, &mut pac.RESETS);

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
            battery_pin,
            adc,
            delay,
            input,
            audio,
            idle: idle::Idle::new(),
        }
    }

    // Copied and modified from rp2040_hal crate.
    fn init_clocks_and_plls(
        xosc_crystal_freq: u32,
        xosc_dev: pac::XOSC,
        clocks_dev: pac::CLOCKS,
        pll_sys_dev: pac::PLL_SYS,
        pll_usb_dev: pac::PLL_USB,
        resets: &mut pac::RESETS,
        watchdog: &mut Watchdog,
    ) -> Result<ClocksManager, InitError> {
        let xosc =
            setup_xosc_blocking(xosc_dev, xosc_crystal_freq.Hz()).map_err(InitError::XoscErr)?;

        // Configure watchdog tick generation to tick over every microsecond
        watchdog.enable_tick_generation((xosc_crystal_freq / 1_000_000) as u8);

        let mut clocks = ClocksManager::new(clocks_dev);

        const PLL_SYS_180MHZ: PLLConfig<Megahertz> = PLLConfig {
            vco_freq: Megahertz(716),
            refdiv: 1,
            post_div1: 4,
            post_div2: 1,
        };

        let pll_sys = setup_pll_blocking(
            pll_sys_dev,
            xosc.operating_frequency().into(),
            PLL_SYS_180MHZ,
            &mut clocks,
            resets,
        )
        .map_err(InitError::PllError)?;
        let pll_usb = setup_pll_blocking(
            pll_usb_dev,
            xosc.operating_frequency().into(),
            PLL_USB_48MHZ,
            &mut clocks,
            resets,
        )
        .map_err(InitError::PllError)?;

        clocks
            .init_default(&xosc, &pll_sys, &pll_usb)
            .map_err(InitError::ClockError)?;
        Ok(clocks)
    }

    pub fn draw(&mut self, func: impl FnOnce(&mut Display)) {
        if self.idle.check_idle(&mut self.input) {
            self.idle.enter_idle(&mut self.display);
        }
        self.display.draw(func);
    }

    pub fn read_battery_raw(&mut self) -> u16 {
        self.adc.read(&mut self.battery_pin).unwrap()
    }

    pub fn read_battery_raw_slow(&mut self) -> u16 {
        let mut sum: u32 = 0;
        let n = 100;
        for _ in 0..n {
            sum += self.read_battery_raw() as u32;
        }
        (sum / n) as u16
    }

    pub fn read_battery_fraction(&mut self) -> f32 {
        let high = 1680.0;
        let low = 1390.0;
        let raw = self.read_battery_raw() as f32;
        ((raw - low) / (high - low)).clamp(0.0, 1.0)
    }
}
