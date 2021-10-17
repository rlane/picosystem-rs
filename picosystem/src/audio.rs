use embedded_hal::digital::v2::OutputPin;
use embedded_hal::digital::v2::ToggleableOutputPin;
use pico::hal;
use pico::hal::pac;
use pico::hal::pac::interrupt;
use rp2040_hal::gpio::dynpin::DynPin;

pub struct Audio;

pub struct StaticAudio {
    pin: DynPin,
    period_us: u32,
}

static mut STATIC_AUDIO: Option<StaticAudio> = None;
const TIMER_REGS: *mut pac::timer::RegisterBlock = 0x40054000 as *mut pac::timer::RegisterBlock;

impl Audio {
    pub fn new(mut pin: DynPin) -> Self {
        pin.into_push_pull_output();
        pin.set_low().unwrap();
        unsafe {
            assert!(STATIC_AUDIO.is_none());
            STATIC_AUDIO = Some(StaticAudio { pin, period_us: 0 });
        };
        Audio
    }

    pub fn start_tone(&mut self, freq: u32) {
        let period_us = 1_000_000 / freq;
        unsafe {
            if STATIC_AUDIO.as_mut().unwrap().period_us == 0 {
                start_timer(period_us);
            }
            STATIC_AUDIO.as_mut().unwrap().period_us = period_us;
            pac::NVIC::unmask(hal::pac::Interrupt::TIMER_IRQ_0);
        }
    }

    pub fn stop(&mut self) {
        pac::NVIC::mask(hal::pac::Interrupt::TIMER_IRQ_0);
        unsafe {
            STATIC_AUDIO.as_mut().unwrap().period_us = 0;
        }
    }
}

unsafe fn start_timer(period_us: u32) {
    (*TIMER_REGS).inte.write(|w| {
        w.alarm_0().set_bit();
        w
    });
    let now = (*TIMER_REGS).timerawl.read().bits();
    (*TIMER_REGS).alarm0.write(|w| w.bits(now + period_us));
    (*TIMER_REGS).intr.write(|w| {
        w.alarm_0().set_bit();
        w
    });
}

#[allow(non_snake_case)]
#[interrupt]
fn TIMER_IRQ_0() {
    unsafe {
        if let Some(s) = STATIC_AUDIO.as_mut() {
            s.pin.toggle().unwrap();
            start_timer(s.period_us);
        }
    }
}
