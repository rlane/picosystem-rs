use picosystem::map::{MapTile, INVALID_TILE, MAP_SIZE, NUM_LAYERS};
use picosystem::tile::TILE_SIZE;
use proc_macro::TokenStream;
use std::collections::HashSet;
use std::path::Path;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Ident, LitStr, Token};

struct MapArgs {
    function_name: Ident,
    path: LitStr,
}

impl Parse for MapArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let function_name = input.parse()?;
        input.parse::<Token![,]>()?;
        let path = input.parse()?;
        Ok(MapArgs {
            function_name,
            path,
        })
    }
}

pub fn map(input: TokenStream) -> TokenStream {
    let MapArgs {
        function_name,
        path,
    } = parse_macro_input!(input as MapArgs);

    let map = tiled::parse_file(&Path::new(&path.value())).unwrap();

    assert_eq!(map.width, MAP_SIZE as u32);
    assert_eq!(map.height, MAP_SIZE as u32);
    assert_eq!(map.tile_width, TILE_SIZE as u32);
    assert_eq!(map.tile_height, TILE_SIZE as u32);
    assert_eq!(map.tilesets.len(), 1);
    assert_eq!(map.layers.len() <= NUM_LAYERS, true);
    assert_eq!(map.infinite, false);

    let mut tile_index_layers = Vec::<Vec<u16>>::new();
    let mut used_tile_functions: HashSet<u16> = HashSet::new();
    for layer in map.layers.iter() {
        let mut tile_index_layer = Vec::<u16>::new();
        if let tiled::LayerData::Finite(rows) = &layer.tiles {
            for row in rows {
                for tile in row {
                    let tile_index = if tile.gid == 0 {
                        INVALID_TILE
                    } else {
                        (tile.gid - 1) as u16
                    };
                    tile_index_layer.push(tile_index);
                    used_tile_functions.insert(tile_index);
                }
            }
            tile_index_layers.push(tile_index_layer);
        }
    }

    let mut tiles = Vec::<MapTile>::new();
    for i in 0..(tile_index_layers[0].len()) {
        let mut tile = MapTile {
            layers: [INVALID_TILE; NUM_LAYERS],
        };
        for j in 0..(tile_index_layers.len()) {
            tile.layers[j] = tile_index_layers[j][i];
        }
        tiles.push(tile);
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
                tiles: {:?},
                tile_functions: [{}],
            }};
            &MAP
        }}",
        &function_name, &tiles, &tile_functions_code
    ));
    code.parse().unwrap()
}
