extern crate prost;
extern crate prost_types;

use crate::prost::Message;
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};
use std::io::{Error, ErrorKind, Result};
use std::io::{Read, Write};

fn main() -> Result<()> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;

    let request = CodeGeneratorRequest::decode(&*buf).map_err(|error| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("invalid FileDescriptorSet: {}", error.to_string()),
        )
    })?;

    let response: CodeGeneratorResponse = prost_build::Config::new().run_plugin(request);

    let mut out = Vec::new();
    response.encode(&mut out).map_err(|error| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("invalid FileDescriptorSet: {}", error.to_string()),
        )
    })?;
    std::io::stdout().write_all(&out)?;

    Ok(())
}
