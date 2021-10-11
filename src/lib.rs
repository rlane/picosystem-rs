#![no_std]

pub mod display;
pub mod hardware;
pub mod usb_logger;

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;
