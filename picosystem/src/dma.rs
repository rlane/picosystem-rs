use rp2040_pac::dma::ch::ch_ctrl_trig::CH_CTRL_TRIG_SPEC as CtrlReg;
use rp2040_pac::dma::ch::ch_ctrl_trig::W as CtrlWriter;
use rp2040_pac::dma::CH;
use rp2040_pac::generic::W;

pub struct DmaChannel {
    pub ch: *mut CH,
}

#[allow(clippy::missing_safety_doc)]
impl DmaChannel {
    pub unsafe fn new(channel: isize) -> Self {
        let ch0 = 0x50000000 as *mut CH;
        DmaChannel {
            ch: ch0.offset(channel),
        }
    }

    pub unsafe fn set_src(&mut self, src: u32) {
        (*self.ch).ch_read_addr.write(|w| w.bits(src));
    }

    pub unsafe fn set_dst(&mut self, dst: u32) {
        (*self.ch).ch_write_addr.write(|w| w.bits(dst));
    }

    pub unsafe fn set_count(&mut self, count: u32) {
        (*self.ch).ch_trans_count.write(|w| w.bits(count));
    }

    pub unsafe fn set_ctrl_and_trigger<F>(&mut self, f: F)
    where
        F: FnOnce(&mut CtrlWriter) -> &mut W<CtrlReg>,
    {
        (*self.ch).ch_ctrl_trig.write(f);
    }

    pub fn wait(&self) {
        unsafe { while (*self.ch).ch_trans_count.read().bits() > 0 {} }
    }
}

fn wordsize(elem_size: u32) -> u32 {
    match elem_size {
        1 => 0,
        2 => 1,
        4 => 2,
        _ => panic!("invalid DMA element size"),
    }
}

pub(crate) unsafe fn set_mem(
    dma_channel: &mut DmaChannel,
    src: u32,
    dst: u32,
    elem_size: u32,
    count: u32,
) {
    dma_channel.set_src(src);
    dma_channel.set_dst(dst);
    dma_channel.set_count(count);
    dma_channel.set_ctrl_and_trigger(|w| {
        w.treq_sel().permanent();
        w.incr_write().set_bit();
        w.data_size().bits(wordsize(elem_size) as u8);
        w.en().set_bit();
        w
    });
    dma_channel.wait();
}

pub(crate) unsafe fn copy_to_spi(
    dma_channel: &mut DmaChannel,
    src: u32,
    dst: u32,
    elem_size: u32,
    count: u32,
) {
    dma_channel.set_src(src);
    dma_channel.set_dst(dst);
    dma_channel.set_count(count);
    dma_channel.set_ctrl_and_trigger(|w| {
        w.treq_sel().bits(16); // SPI0 TX
        w.incr_read().set_bit();
        w.data_size().bits(wordsize(elem_size) as u8);
        w.en().set_bit();
        w
    });
    dma_channel.wait();
}
