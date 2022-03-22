use picosystem::map::{MapTile, INVALID_TILE, NUM_LAYERS};
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

    let map = tiled::Map::parse_file(
        &Path::new(&path.value()),
        &mut tiled::FilesystemResourceCache::new(),
    )
    .expect("Failed to parse map");

    assert_eq!(map.tile_width, TILE_SIZE as u32);
    assert_eq!(map.tile_height, TILE_SIZE as u32);
    assert_eq!(map.tilesets().len(), 1);
    assert_eq!(map.layers().len() <= NUM_LAYERS, true);
    assert_eq!(map.infinite(), false);

    let mut tile_index_layers = Vec::<Vec<u16>>::new();
    let mut used_tile_functions: HashSet<u16> = HashSet::new();
    for layer in map.layers() {
        let mut tile_index_layer = Vec::<u16>::new();
        if let tiled::LayerType::TileLayer(tiled::TileLayer::Finite(tile_layer)) =
            &layer.layer_type()
        {
            for y in 0..tile_layer.height() {
                for x in 0..tile_layer.width() {
                    let tile_index = match tile_layer.get_tile(x as i32, y as i32) {
                        Some(tile) => tile.id() as u16,
                        None => INVALID_TILE,
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
                width: {},
                height: {},
                tiles: &{:?},
                tile_functions: [{}],
            }};
            &MAP
        }}",
        &function_name, map.width, map.height, &tiles, &tile_functions_code
    ));
    code.parse().expect("Failed to parse code")
}
