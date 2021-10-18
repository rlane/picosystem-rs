pub fn time_us() -> u32 {
    unsafe { (*rp2040_pac::TIMER::PTR).timerawl.read().bits() }
}
