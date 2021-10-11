#![no_std]
#![no_main]

use cortex_m_rt::entry;
use log::info;
use rlane_picosystem_games as rpsg;
use rpsg::hardware;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();
    hw.delay.delay_ms(2000);
    info!("Before panic");
    panic!("Panicking");
}
