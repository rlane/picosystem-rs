use crate::map::NUM_LAYERS;

pub const TILE_SIZE: i32 = 32;

pub struct Tile {
    pub data: &'static [u16],
    pub mask: &'static [u32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileId(u32);

pub fn tile_id(tile: &Tile) -> TileId {
    TileId(tile as *const Tile as u32)
}

pub struct GenMapTile {
    pub layers: heapless::Vec<&'static Tile, NUM_LAYERS>,
}

pub struct LoadedTile {
    pub data: [u16; (TILE_SIZE * TILE_SIZE) as usize],
    pub mask: [u32; TILE_SIZE as usize],
}

impl LoadedTile {
    pub fn new() -> Self {
        LoadedTile {
            data: [0; (TILE_SIZE * TILE_SIZE) as usize],
            mask: [0; TILE_SIZE as usize],
        }
    }
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
mod device {
    use crate::display::{framebuffer, Display, HEIGHT, WIDTH};
    use crate::dma;
    use crate::tile::*;
    use crate::time;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::Rectangle;

    fn load_tile(src: &Tile, dst: &mut LoadedTile, masked: bool) {
        let mut buf = [0u16; (2 * TILE_SIZE * TILE_SIZE + 1) as usize];
        assert_eq!(src.data.len() % 2, 0);
        assert_eq!(src.data.len() < buf.len(), true);
        unsafe {
            let mut dma_channel = dma::DmaChannel::new(1);
            dma::copy_flash_to_mem(
                &mut dma_channel,
                src.data.as_ptr() as u32,
                buf.as_mut_ptr() as u32,
                src.data.len() as u32 / 2,
            );
            decompress_dma(&buf[0..src.data.len()], &mut dst.data);
            if masked {
                dma::copy_flash_to_mem(
                    &mut dma_channel,
                    src.mask.as_ptr() as u32,
                    dst.mask.as_ptr() as u32,
                    TILE_SIZE as u32,
                );
            }
        }
    }

    pub fn decompress_dma(input: &[u16], output: &mut [u16]) {
        unsafe {
            let mut dma_channel0 = dma::DmaChannel::new(1);
            let mut dma_channel1 = dma::DmaChannel::new(2);
            let mut src_ptr: *const u16 = input.as_ptr().add(1);
            let end_ptr = input.as_ptr().add(input.len());
            let mut dst_ptr: *mut u16 = output.as_mut_ptr();

            while src_ptr < end_ptr {
                let ctrl = *src_ptr;
                src_ptr = src_ptr.add(1);
                let data_length = ctrl & 0xff;
                let run_length = ctrl >> 8;

                if data_length == 0 {
                    dst_ptr = dst_ptr.add(run_length as usize);
                    continue;
                }

                dma_channel0.wait();
                dma::start_copy_mem(
                    &mut dma_channel0,
                    src_ptr as u32,
                    dst_ptr as u32,
                    2,
                    data_length as u32,
                );
                src_ptr = src_ptr.add(data_length as usize);
                dst_ptr = dst_ptr.add(data_length as usize);

                if run_length > 0 {
                    dma_channel1.wait();
                    dma::start_set_mem(
                        &mut dma_channel1,
                        src_ptr.offset(-1) as u32,
                        dst_ptr as u32,
                        2,
                        run_length as u32,
                    );
                    dst_ptr = dst_ptr.add(run_length as usize);
                }
            }

            dma_channel0.wait();
            dma_channel1.wait();
        }
    }

    fn draw_opaque_tile(display: &mut Display, tile: &LoadedTile, dst: Point, size: Size) -> bool {
        let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
        let mut dma_channel = unsafe { dma::DmaChannel::new(1) };

        let src = clipped_dst.top_left - dst;
        let dst = clipped_dst.top_left;

        let src_data = &tile.data;
        let dst_data = framebuffer();
        let src_index = src.x + src.y * TILE_SIZE;
        let dst_index = dst.x + dst.y * WIDTH as i32;
        unsafe {
            let mut src_ptr = src_data.as_ptr().add(src_index as usize);
            let mut dst_ptr = dst_data.as_mut_ptr().add(dst_index as usize);
            for _ in 0..clipped_dst.size.height {
                dma_channel.wait();
                dma::start_copy_mem(
                    &mut dma_channel,
                    src_ptr as u32,
                    dst_ptr as u32,
                    4,
                    clipped_dst.size.width / 2,
                );
                src_ptr = src_ptr.add(TILE_SIZE as usize);
                dst_ptr = dst_ptr.add(WIDTH as usize);
            }
        }

        dma_channel.wait();
        clipped_dst.size == size
    }

    fn draw_transparent_tile(
        display: &mut Display,
        tile: &LoadedTile,
        dst: Point,
        size: Size,
    ) -> bool {
        let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
        let src = clipped_dst.top_left - dst;
        let dst = clipped_dst.top_left;

        unsafe {
            let mut dma_channel = dma::DmaChannel::new(1);
            let mut src_ptr: *const u16 = tile.data.as_ptr();
            let mut dst_ptr: *mut u16 = framebuffer().as_mut_ptr();
            let mut mask_ptr: *const u32 = tile.mask.as_ptr().add(src.y as usize);
            src_ptr = src_ptr.add((src.x + src.y * TILE_SIZE) as usize);
            dst_ptr = dst_ptr.add((dst.x + dst.y * WIDTH as i32) as usize);
            for _ in 0..clipped_dst.size.height {
                let w = clipped_dst.size.width;
                let mut mask = *mask_ptr;
                mask >>= src.x;
                if w < 32 {
                    mask &= (1 << w) - 1;
                }
                let mut x = 0;
                while mask != 0 {
                    const LOOKAHEAD: u32 = 0x7;
                    let n = if mask & LOOKAHEAD == LOOKAHEAD {
                        let n = mask.trailing_ones();
                        dma_channel.wait();
                        dma::start_copy_mem(&mut dma_channel, src_ptr as u32, dst_ptr as u32, 2, n);
                        n
                    } else if mask & LOOKAHEAD == 0x0 {
                        mask.trailing_zeros()
                    } else {
                        let color = *src_ptr;
                        if mask & 1 != 0 {
                            *dst_ptr = color;
                        }
                        1
                    };
                    src_ptr = src_ptr.add(n as usize);
                    dst_ptr = dst_ptr.add(n as usize);
                    if n == 32 {
                        mask = 0;
                    } else {
                        mask >>= n;
                    }
                    x += n;
                }
                src_ptr = src_ptr.add(TILE_SIZE as usize - x as usize);
                dst_ptr = dst_ptr.add(WIDTH as usize - x as usize);
                mask_ptr = mask_ptr.add(1);
            }
            dma_channel.wait();
        }

        clipped_dst.size == size
    }

    fn copy_tile(display: &mut Display, src: Point, dst: Point, size: Size) {
        let clipped_dst = Rectangle::new(dst, size).intersection(&display.bounding_box());
        let mut dma_channel = unsafe { dma::DmaChannel::new(2) };
        let fb_data = framebuffer();

        let src = src + clipped_dst.top_left - dst;
        let dst = clipped_dst.top_left;
        let size = clipped_dst.size;

        let mut src_index = src.x + src.y * WIDTH as i32;
        let mut dst_index = dst.x + dst.y * WIDTH as i32;
        for _ in 0..size.height {
            unsafe {
                let src_addr = fb_data.as_ptr().add(src_index as usize) as u32;
                let dst_addr = fb_data.as_mut_ptr().add(dst_index as usize) as u32;
                dma_channel.wait();
                dma::start_copy_mem(&mut dma_channel, src_addr, dst_addr, 2, size.width);
            }
            src_index += WIDTH as i32;
            dst_index += WIDTH as i32;
        }
        dma_channel.wait();
    }

    pub fn draw<F>(display: &mut Display, position: Point, map_generator: &F, verbose: bool)
    where
        F: Fn(Point) -> GenMapTile,
    {
        let subtile_mask = 32 - 1;
        let enable_tile_cache = true;

        let mut drawn_y: i32 = 0;
        let mut world_y = position.y;
        let subtile_y = position.y & subtile_mask;

        let mut tile_cache = heapless::LinearMap::<TileId, Point, 64>::new();
        let mut base_tile_cache_misses = 0;
        let mut base_tile_cache_lookups = 0;
        let mut base_tile_cache_insert_failures = 0;

        let mut overlay_tile_cache = heapless::LinearMap::<TileId, LoadedTile, 4>::new();
        let mut overlay_tile_cache_misses = 0;
        let mut overlay_tile_cache_lookups = 0;
        let mut overlay_tile_cache_insert_failures = 0;

        let mut missing_transparent_tiles = heapless::Vec::<(Point, GenMapTile), 64>::new();

        let mut slow_draw = false;
        let mut draw_time = 0;
        let mut load_time = 0;
        loop {
            let progress = display.flush_progress();
            let safe_y = (progress as i32 - WIDTH as i32 + 1) / WIDTH as i32;
            if safe_y - drawn_y < 32 && progress < (WIDTH * HEIGHT) as usize {
                continue;
            } else if safe_y - drawn_y > 64 {
                slow_draw = true;
            }
            let draw_start_time = time::time_us();

            let screen_y = drawn_y - subtile_y;

            let subtile_x = position.x & subtile_mask;

            for screen_x in (-subtile_x..(WIDTH as i32)).step_by(32) {
                let world_x = position.x + screen_x;
                let map_coord = Point::new(world_x & !subtile_mask, world_y & !subtile_mask);
                let screen_coord = Point::new(screen_x, screen_y);
                let map_tile = map_generator(map_coord);
                let base_tile = map_tile.layers[0];
                base_tile_cache_lookups += 1;
                if let Some(cached_src) = tile_cache.get(&tile_id(base_tile)) {
                    copy_tile(display, *cached_src, screen_coord, Size::new(32, 32));
                    for overlay_tile in map_tile.layers[1..].iter() {
                        overlay_tile_cache_lookups += 1;
                        if let Some(cached_overlay_tile) =
                            overlay_tile_cache.get(&tile_id(overlay_tile))
                        {
                            draw_transparent_tile(
                                display,
                                cached_overlay_tile,
                                screen_coord,
                                Size::new(32, 32),
                            );
                        } else {
                            overlay_tile_cache_misses += 1;
                            let mut loaded_tile = LoadedTile::new();
                            let start_time = time::time_us();
                            load_tile(overlay_tile, &mut loaded_tile, true);
                            load_time += time::time_us() - start_time;
                            draw_transparent_tile(
                                display,
                                &loaded_tile,
                                screen_coord,
                                Size::new(32, 32),
                            );
                            if let Err(_) =
                                overlay_tile_cache.insert(tile_id(overlay_tile), loaded_tile)
                            {
                                overlay_tile_cache_insert_failures += 1;
                            }
                        }
                    }
                } else {
                    base_tile_cache_misses += 1;
                    let mut loaded_tile = LoadedTile::new();
                    let start_time = time::time_us();
                    load_tile(base_tile, &mut loaded_tile, false);
                    load_time += time::time_us() - start_time;
                    if (draw_opaque_tile(display, &loaded_tile, screen_coord, Size::new(32, 32))
                        || (screen_x >= 0 && screen_y < 0))
                        && enable_tile_cache
                    {
                        if let Err(_) = tile_cache.insert(tile_id(base_tile), screen_coord) {
                            base_tile_cache_insert_failures += 1;
                        }
                    }
                    if map_tile.layers.len() > 1 {
                        let _ = missing_transparent_tiles.push((screen_coord, map_tile));
                    }
                }
            }

            draw_time += time::time_us() - draw_start_time;

            drawn_y += 32;
            world_y += 32;
            if screen_y < 0 {
                tile_cache.clear();
            } else if screen_y + 32 >= HEIGHT as i32 {
                break;
            }
        }

        let draw_start_time = time::time_us();
        for (screen_coord, map_tile) in missing_transparent_tiles {
            for overlay_tile in map_tile.layers[1..].iter() {
                overlay_tile_cache_lookups += 1;
                if let Some(cached_overlay_tile) = overlay_tile_cache.get(&tile_id(overlay_tile)) {
                    draw_transparent_tile(
                        display,
                        cached_overlay_tile,
                        screen_coord,
                        Size::new(32, 32),
                    );
                } else {
                    overlay_tile_cache_misses += 1;
                    let mut loaded_tile = LoadedTile::new();
                    let start_time = time::time_us();
                    load_tile(overlay_tile, &mut loaded_tile, true);
                    load_time += time::time_us() - start_time;
                    draw_transparent_tile(display, &loaded_tile, screen_coord, Size::new(32, 32));
                    if let Err(_) = overlay_tile_cache.insert(tile_id(overlay_tile), loaded_tile) {
                        overlay_tile_cache_insert_failures += 1;
                    }
                }
            }
        }
        draw_time += time::time_us() - draw_start_time;

        if verbose {
            log::info!("draw_time={}us load_time={}us", draw_time, load_time);
            log::info!("position: {:?}", position);
            log::info!(
                "Base tile cache: misses={} lookups={} insert_failures={} miss_rate={:.2}%",
                base_tile_cache_misses,
                base_tile_cache_lookups,
                base_tile_cache_insert_failures,
                base_tile_cache_misses as f32 / base_tile_cache_lookups as f32 * 100.0
            );
            log::info!(
                "Overlay tile cache: misses={} lookups={} insert_failures={} miss_rate={:.2}%",
                overlay_tile_cache_misses,
                overlay_tile_cache_lookups,
                overlay_tile_cache_insert_failures,
                overlay_tile_cache_misses as f32 / overlay_tile_cache_lookups as f32 * 100.0
            );
            if slow_draw {
                log::info!("Slow draw detected");
            }
        }
    }
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub use device::draw;
