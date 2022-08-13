[![Documentation](https://docs.rs/prost-build/badge.svg)](https://docs.rs/prost-build/)
[![Crate](https://img.shields.io/crates/v/prost-build.svg)](https://crates.io/crates/prost-build)

# `prost-build`

`prost-build` makes it easy to generate Rust code from `.proto` files as part of
a Cargo build. See the crate [documentation](https://docs.rs/prost-build/) for examples
of how to integrate `prost-build` into a Cargo project.

## `protoc`

`prost-build` uses `protoc` to parse the proto files. There are a few ways to make `protoc`
available for `prost-build`. 

The first option is to include `protoc` in your `PATH` this
can be done by following the [`protoc` install instructions]. In addition, its possible to
pass the `PROTOC=<my/path/to/protoc>` environment variable.

[`protoc` install instructions]: https://github.com/protocolbuffers/protobuf#protocol-compiler-installation

The second option is to provide the `vendored` feature flag to `prost-build`. This will
force `prost-build` to compile `protoc` from the bundled source. This will require that
you have the correct dependencies installed include a C++ toolchain, cmake, etc. For
more info on what the required dependencies are check [here].

[here]: https://github.com/protocolbuffers/protobuf/blob/master/src/README.md

If you would like to always ignore vendoring `protoc` you can additionally pass
`PROTOC_NO_VENDOR` and this will always check the `PATH`/`PROTOC` environment
variables and never compile `protoc` from source.

## License

`prost-build` is distributed under the terms of the Apache License (Version 2.0).

See [LICENSE](../LICENSE) for details.

Copyright 2017 Dan Burkert
