# Prost version 0.14.1

_PROST!_ is a [Protocol Buffers](https://protobuf.dev/) implementation for the [Rust Language](https://www.rust-lang.org/). `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.

## ⚠️ Revert emission of `rerun` commands

Version 0.14.1 reverts the emission of `rerun` commands. Other than this change, it is identical to 0.14.0.

In version 0.14.0, `prost-build` began emitting `rerun` commands. While intended to improve build correctness, this change caused regressions for some users—for example, those generating `protos` from an `includes` directory. These edge cases are difficult to address reliably, so the change has been rolled back in 0.14.1.

For more details, see [issue #1296](https://github.com/tokio-rs/prost/issues/1296).

## Breaking changes
- prost: Relax Message Debug trait bound (#1147)

  BREAKING CHANGE: `trait Debug` was a supertrait of `trait Message`. This is no longer required by `prost`. If your code relies on `trait Debug` being implemented for every `impl Message`, you must now explicitly state that you require both Debug and Message. For example: `where M: Debug + Message`

- prost: Remove prost-derive feature (#1247)

  BREAKING CHANGE: Feature flag `prost-derive` is renamed to `derive`. Please rename any usage of `prost-derive` feature in your `Cargo.toml`.

- prost-build: Prevent repeated fields to be boxed (#1237)

  BREAKING CHANGE: A repeated field that is manually marked as boxed was typed as `Vec<Box<T>>`. Those fields are now simply typed as `Vec<T>` to prevent double indirection. The `boxed` configuration is effectively ignored for repeated fields.

- prost-build: Make `type_name_domain` cumulative (#1228)

  BREAKING CHANGE: The configuration for domain names of messages is now cumulative. All calls to `prost_build::Config::type_name_domain` are now concatenated. The previous behavior was that only the arguments of the last call were used. If you do multiple calls to type_name_domain, you need to remove all but the last call to maintain the same behavior.

- prost-build: Derive Eq and Hash trait for messages where possible (#1175)

  BREAKING CHANGE: `prost-build` will automatically derive `trait Eq` and `trait Hash` for types where all field support those as well. If you manually `impl Eq` and/or `impl Hash` for generated types, then you need to remove the manual implementation. If you use `type_attribute` to `derive(Eq)` and/or `derive(Hash)`, then you need to remove those.

## Features

- prost-types: Implement conversion `Duration` to/from `chrono::TimeDelta` (#1236)
- prost-build: Prepare for 2024 keyword `gen` (#1257)

## Dependencies

- *(deps)* Update pulldown-cmark to 0.13 (#1259)
- *(deps)* update criterion requirement from 0.5 to 0.6 (#1280)

## Documentation

- Update dead link LICENSE in `prost-types/README.md` (#1262)

## Styling
- Use DoubleEndedIterator::next_back (#1255)
- Fix typo (#1260)

## Testing

- Run tests using edition 2024 (#1254)
- Run clippy with edition 2024 enabled (#1256)

# Prost version 0.14.0

_PROST!_ is a [Protocol Buffers](https://protobuf.dev/) implementation for the [Rust Language](https://www.rust-lang.org/). `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.

## Breaking changes
- prost: Relax Message Debug trait bound (#1147)

  BREAKING CHANGE: `trait Debug` was a supertrait of `trait Message`. This is no longer required by `prost`. If your code relies on `trait Debug` being implemented for every `impl Message`, you must now explicitly state that you require both Debug and Message. For example: `where M: Debug + Message`

- prost: Remove prost-derive feature (#1247)

  BREAKING CHANGE: Feature flag `prost-derive` is renamed to `derive`. Please rename any usage of `prost-derive` feature in your `Cargo.toml`.

- prost-build: Emit `rerun` commands (#1140)

  BREAKING CHANGE: Previously `cargo` assumed it had to rerun `build.rs` if any files in the project changed. `prost-build` will now emit `rerun` commands, which means only the explicitly marked files cause a rerun. If your `build.rs` is dependent on any other file paths than those given to `prost-build`, then your `build.rs` needs to emit `rerun` commands as well.

- prost-build: Prevent repeated fields to be boxed (#1237)

  BREAKING CHANGE: A repeated field that is manually marked as boxed was typed as `Vec<Box<T>>`. Those fields are now simply typed as `Vec<T>` to prevent double indirection. The `boxed` configuration is effectively ignored for repeated fields.

- prost-build: Make `type_name_domain` cumulative (#1228)

  BREAKING CHANGE: The configuration for domain names of messages is now cumulative. All calls to `prost_build::Config::type_name_domain` are now concatenated. The previous behavior was that only the arguments of the last call were used. If you do multiple calls to type_name_domain, you need to remove all but the last call to maintain the same behavior.

- prost-build: Derive Eq and Hash trait for messages where possible (#1175)

  BREAKING CHANGE: `prost-build` will automatically derive `trait Eq` and `trait Hash` for types where all field support those as well. If you manually `impl Eq` and/or `impl Hash` for generated types, then you need to remove the manual implementation. If you use `type_attribute` to `derive(Eq)` and/or `derive(Hash)`, then you need to remove those.

## Features

- prost-types: Implement conversion `Duration` to/from `chrono::TimeDelta` (#1236)
- prost-build: Prepare for 2024 keyword `gen` (#1257)

## Dependencies

- *(deps)* Update pulldown-cmark to 0.13 (#1259)
- *(deps)* update criterion requirement from 0.5 to 0.6 (#1280)

## Documentation

- Update dead link LICENSE in `prost-types/README.md` (#1262)

## Styling
- Use DoubleEndedIterator::next_back (#1255)
- Fix typo (#1260)

## Testing

- Run tests using edition 2024 (#1254)
- Run clippy with edition 2024 enabled (#1256)

# Prost version 0.13.5

_PROST!_ is a [Protocol Buffers](https://protobuf.dev/) implementation for the [Rust Language](https://www.rust-lang.org/). `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.

## Features

- prost-types: Derive Arbitrary (#1188)

## Documentation

- Use intra doc links instead of HTML tags (#1219)

## Dependencies

- Update pulldown-cmark-to-cmark requirement from >=16, <=19 to >=16, <=20 (#1206)
- Update itertools requirement from >=0.10, <=0.13 to >=0.10, <=0.14 (#1222)
- Update petgraph requirement to include 0.7 (#1226)
- Update rand requirement from 0.8 to 0.9 (#1233)
- Bump clippy to 1.83 (#1220)
- Update flake.lock (#1216)

## Styling

- Replace unnecessary `map_or` (#1221)
- prost-build: Use enum getter (#1238)

## Testing

- default_enum_value: Move tests to separate module (#1198)
- nesting: Move tests to separate module (#1218)
- recursive_oneof: Move tests to separate module (#1225)
- boxed_field: Box an oneof field (#1235)
- groups: Move tests to separate module (#1234)
- default_string_escape: Move tests to separate module (#1239)
- Move DecodeError tests closer to the implementation (#1227)

# Prost version 0.13.4

_PROST!_ is a [Protocol Buffers](https://protobuf.dev/) implementation for the [Rust Language](https://www.rust-lang.org/). `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.

## Features

- Impl Name for Protobuf well-known wrapper types (#1174)

## Performance

- NonZeroU64 to optimize encoded_len_varint (#1192)

## Dependencies

- Remove unused `bytes` dependency from `prost-build` (#1169)
- Update pulldown-cmark-to-cmark requirement from >=16, <=17 to >=16, <=18 (#1173)
- Update pulldown-cmark-to-cmark requirement from >=16, <=18 to >=16, <=19 (#1195)
- Update protobuf to v25.3 (#1165)
- Update protobuf to v25.4 (#1176)

## Styling

- Remove explicit lifetimes (#1180)
- Remove unnecessary empty line after document (#1181)

## Testing

- *(boxed_field)* Confirm `Foo::bar` is boxed (#1168)
- Move build.rs to standard location (#1167)
- *(custom_debug)* Merge `skip_debug` into `custom_debug` (#1178)
- Rename `invalid_doctest` to `disable_comments` (#1183)
- *(custom_attributes)* Move module to separate file (#1187)

## Build

- Bump clippy version to 1.82 (#1182)
- Restrict permissions of `GITHUB_TOKEN` (#1189)

# Prost version 0.13.3

_PROST!_ is a [Protocol Buffers](https://protobuf.dev/) implementation for the [Rust Language](https://www.rust-lang.org/). `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.


### Features

- *(prost-types)* Add `normalized` functions (#1158)

### Bug Fixes

- *(prost-build)* Remove `derived(Copy)` on boxed fields (#1157)

### Documentation

- *(prost-types)* Add description of using Any (#1141)
- *(prost-build)* Use `cargo add` in example (#1149)

### Styling

- Use `Path::display()` when printing a path (#1150)
- `MessageGraph::new()` can't actually fail (#1151)
- *(generated-code)* Use `Self` in `as_str_name` (#1154)

### Testing

- Actually test `skip_debug` for `prost::Oneof` (#1148)
- *(prost-build)* Validate error texts (#1152)
- *(prost-build)* Fix error texts (#1156)

### Build

- Increase MSRV to 1.71.1 (#1135)
- *(deps)* Update pulldown-cmark to 0.12 and pulldown-cmark-to-cmark to 16 (#1144)
- *(protobuf)* Compile and install protoc on Windows (#1145)
- *(protobuf)* Use same `protoc` from same repo as .proto-files (#1136)
- *(deps)* Update pulldown-cmark-to-cmark from 16 to 17 (#1155)
- Unify assert on fixtures (#1142)

# Prost version 0.13.2

_PROST!_ is a [Protocol Buffers](https://protobuf.dev/) implementation for the [Rust Language](https://www.rust-lang.org/). `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.

## Features

- prost-build: Add protoc executable path to Config (#1126)
- prost-build: Extract file descriptor loading from compile_protos() (#1067)

## Bug Fixes

- prost-types: Fix date-time parsing (#1096)
- prost-types: '+' is not a numeric digit (#1104)
- prost-types: Converting DateTime to Timestamp is fallible (#1095)
- prost-types: Parse timestamp with long second fraction (#1106)
- prost-types: Format negative fractional duration (#1110)
- prost-types: Allow unknown local time offset (#1109)

## Styling

- Remove use of legacy numeric constants (#1089)
- Move encoding functions into separate modules (#1111)
- Remove needless borrow (#1122)

## Testing

- Add tests for public interface of DecodeError (#1120)
- Add `parse_date` fuzzing target (#1127)
- Fix build without std (#1134)
- Change some proptest to kani proofs (#1133)
- Add `parse_duration` fuzzing target (#1129)
- fuzz: Fix building of fuzzing targets (#1107)
- fuzz: Add fuzz targets to workspace (#1117)

## Miscellaneous Tasks

- Move old protobuf benchmark into prost (#1100)
- Remove allow clippy::derive_partial_eq_without_eq (#1115)
- Run `cargo test` without `all-targets` (#1118)
- dependabot: Add github actions (#1121)
- Update to cargo clippy version 1.80 (#1128)

## Build

- Use `proc-macro` in Cargo.toml (#1102)
- Ignore missing features in `tests` crates (#1101)
- Use separated build directory for protobuf (#1103)
- protobuf: Don't install unused test proto (#1116)
- protobuf: Use crate `cmake` (#1137)
- deps: Update devcontainer to Debian Bookworm release (#1114)
- deps: Bump actions/upload-artifact from 3 to 4 (#1123)
- deps: Bump baptiste0928/cargo-install from 2 to 3 (#1124)
- deps: bump model-checking/kani-github-action from 0.32 to 1.1 (#1125)

# Prost version 0.13.1

_PROST!_ is a [Protocol Buffers](https://protobuf.dev/) implementation for the [Rust Language](https://www.rust-lang.org/). `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.

## Bug fixes

* Enum variant named Error causes ambiguous item (#1098)

# PROST version 0.13.0

**note**: this version was yanked in favor of 0.13.1

_PROST!_ is a [Protocol Buffers](https://protobuf.dev/) implementation for the [Rust Language](https://www.rust-lang.org/). `prost` generates simple, idiomatic Rust code from `proto2` and `proto3` files.

This major update brings new features and fixes:

## Breaking changes
- derive Copy trait for messages where possible (#950)

  `prost-build` will automatically derive `trait Copy` for some messages. If you manually implement `Copy` you should remove your implementation.

- Change generated functions signatures to remove type parameters (#1045)

  The function signature of `trait Message` is changed to use `impl Buf` instead of a named generic type. If you implement `trait Message`, you should change the function signature.

- Lightweight error value in TryFrom<i32> for enums (#1010)

  When a `impl TryFrom<i32>` is generated by `prost` derive macros, it will now return the error type `UnknownEnumValue` instead of `DecodeError`. The new error can be used to retreive the integer value that failed to convert.

## Features
- fix: Only touch include file if contents is changed (#1058)

  Most generated files are untouched when the contents doesn't change. Use the same mechanism for include file as well.

## Dependencies
- update env_logger requirement from 0.10 to 0.11 (#1074)
- update criterion requirement from 0.4 to 0.5 (#1071)
- Remove unused libz-sys (#1077)
- build(deps): update itertools requirement from >=0.10, <=0.12 to >=0.10, <=0.13 (#1070)

## Documentation
- better checking of tag duplicates, avoid discarding invalid variant errs (#951)
- docs: Fix broken link warnings (#1056)
- Add missing LICENSE symlink (#1086)

## Internal
- workspace package metadata (#1036)
- fix: Build error due to merge conflict (#1068)
- build: Fix release scripts (#1055)
- chore: Add ci to check MSRV (#1057)
- ci: Add all tests pass job (#1069)
- ci: Add Dependabot (#957)
- ci: Ensure both README are the same and prost version is correct  (#1078)
- ci: Set rust version of clippy job to a fixed version (#1090)
