extern crate bytes;
extern crate env_logger;
extern crate test_all_types;
extern crate prost;
#[macro_use]
extern crate prost_derive;

include!(concat!(env!("OUT_DIR"), "/conformance.rs"));

use std::io::{
    Cursor,
    Read,
    Write,
    self,
};

use bytes::{
    Buf,
    ByteOrder,
    LittleEndian,
};
use prost::Message;

use test_all_types::{
    RoundtripResult,
    test_all_types_proto3_roundtrip,
};

fn main() {
    env_logger::init().unwrap();
    let mut bytes = Vec::new();

    loop {
        bytes.resize(4, 0);

        if let Err(_) = io::stdin().read_exact(&mut bytes[..]) {
            // No more test cases.
            break;
        }

        let len = LittleEndian::read_u32(&bytes[..]) as usize;

        bytes.resize(len, 0);
        io::stdin().read_exact(&mut bytes[..]).unwrap();

        let result = match ConformanceRequest::decode(&mut Buf::take(Cursor::new(&mut bytes), len)) {
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
        Some(WireFormat::Json) => {
            return conformance_response::Result::Skipped("JSON output is not supported".to_string());
        },
        None => {
            return conformance_response::Result::ParseError("unrecognized requested output format".to_string());
        },
        _ => (),
    };

    let buf = match request.payload {
        None => return conformance_response::Result::ParseError("no payload".to_string()),
        Some(conformance_request::Payload::JsonPayload(_)) =>
            return conformance_response::Result::Skipped("JSON input is not supported".to_string()),
        Some(conformance_request::Payload::ProtobufPayload(buf)) => buf,
    };

    match test_all_types_proto3_roundtrip(&buf) {
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
