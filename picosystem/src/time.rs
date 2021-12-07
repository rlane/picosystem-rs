pub fn time_us() -> u32 {
    unsafe { (*rp2040_pac::TIMER::PTR).timerawl.read().bits() }
}

pub fn time_us64() -> u64 {
    unsafe {
        (*rp2040_pac::TIMER::PTR).timelr.read().bits() as u64
            | (((*rp2040_pac::TIMER::PTR).timehr.read().bits() as u64) << 32)
    }
}
