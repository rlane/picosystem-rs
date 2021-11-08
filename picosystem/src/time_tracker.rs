use crate::time;
use log::info;

pub struct TimeTracker {
    name: &'static str,
    last_display_us: u32,
    accumulated_us: u32,
    count: u32,
}

impl TimeTracker {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            last_display_us: 0,
            accumulated_us: 0,
            count: 0,
        }
    }

    pub fn run<F: FnOnce() -> ()>(&mut self, func: F) {
        let start = time::time_us();
        func();
        let end = time::time_us();
        let elapsed = end - start;
        self.accumulated_us += elapsed;
        self.count += 1;
        if end - self.last_display_us >= 1_000_000 {
            info!(
                "TimeTracker {}: {} us/call {} us/sec",
                &self.name,
                self.accumulated_us / self.count,
                (self.accumulated_us * 1000) / ((end - self.last_display_us) / 1000)
            );
            self.last_display_us = end;
            self.count = 0;
            self.accumulated_us = 0;
        }
    }
}
