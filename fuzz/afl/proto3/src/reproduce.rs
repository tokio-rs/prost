use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::roundtrip_proto;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <path-to-crash>", args[0]);
        std::process::exit(1);
    }

    let data = std::fs::read(&args[1]).expect(&format!("Could not open file {}", args[1]));
    let _ = roundtrip_proto::<TestAllTypesProto3>(&data).unwrap_error();
}
