use core::panic::PanicInfo;
use pico::hal::pac;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    //cortex_m::interrupt::disable();
    log::error!("{}", info);
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
    }
    loop {}
}
