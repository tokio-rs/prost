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
    ConformanceRequest,
    ConformanceResponse,
};

fn main() {
    env_logger::init().unwrap();
    let mut bytes = vec![0; 4];

    io::stdin().read_exact(&mut bytes[..]).unwrap();
    let len = LittleEndian::read_u32(&bytes[..]) as usize;

    trace!("len: {}", len);

    bytes.resize(len, 0);
    io::stdin().read_exact(&mut bytes[..]).unwrap();

    let request = ConformanceRequest::decode(&mut Buf::take(Cursor::new(&mut bytes), len)).unwrap();
    let mut response = ConformanceResponse::default();

    trace!("request: {:#?}", request);
}
