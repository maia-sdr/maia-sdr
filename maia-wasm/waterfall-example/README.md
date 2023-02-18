# maia-wasm waterfall example

This crate contains an example of how to use the maia-wasm WebGL2 waterfall in a
standalone way. The spectrum data used in this example is stored in a JPEG file,
which is embedded into the wasm file. The JPEG is decoded when the waterfall is
created, and a closure called using `setInterval()` puts the spectrum data in the
waterfall one line at a time.

Note that since the spectrum data is stored in JPEG format, there are visible
JPEG artifacts, specially in the noise floor and around narrowband
signals. These artifacts are only present in this example, and not in the normal
usage of maia-wasm.

## Building

The crate can be built with [wasm-pack](https://rustwasm.github.io/wasm-pack/)
by doing
```
wasm-pack build -t web
```

## Running

A HTTP server needs to be spawned in the crate root directory and the
`index.html` file must be loaded in a web browser. To run an HTTP server, it is possible to use
```
python3 -m http.server
```

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
