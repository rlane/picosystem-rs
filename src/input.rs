use embedded_hal::digital::v2::InputPin;
use rp2040_hal::gpio::dynpin::DynPin;

pub struct Input {
    dpad_left_pin: DynPin,
    dpad_right_pin: DynPin,
    dpad_up_pin: DynPin,
    dpad_down_pin: DynPin,
    button_x_pin: DynPin,
    button_y_pin: DynPin,
    button_a_pin: DynPin,
    button_b_pin: DynPin,
}

impl Input {
    pub fn new(
        mut dpad_left_pin: DynPin,
        mut dpad_right_pin: DynPin,
        mut dpad_up_pin: DynPin,
        mut dpad_down_pin: DynPin,
        mut button_x_pin: DynPin,
        mut button_y_pin: DynPin,
        mut button_a_pin: DynPin,
        mut button_b_pin: DynPin,
    ) -> Self {
        dpad_left_pin.into_pull_down_input();
        dpad_right_pin.into_pull_down_input();
        dpad_up_pin.into_pull_down_input();
        dpad_down_pin.into_pull_down_input();
        button_x_pin.into_pull_down_input();
        button_y_pin.into_pull_down_input();
        button_a_pin.into_pull_down_input();
        button_b_pin.into_pull_down_input();
        Input {
            dpad_left_pin,
            dpad_right_pin,
            dpad_up_pin,
            dpad_down_pin,
            button_x_pin,
            button_y_pin,
            button_a_pin,
            button_b_pin,
        }
    }

    pub fn dpad_left(&self) -> bool {
        self.dpad_left_pin.is_low().unwrap()
    }

    pub fn dpad_right(&self) -> bool {
        self.dpad_right_pin.is_low().unwrap()
    }

    pub fn dpad_up(&self) -> bool {
        self.dpad_up_pin.is_low().unwrap()
    }

    pub fn dpad_down(&self) -> bool {
        self.dpad_down_pin.is_low().unwrap()
    }

    pub fn button_x(&self) -> bool {
        self.button_x_pin.is_low().unwrap()
    }

    pub fn button_y(&self) -> bool {
        self.button_y_pin.is_low().unwrap()
    }

    pub fn button_a(&self) -> bool {
        self.button_a_pin.is_low().unwrap()
    }

    pub fn button_b(&self) -> bool {
        self.button_b_pin.is_low().unwrap()
    }
}
