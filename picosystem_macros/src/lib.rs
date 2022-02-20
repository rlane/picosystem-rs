mod atlas;
mod map;

use image::io::Reader as ImageReader;
use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Ident, LitInt, LitStr, Token};

struct Sprite {
    function_name: Ident,
    path: LitStr,
    width: LitInt,
}

impl Parse for Sprite {
    fn parse(input: ParseStream) -> Result<Self> {
        let function_name = input.parse()?;
        input.parse::<Token![,]>()?;
        let path = input.parse()?;
        input.parse::<Token![,]>()?;
        let width = input.parse()?;
        Ok(Sprite {
            function_name,
            path,
            width,
        })
    }
}

#[proc_macro]
pub fn sprite(input: TokenStream) -> TokenStream {
    let Sprite {
        function_name,
        path,
        width,
    } = parse_macro_input!(input as Sprite);
    let width = width.base10_parse::<u32>().unwrap();
    let img = ImageReader::open(path.value())
        .expect(&format!("Could not load image {:?}", &path))
        .decode()
        .expect(&format!("Could not decode image {:?}", &path))
        .resize(width, 16384, image::imageops::FilterType::Triangle)
        .into_rgba8();
    let transparent_color = 0;
    let mut found_transparent_color = false;
    let data: Vec<u16> = img
        .pixels()
        .map(|p| {
            let r = p[0] as u16;
            let g = p[1] as u16;
            let b = p[2] as u16;
            let a = p[3] as u16;
            if a != 255 {
                found_transparent_color = true;
                transparent_color
            } else {
                ((r >> 3) << 11) | ((g >> 2) << 5) | ((b >> 3) << 0)
            }
        })
        .collect();

    let mut code = String::new();
    code.push_str(&format!(
        r#"
        pub fn {}() -> &'static picosystem::sprite::Sprite<'static> {{
            #[link_section = ".static_rodata"]
            static DATA: [u16; {}] = {:?};
            #[link_section = ".static_rodata"]
            static SPRITE: picosystem::sprite::Sprite<'static> = picosystem::sprite::Sprite {{
                size: embedded_graphics::geometry::Size::new({}, {}),
                transparent_color: {:?},
                data: &DATA
            }};
            &SPRITE
        }}"#,
        &function_name,
        data.len(),
        &data,
        img.width(),
        img.height(),
        if found_transparent_color {
            Some(transparent_color)
        } else {
            None
        }
    ));
    code.parse().unwrap()
}

#[proc_macro]
pub fn atlas(input: TokenStream) -> TokenStream {
    atlas::atlas(input)
}

#[proc_macro]
pub fn map(input: TokenStream) -> TokenStream {
    map::map(input)
}
