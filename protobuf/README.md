# `protobuf`

`protobuf` is an internal library used by `prost` conformance tests, benchmarks,
and integration-tests. `protobuf` downloads, compiles, and installs the
[Protobuf][1] project, including the conformance test runner, `libprotobuf`,
benchmark data and test and benchmark .protos into the Cargo target directory.

[1]: https://github.com/google/protobuf/
