#![no_std]

pub mod map;
pub mod sprite;
pub mod tile;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod audio;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod display;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod dma;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod fps_monitor;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod hardware;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod idle;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod input;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod time;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod time_tracker;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod interrupts;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod usb_logger;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod panic;
