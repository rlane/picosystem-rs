use core::panic::PanicInfo;
use cortex_m_rt::{exception, ExceptionFrame};
use pico::hal::pac;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        turn_on_leds();
    }
    //cortex_m::interrupt::disable();
    log::error!("{}", info);
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
    }
    loop {}
}

// Turn on LEDs during a hard fault.
#[exception]
unsafe fn HardFault(_: &ExceptionFrame) -> ! {
    turn_on_leds();
    loop {}
}

unsafe fn turn_on_leds() {
    const SIO_REGS: *mut pac::sio::RegisterBlock = 0xd0000000 as *mut pac::sio::RegisterBlock;
    (*SIO_REGS)
        .gpio_out_set
        .write(|w| w.bits((1 << 13) | (1 << 14) | (1 << 15)));
}
