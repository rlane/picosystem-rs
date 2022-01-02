use image::io::Reader as ImageReader;
use image::GenericImageView;
use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Ident, LitInt, LitStr, Token};

const TILE_SIZE: usize = 32;

struct Atlas {
    function_name: Ident,
    path: LitStr,
    tile_size: LitInt,
}

impl Parse for Atlas {
    fn parse(input: ParseStream) -> Result<Self> {
        let function_name = input.parse()?;
        input.parse::<Token![,]>()?;
        let path = input.parse()?;
        input.parse::<Token![,]>()?;
        let tile_size = input.parse()?;
        Ok(Atlas {
            function_name,
            path,
            tile_size,
        })
    }
}

pub fn atlas(input: TokenStream) -> TokenStream {
    let Atlas {
        function_name,
        path,
        tile_size,
    } = parse_macro_input!(input as Atlas);
    let tile_size = tile_size.base10_parse::<u32>().unwrap();
    assert_eq!(tile_size as usize, TILE_SIZE);
    let img = ImageReader::open(path.value())
        .expect(&format!("Could not load image {:?}", &path))
        .decode()
        .expect(&format!("Could not decode image {:?}", &path))
        .into_rgba8();

    let mut tile_index = 0;
    let mut code = String::new();
    for y in 0..img.height() / tile_size {
        for x in 0..img.width() / tile_size {
            let tile = img.view(x * tile_size, y * tile_size, tile_size, tile_size);

            let transparent_color = 0;
            let mut found_transparent_color = false;
            let data: Vec<u16> = tile
                .pixels()
                .map(|(_, _, p)| {
                    let r = p[0] as u16;
                    let g = p[1] as u16;
                    let b = p[2] as u16;
                    let a = p[3] as u16;
                    if a != 255 {
                        found_transparent_color = true;
                        transparent_color
                    } else {
                        (((r >> 3) << 11) | ((g >> 2) << 5) | ((b >> 3) << 0)).to_be()
                    }
                })
                .collect();

            let mut mask = [0u32; TILE_SIZE];
            for y in 0..TILE_SIZE {
                let mut m: u32 = 0;
                for x in 0..TILE_SIZE {
                    let color = data[(y * TILE_SIZE + x) as usize];
                    if color != 0 {
                        m |= 1 << x;
                    }
                }
                mask[y as usize] = m;
            }

            let mut compressed_data = [0u16; 2 * TILE_SIZE * TILE_SIZE + 1];
            let mut compressed_length =
                picosystem_compressor::compress(&data, &mut compressed_data);
            if compressed_length % 2 != 0 {
                compressed_length += 1;
            }

            code.push_str(&format!(
                r"
        pub fn {}{}() -> &'static picosystem::tile::Tile {{
            static COMPRESSION_RATIO: u32 = {};
            static DATA: [u16; {}] = {:?};
            static MASK: [u32; {}] = {:?};
            static TILE: picosystem::tile::Tile = picosystem::tile::Tile {{
                data: &DATA,
                mask: &MASK,
            }};
            &TILE
        }}",
                &function_name,
                tile_index,
                (100.0 * compressed_length as f64 / data.len() as f64) as u32,
                compressed_length,
                &compressed_data[0..compressed_length],
                mask.len(),
                &mask
            ));

            tile_index += 1;
        }
    }

    code.parse().unwrap()
}
