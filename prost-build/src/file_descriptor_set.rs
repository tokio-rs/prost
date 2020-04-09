use std::io::{prelude::*, Result};
use std::path::PathBuf;

use flate2::bufread::GzEncoder;
use flate2::Compression;
use log::trace;

const FDSET_NAME: &str = "fdset";

pub fn compile(target: &PathBuf, contents: &Vec<u8>) -> Result<()> {
    let output_path = target.join(FDSET_NAME);
    let previous_content = std::fs::read(&output_path);

    let new_content = gzip(&contents)?;

    if previous_content
        .map(|previous_content| previous_content == new_content)
        .unwrap_or(false)
    {
        trace!("unchanged: {:?}", FDSET_NAME);
    } else {
        trace!("writing: {:?}", FDSET_NAME);
        std::fs::write(output_path, new_content)?;
    }

    Ok(())
}

fn gzip(buf: &[u8]) -> Result<Vec<u8>> {
    let mut gzip_buf = Vec::new();
    let mut gz = GzEncoder::new(buf, Compression::default());
    if gz.read_to_end(&mut gzip_buf)? == 0 {
        Err(std::io::ErrorKind::InvalidData.into())
    } else {
        Ok(gzip_buf)
    }
}
