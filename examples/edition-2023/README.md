# 🪄 Protobuf Edition 2023 Example

This example demonstrates how to use `prost` with Protocol Buffers Edition 2023 syntax.

## 📋 Overview

The example includes:

- 📄 **`bar.proto`**: A protobuf file using `edition = "2023"` syntax with an `Order` message containing a timestamp,
  `beverage` enum, `message` string, and `tip` amount.
- 🔧 **`build.rs`**: Uses `prost-build` to generate Rust code from the proto file at build time.
- 🦀 **`src/main.rs`**: A program that demonstrates protobuf serialization/deserialization with a humorous bartender.

## ✨ Key Features Demonstrated

1. 🎯 **Edition 2023 Syntax**: Shows how to work with the new protobuf editions syntax
2. 🔍 **Field Presence**: In edition 2023, fields have explicit presence by default, so all fields are `Option<T>`
3. 📦 **Serialization/Deserialization**: Encodes messages to protobuf binary format and decodes them back
4. ✅ **Field Presence Detection**: Checks whether optional fields are set after deserialization
5. 🔢 **Enum Handling**: Demonstrates working with protobuf enums in edition 2023
6. ⏰ **Well-Known Types**: Uses `google.protobuf.Timestamp` with ISO 8601 formatting

## 🚀 Running the Example

**Without a tip 🤔:**

```bash
cargo run -p edition-2023-example
```

**With a tip 🍻:**

```bash
cargo run -p edition-2023-example -- --tip 42
```

**Build and run standalone executable:**

```bash
# Build optimized binary
cargo build -p edition-2023-example --release

# Run the executable directly
./target/release/edition-2023-example --tip 42
```

## ⚙️ Command Line Arguments

- 💰 `--tip <amount>`: Optional tip amount in gold doubloons. Don't be stingy with the bartender 😉

## 🎓 What This Example Demonstrates

### 🔍 Explicit Field Presence

This example showcases **Protobuf Editions 2023's explicit field presence**:

- The `tip` field is `Option<u32>` (not just `u32`)
- When `tip` is `None`, it's not serialized (saves 2 bytes: 134 vs 136 bytes)
- After deserialization, you can distinguish between:
  - Field not set: `None`
  - Field set to zero: `Some(0)`
  - Field set to a value: `Some(42)`

This is a key improvement over `proto3`, where primitive fields had implicit presence and you couldn't distinguish
between "not set" and "set to default value".

### 🔄 Serialization Round-Trip

The example demonstrates a complete protobuf workflow:

1. ✏️ Create a message with fields
2. 📦 Serialize to binary protobuf format (`encode`)
3. 🔓 Deserialize back from bytes (`decode`)
4. ✅ Verify field presence is preserved

## 📝 Notes on Edition 2023

In [protobuf editions 2023](https://protobuf.dev/editions/overview/):

- All scalar fields have explicit presence (wrapped in `Option<T>`)
- Enums are open by default (allowing unknown values)
- Field presence semantics are more explicit and consistent
- Fields that are `None` are not serialized, reducing message size

This is different from `proto3` where primitive fields had implicit presence.
