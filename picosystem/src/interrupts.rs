use pico::hal::pac;
use pico::hal::pac::interrupt;

pub enum GpioEvent {
    Low = 1,
    High = 2,
    EdgeLow = 4,
    EdgeHigh = 8,
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn enable_gpio_interrupt(gpio: usize, event: GpioEvent) {
    let regs = &*pac::IO_BANK0::PTR;
    regs.proc0_inte[gpio / 8].modify(|r, w| w.bits(r.bits() | (event as u32) << (4 * (gpio % 8))));
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn disable_gpio_interrupt(gpio: usize, event: GpioEvent) {
    let regs = &*pac::IO_BANK0::PTR;
    regs.proc0_inte[gpio / 8]
        .modify(|r, w| w.bits(r.bits() & !((event as u32) << (4 * (gpio % 8)))));
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn unmask_gpio_interrupt() {
    pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
}

pub fn mask_gpio_interrupt() {
    pac::NVIC::mask(pac::Interrupt::IO_IRQ_BANK0);
}

pub fn acknowledge_gpio_interrupt() {
    unsafe {
        let regs = &*pac::IO_BANK0::PTR;
        for i in 0..3 {
            let v = regs.intr[i].read().bits();
            regs.intr[i].write(|w| w.bits(v));
        }
    }
}

#[allow(non_snake_case)]
#[interrupt]
unsafe fn IO_IRQ_BANK0() {
    acknowledge_gpio_interrupt();
}
