use rp2040_pac::dma::ch::ch_ctrl_trig::CH_CTRL_TRIG_SPEC as CtrlReg;
use rp2040_pac::dma::ch::ch_ctrl_trig::W as CtrlWriter;
use rp2040_pac::dma::CH;
use rp2040_pac::generic::W;

pub struct DmaChannel {
    pub channel: usize,
    pub ch: &'static CH,
}

#[allow(clippy::missing_safety_doc)]
impl DmaChannel {
    pub unsafe fn new(channel: usize) -> Self {
        DmaChannel {
            channel,
            ch: &(*rp2040_pac::DMA::PTR).ch[channel],
        }
    }

    pub unsafe fn set_src(&mut self, src: u32) {
        self.ch.ch_read_addr.write(|w| w.bits(src));
    }

    pub unsafe fn set_dst(&mut self, dst: u32) {
        self.ch.ch_write_addr.write(|w| w.bits(dst));
    }

    pub unsafe fn set_count(&mut self, count: u32) {
        self.ch.ch_trans_count.write(|w| w.bits(count));
    }

    pub unsafe fn set_ctrl_and_trigger<F>(&mut self, f: F)
    where
        F: FnOnce(&mut CtrlWriter) -> &mut W<CtrlReg>,
    {
        self.ch.ch_ctrl_trig.write(f);
    }

    pub fn wait(&self) {
        while self.ch.ch_trans_count.read().bits() > 0 {}
    }

    pub fn get_src(&self) -> u32 {
        self.ch.ch_read_addr.read().bits()
    }

    pub fn get_dst(&self) -> u32 {
        self.ch.ch_write_addr.read().bits()
    }

    pub fn get_count(&self) -> u32 {
        self.ch.ch_trans_count.read().bits()
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
    let channel = dma_channel.channel;
    dma_channel.set_src(src);
    dma_channel.set_dst(dst);
    dma_channel.set_count(count);
    dma_channel.set_ctrl_and_trigger(|w| {
        w.treq_sel().permanent();
        w.chain_to().bits(channel as u8);
        w.incr_write().set_bit();
        w.data_size().bits(wordsize(elem_size) as u8);
        w.en().set_bit();
        w
    });
    dma_channel.wait();
}

pub unsafe fn copy_mem(
    dma_channel: &mut DmaChannel,
    src: u32,
    dst: u32,
    elem_size: u32,
    count: u32,
) {
    let channel = dma_channel.channel;
    dma_channel.set_src(src);
    dma_channel.set_dst(dst);
    dma_channel.set_count(count);
    dma_channel.set_ctrl_and_trigger(|w| {
        w.treq_sel().permanent();
        w.chain_to().bits(channel as u8);
        w.incr_write().set_bit();
        w.incr_read().set_bit();
        w.data_size().bits(wordsize(elem_size) as u8);
        w.en().set_bit();
        w
    });
    dma_channel.wait();
}

pub unsafe fn copy_mem_bswap(
    dma_channel: &mut DmaChannel,
    src: u32,
    dst: u32,
    elem_size: u32,
    count: u32,
) {
    let channel = dma_channel.channel;
    dma_channel.set_src(src);
    dma_channel.set_dst(dst);
    dma_channel.set_count(count);
    dma_channel.set_ctrl_and_trigger(|w| {
        w.bswap().set_bit();
        w.treq_sel().permanent();
        w.chain_to().bits(channel as u8);
        w.incr_write().set_bit();
        w.incr_read().set_bit();
        w.data_size().bits(wordsize(elem_size) as u8);
        w.en().set_bit();
        w
    });
    dma_channel.wait();
}

pub unsafe fn copy_flash_to_mem(dma_channel: &mut DmaChannel, src: u32, dst: u32, count: u32) {
    // Flush XIP FIFO.
    let xip_ctrl = &*pico::pac::XIP_CTRL::PTR;
    while xip_ctrl.stat.read().fifo_empty().bit_is_clear() {
        log::info!("XIP FIFO not empty");
        cortex_m::asm::nop();
    }
    xip_ctrl.stream_addr.write(|w| w.bits(src));
    xip_ctrl.stream_ctr.write(|w| w.bits(count));

    let channel = dma_channel.channel;
    dma_channel.set_src(0x50400000); // XIP_AUX_BASE
    dma_channel.set_dst(dst);
    dma_channel.set_count(count);
    dma_channel.set_ctrl_and_trigger(|w| {
        w.treq_sel().bits(37); // DREQ_XIP_STREAM
        w.chain_to().bits(channel as u8);
        w.incr_write().set_bit();
        w.data_size().bits(2); // 4 bytes
        w.en().set_bit();
        w
    });
    dma_channel.wait();

    while xip_ctrl.stat.read().fifo_empty().bit_is_clear() {
        log::info!("XIP FIFO not empty");
        cortex_m::asm::nop();
    }
}

pub(crate) unsafe fn start_copy_to_spi(
    dma_channel: &mut DmaChannel,
    src: u32,
    dst: u32,
    elem_size: u32,
    count: u32,
) {
    let channel = dma_channel.channel;
    dma_channel.set_src(src);
    dma_channel.set_dst(dst);
    dma_channel.set_count(count);
    dma_channel.set_ctrl_and_trigger(|w| {
        w.treq_sel().bits(16); // SPI0 TX
        w.chain_to().bits(channel as u8);
        w.incr_read().set_bit();
        w.data_size().bits(wordsize(elem_size) as u8);
        w.en().set_bit();
        w
    });
}
