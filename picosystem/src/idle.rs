use cortex_m::delay::Delay;

use crate::{display, input, interrupts, time};

const IDLE_TIME_US: u64 = 300_000_000;

pub struct Idle {
    last_active_time: u64,
}

#[allow(clippy::new_without_default)]
impl Idle {
    pub fn new() -> Idle {
        Idle {
            last_active_time: 0,
        }
    }

    pub fn check_idle(&mut self, input: &mut input::Input) -> bool {
        let now = time::time_us64();
        if input.is_active() {
            self.last_active_time = now;
        } else if now - self.last_active_time > IDLE_TIME_US {
            return true;
        }
        false
    }

    pub fn enter_idle(&mut self, display: &mut display::Display, delay: &mut Delay) {
        display.disable_backlight(delay);
        unsafe {
            let inputs = 16..24;
            for gpio in inputs.clone() {
                interrupts::enable_gpio_interrupt(gpio, interrupts::GpioEvent::EdgeLow);
            }
            interrupts::acknowledge_gpio_interrupt();
            interrupts::unmask_gpio_interrupt();
            cortex_m::asm::wfi();
            interrupts::mask_gpio_interrupt();
            for gpio in inputs {
                interrupts::disable_gpio_interrupt(gpio, interrupts::GpioEvent::EdgeLow);
            }
        }
        display.enable_backlight(delay);
        self.last_active_time = time::time_us64();
    }
}