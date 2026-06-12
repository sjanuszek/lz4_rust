mod compressor;
mod decompressor;

const MIN_MATCHLENGTH: usize = 4;
const TOKEN_UPPERBOUND: usize = 15;

use core::fmt;

pub use compressor::compress_block;
pub use decompressor::decompress_block;

#[derive(Debug)]
pub enum Error {
    OffsetZero
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            OffsetZero => write!(f, "offset bytes are set to zero"),
        }
    }
}
