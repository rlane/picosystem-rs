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

    pub fn run<F: FnOnce()>(&mut self, func: F) {
        let start = time::time_us();
        func();
        self.update(time::time_us() - start);
    }

    pub fn start<'a>(&'a mut self) -> Tracked<'a> {
        Tracked {
            tracker: self,
            start: time::time_us(),
        }
    }

    pub fn update(&mut self, elapsed: u32) {
        self.accumulated_us += elapsed;
        self.count += 1;
        if time::time_us() - self.last_display_us >= 1_000_000 {
            info!(
                "TimeTracker {}: {} us/call {} us/sec",
                &self.name,
                self.accumulated_us / self.count,
                (self.accumulated_us * 1000) / ((time::time_us() - self.last_display_us) / 1000)
            );
            self.last_display_us = time::time_us();
            self.count = 0;
            self.accumulated_us = 0;
        }
    }
}

pub struct Tracked<'a> {
    tracker: &'a mut TimeTracker,
    start: u32,
}

impl Drop for Tracked<'_> {
    fn drop(&mut self) {
        self.tracker.update(time::time_us() - self.start);
    }
}
