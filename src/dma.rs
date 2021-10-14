fn wordsize(elem_size: u32) -> u32 {
    match elem_size {
        1 => 0,
        2 => 1,
        4 => 2,
        _ => panic!("invalid DMA element size"),
    }
}

pub(crate) unsafe fn set_mem(src: u32, dst: u32, elem_size: u32, count: u32) {
    let dma_base = 0x50000000 as *mut u32;
    let ch0_read_addr = dma_base.offset(0);
    let ch0_write_addr = dma_base.offset(1);
    let ch0_trans_count = dma_base.offset(2);
    let ch0_ctrl_trig = dma_base.offset(3);
    ch0_read_addr.write_volatile(src);
    ch0_write_addr.write_volatile(dst);
    ch0_trans_count.write_volatile(count);
    ch0_ctrl_trig.write_volatile((1 << 0) | (wordsize(elem_size) << 2) | (1 << 5) | (0x3f << 15));
    while ch0_trans_count.read_volatile() > 0 {}
}

pub(crate) unsafe fn copy_to_spi(src: u32, dst: u32, elem_size: u32, count: u32) {
    let dma_base = 0x50000000 as *mut u32;
    let ch0_read_addr = dma_base.offset(0);
    let ch0_write_addr = dma_base.offset(1);
    let ch0_trans_count = dma_base.offset(2);
    let ch0_ctrl_trig = dma_base.offset(3);
    ch0_read_addr.write_volatile(src);
    ch0_write_addr.write_volatile(dst);
    ch0_trans_count.write_volatile(count);
    ch0_ctrl_trig.write_volatile((1 << 0) | (wordsize(elem_size) << 2) | (1 << 4) | (16 << 15));
    while ch0_trans_count.read_volatile() > 0 {}
}
