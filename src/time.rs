pub fn time_us() -> u32 {
    unsafe {
        let timer_base = 0x40054000 as *mut u32;
        let timerawl = timer_base.offset(10);
        timerawl.read_volatile()
    }
}
