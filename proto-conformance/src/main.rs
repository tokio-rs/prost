extern crate bytes;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate proto;
#[macro_use]
extern crate proto_derive;

mod conformance;
mod protobuf_unittest;
mod protobuf_unittest_import;

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
use proto::Message;

use conformance::{
    conformance_request,
    conformance_response,
    ConformanceRequest,
    ConformanceResponse,
    WireFormat,
};

fn main() {
    env_logger::init().unwrap();
    let mut bytes = Vec::new();

    loop {
        bytes.resize(4, 0);

        io::stdin().read_exact(&mut bytes[..]).expect("input closed");
        let len = LittleEndian::read_u32(&bytes[..]) as usize;

        trace!("request len: {}", len);

        bytes.resize(len, 0);
        io::stdin().read_exact(&mut bytes[..]).unwrap();

        let result = match ConformanceRequest::decode(&mut Buf::take(Cursor::new(&mut bytes), len)) {
            Ok(request) => handle_request(request),
            Err(error) => conformance_response::Result::ParseError(format!("{:?}", error)),
        };

        let mut response = ConformanceResponse::default();
        response.result = Some(result);

        trace!("response: {:#?}", response);

        let len = response.encoded_len();
        trace!("response len: {}", len);
        bytes.resize(4, 0);

        LittleEndian::write_u32(&mut bytes[..4], len as u32);
        response.encode(&mut bytes);
        assert_eq!(len + 4, bytes.len());

        let mut stdout = io::stdout();
        stdout.lock().write_all(&bytes).unwrap();
        stdout.flush().unwrap();
    }
}

fn handle_request(request: ConformanceRequest) -> conformance_response::Result {
    trace!("request: {:#?}", request);

    if let WireFormat::Json = request.requested_output_format {
        return conformance_response::Result::Skipped("JSON output is not supported".to_string());
    }

    if let conformance_request::Payload::JsonPayload(_) = request.payload.unwrap() {
        return conformance_response::Result::Skipped("JSON input is not supported".to_string());
    }

    return conformance_response::Result::RuntimeError("unimplemented".to_string());
}
