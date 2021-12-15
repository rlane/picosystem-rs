use crate::time;
use embedded_hal::digital::v2::InputPin;
use rp2040_hal::gpio::dynpin::DynPin;

const DEBOUNCE_US: u64 = 30_000;
const REPEAT_US: u64 = 200_000;

pub struct Button {
    pin: DynPin,
    press_inhibit: bool,
    last_held_time: u64,
    last_repeat_time: u64,
}

impl Button {
    pub fn new(mut pin: DynPin) -> Button {
        pin.into_pull_down_input();
        Button {
            pin,
            press_inhibit: false,
            last_held_time: 0,
            last_repeat_time: 0,
        }
    }

    pub fn is_held(&self) -> bool {
        self.pin.is_low().unwrap()
    }

    pub fn is_pressed(&mut self) -> bool {
        if self.is_held() {
            let now = time::time_us64();
            self.last_held_time = now;
            if self.press_inhibit {
                if now - self.last_repeat_time > REPEAT_US {
                    self.last_repeat_time = now;
                    true
                } else {
                    false
                }
            } else {
                self.press_inhibit = true;
                self.last_repeat_time = now;
                true
            }
        } else if self.press_inhibit && time::time_us64() > self.last_held_time + DEBOUNCE_US {
            self.press_inhibit = false;
            false
        } else {
            false
        }
    }
}

pub struct Input {
    pub dpad_left: Button,
    pub dpad_right: Button,
    pub dpad_up: Button,
    pub dpad_down: Button,
    pub button_x: Button,
    pub button_y: Button,
    pub button_a: Button,
    pub button_b: Button,
}

impl Input {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dpad_left_pin: DynPin,
        dpad_right_pin: DynPin,
        dpad_up_pin: DynPin,
        dpad_down_pin: DynPin,
        button_x_pin: DynPin,
        button_y_pin: DynPin,
        button_a_pin: DynPin,
        button_b_pin: DynPin,
    ) -> Self {
        Input {
            dpad_left: Button::new(dpad_left_pin),
            dpad_right: Button::new(dpad_right_pin),
            dpad_up: Button::new(dpad_up_pin),
            dpad_down: Button::new(dpad_down_pin),
            button_x: Button::new(button_x_pin),
            button_y: Button::new(button_y_pin),
            button_a: Button::new(button_a_pin),
            button_b: Button::new(button_b_pin),
        }
    }

    pub fn is_active(&self) -> bool {
        for button in [
            &self.dpad_left,
            &self.dpad_right,
            &self.dpad_down,
            &self.dpad_up,
            &self.button_x,
            &self.button_y,
            &self.button_a,
            &self.button_b,
        ] {
            if button.is_held() {
                return true;
            }
        }
        false
    }
}
