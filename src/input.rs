use embedded_hal::digital::v2::InputPin;
use rp2040_hal::gpio::dynpin::DynPin;

pub struct Button {
    pin: DynPin,
}

impl Button {
    pub fn new(mut pin: DynPin) -> Button {
        pin.into_pull_down_input();
        Button { pin }
    }

    pub fn is_held(&self) -> bool {
        self.pin.is_low().unwrap()
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
}
