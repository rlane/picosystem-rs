#![no_std]
#![no_main]

use core::fmt;
use core::fmt::Write;
use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::rate::*;
use log::info;
use log::LevelFilter;
use pico::hal;
use pico::hal::pac;
use pico::hal::pac::interrupt;
use pico::hal::prelude::*;
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

/// The USB Device Driver (shared with the interrupt).
static mut USB_DEVICE: Option<UsbDevice<hal::usb::UsbBus>> = None;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

/// The USB Serial Device Driver (shared with the interrupt).
static mut USB_SERIAL: Option<SerialPort<hal::usb::UsbBus>> = None;

static LOGGER: UsbSerialLogger = UsbSerialLogger;

#[entry]
fn main() -> ! {
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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    setup_usb_logging(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        &mut pac.RESETS,
        clocks.usb_clock,
    );

    let sio = hal::sio::Sio::new(pac.SIO);
    let pins = pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.gpio15.into_push_pull_output();

    let mut i = 0;
    loop {
        info!("Info log {}", i);
        log::error!("Error log");
        led_pin.set_high().unwrap();
        delay.delay_ms(500);
        led_pin.set_low().unwrap();
        delay.delay_ms(500);
        i += 1;
    }
}

fn setup_usb_logging(
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

fn write_to_serial(buf: &[u8]) {
    unsafe {
        pac::NVIC::mask(hal::pac::Interrupt::USBCTRL_IRQ);
        let serial = USB_SERIAL.as_mut().unwrap();
        let _ = serial.write(buf);
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
    }
}

#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    let usb_dev = USB_DEVICE.as_mut().unwrap();
    let serial = USB_SERIAL.as_mut().unwrap();

    if usb_dev.poll(&mut [serial]) {
        let mut buf = [0u8; 64];
        let _ = serial.read(&mut buf);
    }
}

use log::{Level, Metadata, Record};

struct UsbSerialLogger;

impl log::Log for UsbSerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut writer = UsbSerialWriter;
            write!(&mut writer, "{} - {}\r\n", record.level(), record.args()).unwrap();
        }
    }

    fn flush(&self) {}
}

struct UsbSerialWriter;

impl fmt::Write for UsbSerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_to_serial(s.as_bytes());
        Ok(())
    }
}
