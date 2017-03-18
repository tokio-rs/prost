# Scalar Types

double   | 64-bit           | f64
float    | 32-bit           | f32
int32    | Varint           | i32
int64    | Varint           | i64
uint32   | Varint           | u32
uint64   | Varint           | u64
sint32   | Varint           | i32[Signed]
sint64   | Varint           | i64[Signed]
fixed32  | 32-bit           | u32[Fixed]
fixed64  | 64-bit           | u64[fixed]
sfixed32 | 32-bit           | i32[Fixed]
sfixed64 | 64-bit           | i64[Fixed]
bool     | Varint           | bool
string   | Length-delimited | String
bytes    | Length-delimited | Vec<u8>
enum     | Varint           | enum

# Nested Types

message | Length-delimited | message
repeated <scalar> | packed -> Length-delimited | unpacked -> wire_type(<scalar>)
repeated <message> | Length-delimited
map<scalar, other> | Length-delimited
