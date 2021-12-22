use proc_macro::TokenStream;
use std::collections::HashSet;
use std::path::Path;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Ident, LitStr, Token};

struct Map {
    function_name: Ident,
    path: LitStr,
}

impl Parse for Map {
    fn parse(input: ParseStream) -> Result<Self> {
        let function_name = input.parse()?;
        input.parse::<Token![,]>()?;
        let path = input.parse()?;
        Ok(Map {
            function_name,
            path,
        })
    }
}

pub fn map(input: TokenStream) -> TokenStream {
    let Map {
        function_name,
        path,
    } = parse_macro_input!(input as Map);

    let map = tiled::parse_file(&Path::new(&path.value())).unwrap();

    assert_eq!(map.width, 100);
    assert_eq!(map.height, 100);
    assert_eq!(map.tile_width, 32);
    assert_eq!(map.tile_height, 32);
    assert_eq!(map.tilesets.len(), 1);
    assert_eq!(map.layers.len(), 2);
    assert_eq!(map.infinite, false);

    let mut base_tile_indices: Vec<u16> = Vec::new();
    let mut overlay_tile_indices: Vec<u16> = Vec::new();
    let mut used_tile_functions: HashSet<u16> = HashSet::new();
    for layer in &map.layers {
        let tile_indices = match layer.name.as_str() {
            "Base" => &mut base_tile_indices,
            "Overlay" => &mut overlay_tile_indices,
            _ => panic!("unexpected layer name"),
        };
        if let tiled::LayerData::Finite(rows) = &layer.tiles {
            for row in rows {
                for tile in row {
                    let tile_index = if tile.gid == 0 {
                        !0
                    } else {
                        (tile.gid - 1) as u16
                    };
                    tile_indices.push(tile_index);
                    used_tile_functions.insert(tile_index);
                }
            }
        }
    }

    let mut tile_functions_code = String::new();
    for i in 0..2048 {
        if used_tile_functions.contains(&i) {
            tile_functions_code.push_str(&format!("atlas{},\n", i));
        } else {
            tile_functions_code.push_str(&format!("atlas{},\n", 0));
        }
    }

    let mut code = String::new();
    code.push_str(&format!(
        r"
        pub fn {}() -> &'static Map {{
            static MAP: Map = Map {{
                base_tile_indices: {:?},
                overlay_tile_indices: {:?},
                tile_functions: [{}],
            }};
            &MAP
        }}",
        &function_name, &base_tile_indices, &overlay_tile_indices, &tile_functions_code
    ));
    code.parse().unwrap()
}
