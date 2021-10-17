#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use log::info;
use rlane_picosystem_games as rpsg;
use rpsg::hardware;

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[entry]
fn main() -> ! {
    let mut hw = hardware::Hardware::new();

    info!("Finished initialization");

    hw.blue_led_pin.set_low().unwrap();

    let src: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let dst: [u8; 8] = [0; 8];

    info!("initial src={:?}", src);
    info!("initial dst={:?}", dst);

    unsafe {
        let dma_base = 0x50000000 as *mut u32;
        let ch0_read_addr = dma_base.offset(0);
        let ch0_write_addr = dma_base.offset(1);
        let ch0_trans_count = dma_base.offset(2);
        let ch0_ctrl_trig = dma_base.offset(3);
        ch0_read_addr.write_volatile(src.as_ptr() as u32);
        ch0_write_addr.write_volatile(dst.as_ptr() as u32);
        ch0_trans_count.write_volatile(16);
        cortex_m::asm::dmb();

        info!(
            "initial read_addr={:?} write_addr={:?} trans_count={:?} ctrl_trig={:?}",
            ch0_read_addr.read_volatile(),
            ch0_write_addr.read_volatile(),
            ch0_trans_count.read_volatile(),
            ch0_ctrl_trig.read_volatile()
        );

        ch0_ctrl_trig.write_volatile(1 | (0x3f << 15) | (1 << 5) | (1 << 4));
        cortex_m::asm::dmb();
        while ch0_trans_count.read_volatile() > 0 {}

        hw.blue_led_pin.set_high().unwrap();

        info!("Finished DMA");

        info!("final   src={:?}", src);
        info!("final   dst={:?}", dst);

        info!(
            "final   read_addr={:?} write_addr={:?} trans_count={:?} ctrl_trig={:?}",
            ch0_read_addr.read_volatile(),
            ch0_write_addr.read_volatile(),
            ch0_trans_count.read_volatile(),
            ch0_ctrl_trig.read_volatile()
        );
    }

    loop {}
}
