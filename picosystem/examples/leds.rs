#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use log::info;
use picosystem::hardware;

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    info!("Finished initialization");

    hw.red_led_pin.set_low().unwrap();
    hw.green_led_pin.set_low().unwrap();
    hw.blue_led_pin.set_low().unwrap();

    loop {
        hw.red_led_pin.set_high().unwrap();
        hw.delay.delay_ms(500);
        hw.red_led_pin.set_low().unwrap();

        hw.green_led_pin.set_high().unwrap();
        hw.delay.delay_ms(500);
        hw.green_led_pin.set_low().unwrap();

        hw.blue_led_pin.set_high().unwrap();
        hw.delay.delay_ms(500);
        hw.blue_led_pin.set_low().unwrap();
    }
}
