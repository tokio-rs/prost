extern crate bytes;
extern crate env_logger;
extern crate prost;
extern crate tests;

#[macro_use]
extern crate prost_derive;

include!(concat!(env!("OUT_DIR"), "/conformance.rs"));

use std::io::{
    Read,
    Write,
    self,
};

use bytes::{
    ByteOrder,
    LittleEndian,
};
use prost::Message;

use tests::protobuf_test_messages::proto2::TestAllTypesProto2;
use tests::protobuf_test_messages::proto3::TestAllTypesProto3;
use tests::{
    RoundtripResult,
    roundtrip,
};

fn main() {
    env_logger::init();
    let mut bytes = Vec::new();

    loop {
        bytes.resize(4, 0);

        if io::stdin().read_exact(&mut bytes[..]).is_err() {
            // No more test cases.
            break;
        }

        let len = LittleEndian::read_u32(&bytes[..]) as usize;

        bytes.resize(len, 0);
        io::stdin().read_exact(&mut bytes[..]).unwrap();

        let result = match ConformanceRequest::decode(&bytes) {
            Ok(request) => handle_request(request),
            Err(error) => conformance_response::Result::ParseError(format!("{:?}", error)),
        };

        let mut response = ConformanceResponse::default();
        response.result = Some(result);

        let len = response.encoded_len();
        bytes.resize(4, 0);

        LittleEndian::write_u32(&mut bytes[..4], len as u32);
        response.encode(&mut bytes).unwrap();
        assert_eq!(len + 4, bytes.len());

        let mut stdout = io::stdout();
        stdout.lock().write_all(&bytes).unwrap();
        stdout.flush().unwrap();
    }
}

fn handle_request(request: ConformanceRequest) -> conformance_response::Result {
    match request.requested_output_format() {
        WireFormat::Unspecified => {
            return conformance_response::Result::ParseError("output format unspecified".to_string());
        },
        WireFormat::Json => {
            return conformance_response::Result::Skipped("JSON output is not supported".to_string());
        },
        WireFormat::Protobuf => (),
    };

    let buf = match request.payload {
        None => return conformance_response::Result::ParseError("no payload".to_string()),
        Some(conformance_request::Payload::JsonPayload(_)) =>
            return conformance_response::Result::Skipped("JSON input is not supported".to_string()),
        Some(conformance_request::Payload::ProtobufPayload(buf)) => buf,
    };

    let roundtrip = match &*request.message_type {
        "protobuf_test_messages.proto2.TestAllTypesProto2" => roundtrip::<TestAllTypesProto2>(&buf),
        "protobuf_test_messages.proto3.TestAllTypesProto3" => roundtrip::<TestAllTypesProto3>(&buf),
         _ => return conformance_response::Result::ParseError(
             format!("unknown message type: {}", request.message_type)),
    };

    match roundtrip {
        RoundtripResult::Ok(buf) => {
            conformance_response::Result::ProtobufPayload(buf)
        },
        RoundtripResult::DecodeError(error) => {
            conformance_response::Result::ParseError(error.to_string())
        },
        RoundtripResult::Error(error) => {
            conformance_response::Result::RuntimeError(error.to_string())
        },
    }
}
