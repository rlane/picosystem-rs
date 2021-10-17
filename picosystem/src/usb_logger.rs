// Based on https://github.com/rp-rs/rp-hal/blob/c8bb2e43c792dd3975a255d7eba479547411aec6/boards/pico/examples/pico_usb_serial_interrupt.rs
use crate::time;
use core::fmt;
use core::fmt::Write;
use log::LevelFilter;
use log::{Level, Metadata, Record};
use pico::hal;
use pico::hal::pac;
use pico::hal::pac::interrupt;
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

/// The USB Device Driver (shared with the interrupt).
static mut USB_DEVICE: Option<UsbDevice<hal::usb::UsbBus>> = None;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

/// The USB Serial Device Driver (shared with the interrupt).
static mut USB_SERIAL: Option<SerialPort<hal::usb::UsbBus>> = None;

static LOGGER: UsbSerialLogger = UsbSerialLogger;

pub fn init(
    regs: pac::USBCTRL_REGS,
    dpram: pac::USBCTRL_DPRAM,
    resets: &mut pac::RESETS,
    clock: hal::clocks::UsbClock,
) {
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(regs, dpram, clock, true, resets));
    unsafe {
        USB_BUS = Some(usb_bus);
    }
    let usb_bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

    let serial = SerialPort::new(usb_bus_ref);

    // Create a USB device with a fake VID and PID
    let usb_dev = UsbDeviceBuilder::new(usb_bus_ref, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();

    unsafe {
        USB_DEVICE = Some(usb_dev);
        USB_SERIAL = Some(serial);
    }

    unsafe {
        log::set_logger_racy(&LOGGER)
            .map(|()| log::set_max_level(LevelFilter::Info))
            .unwrap();
    }

    // Enable the USB interrupt
    unsafe {
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
    };
}

#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    let usb_dev = USB_DEVICE.as_mut().unwrap();
    let serial = USB_SERIAL.as_mut().unwrap();

    if usb_dev.poll(&mut [serial]) {
        let mut buf = [0u8; 64];
        match serial.read(&mut buf) {
            Ok(0) => {}
            Ok(count) => {
                buf.iter_mut().take(count).for_each(|b| {
                    if *b == 0 {
                        log::info!("Entering flash mode");
                        hal::rom_data::reset_to_usb_boot(0, 0);
                    }
                });
            }
            Err(_) => {}
        }
    }
}

struct UsbSerialLogger;

impl log::Log for UsbSerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut writer = UsbSerialWriter;
            pac::NVIC::mask(hal::pac::Interrupt::USBCTRL_IRQ);
            write!(
                &mut writer,
                "{:.3} {} - {}\r\n",
                time::time_us() as f32 / 1000.0,
                record.level(),
                record.args()
            )
            .unwrap();
            unsafe {
                pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
            }
        }
    }

    fn flush(&self) {}
}

struct UsbSerialWriter;

impl fmt::Write for UsbSerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            let serial = USB_SERIAL.as_mut().unwrap();
            let _ = serial.write(s.as_bytes());
        }
        Ok(())
    }
}
