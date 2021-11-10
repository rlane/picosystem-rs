# picosystem-rs

This repository contains a Rust SDK for the [PicoSystem][1] handheld game console
and a several games built with it.

All the included games are compiled into a single UF2 binary that boots into a
menu screen. You can also use this SDK to create standalone games.

[1]: https://shop.pimoroni.com/products/picosystem

## Compiling and Running

Install the required toolchain:

```
rustup target add thumbv6m-none-eabi
cargo install elf2uf2-rs flip-link
```

Put the PicoSystem into USB boot mode, mount the drive, then:

```
cargo run --release
```

## Demo Games

 * Maze
 * Draw
 * Life
 * Music
 * Tanks
 * Hangman
 * Invaders
 * Blob

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Acknowledgements

- [rp-rs/rp-hal](https://github.com/rp-rs/rp-hal): Rust Pico HAL
- [pimoroni/picosystem](https://github.com/pimoroni/picosystem): Official C++ SDK
