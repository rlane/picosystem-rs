use crate::time;
use log::info;

pub struct FpsMonitor {
    last_time_us: u32,
    frames: u32,
}

impl FpsMonitor {
    const FPS_INTERVAL_US: u32 = 1_000_000;

    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            last_time_us: time::time_us(),
            frames: 0,
        }
    }

    pub fn update(&mut self) {
        let now = time::time_us();
        if now - self.last_time_us >= Self::FPS_INTERVAL_US {
            info!("FPS: {}", self.frames);
            self.last_time_us = now;
            self.frames = 0;
        } else {
            self.frames += 1;
        }
    }
}
