[![Documentation](https://docs.rs/prost-codegen/badge.svg)](https://docs.rs/prost-codegen/)
[![Crate](https://img.shields.io/crates/v/prost-codegen.svg)](https://crates.io/crates/prost-codegen)

# `prost-codegen`

`prost-codegen` is a library which generates Rust source code from Protobuf
`FileDescriptorProto` messages, as well as a binary plugin, `protoc-gen-rust`,
for the Protubuf compiler (`protoc`). For the most part, users of `prost` will
not need to interact with `proto-codegen` as a library.

## Using `protoc-gen-prost`

`protoc-gen-prost` is used as a plugin for `protoc` in order to perform
ahead-of-time compilation of `.proto` files into Rust source files. In fact,
`prost-codegen` includes [pre-generated sources](src/google/protobuf/mod.rs)
from `.proto` files defined in the Protobuf project. These files can be
re-generated using `protoc-gen-prost` and `protoc`.

```bash
PROTOBUF_HOME=<path-to-protobuf-repo>
cargo build --release -p prost-codegen
protoc --prost_out=prost-codegen/src/ \
       --plugin=target/release/protoc-gen-prost \
       -I$PROTOBUF_HOME/src/cpp/protobuf/src/ \
       $PROTOBUF_HOME/src/cpp/protobuf/src/google/protobuf/compiler/plugin.proto
```

For more information about Protobuf plugins, see the compiler help
(`protoc --help`) and the
[Protocol Buffers Reference](https://developers.google.com/protocol-buffers/docs/reference/other).
