# maia-httpd

[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/maia-httpd.svg
[crates-url]: https://crates.io/crates/maia-httpd

maia-httpd is a Rust crate that implements the HTTP server used in [Maia
SDR](https://maia-sdr.org/). This web server runs on the Zynq ARM CPU and
streams data to web browsers running on a client device.

## Building

In order to simplify building maia-httpd for the [Pluto SDR
firmware](https://github.com/maia-sdr/plutosdr-fw), which uses a buildroot
uclibc toolchain, a custom Docker image can be used to build against this
toolchain with [cross](https://github.com/cross-rs/cross). This image is already
configured in `Cross.toml`.

The crate can be built as
```
cross build --release --target armv7-unknown-linux-gnueabihf
```

## API documentation

The API documentation is hosted in [docs.rs](https://docs.rs/maia-httpd/).

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
