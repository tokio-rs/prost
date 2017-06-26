[![Documentation](https://docs.rs/prost-build/badge.svg)](https://docs.rs/prost-codegen/)
[![Crate](https://img.shields.io/crates/v/prost-build.svg)](https://crates.io/crates/prost-codegen)

# `prost-build`

`prost-build` is a small library meant to be used from Cargo `build.rs` scripts
in order to make build-time code generation of `.proto` files easy.

`prost-build` automatically downloads a pre-compiled `protoc` at build time on
Linux, macOS, and Windows in order to generate code.

## Example Project

Let's create a small crate, `snazzy`, that defines a collection of
snazzy new items in a protobuf file.

```bash
cargo new snazzy
```

First, add `prost`, `prost-build`, and `prost`'s public dependencies to the
`Cargo.toml`:

```toml
[dependencies]
bytes = <bytes-version>
prost = <prost-version>
prost-derive = <prost-version>

[build-dependencies]
prost-build = <prost-version>
```

Next, add `src/items.proto` to the project:

```proto
syntax = "proto3";

package snazzy.items;

// A snazzy new shirt!
message Shirt {
  enum Size {
    SMALL = 0;
    MEDIUM = 1;
    LARGE = 2;
  }

  string color = 1;
  Size size = 2;
}
```

To generate Rust code from `items.proto`, we use `prost-build` in the crate's
`build.rs` build-script:

```rust
extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/items.proto"],
                                &["src/"],
                                None).unwrap();
}
```

And finally, in `lib.rs`, include the generated code:

```rust
extern crate prost;
#[macro_use]
extern crate prost_derive;

// Include the `items` module, which is generated from items.proto.
pub mod items {
    include!(concat!(env!("OUT_DIR"), "/items.rs"));
}

pub fn create_large_shirt(color: String) -> items::Shirt {
    let mut shirt = items::Shirt::default();
    shirt.color = color;
    shirt.set_size(items::shirt::Size::Large);
    shirt
}
```

That's it! Run `cargo doc` to see documentation for the generated code. The full
example project can be found on [GitHub](https://github.com/danburkert/snazzy).
