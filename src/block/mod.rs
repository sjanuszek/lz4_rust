mod compressor;
mod decompressor;

const MIN_MATCHLENGTH: usize = 4;
const TOKEN_UPPERBOUND: usize = 15;

pub use compressor::compress_block;
pub use decompressor::decompress_block;

#[derive(Debug)]
pub enum Error {
    OffsetZero
}
