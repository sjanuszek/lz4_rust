mod compressor;
mod decompressor;

static MIN_MATCHLENGTH: usize = 4;
static TOKEN_UPPERBOUND: usize = 15;

pub use compressor::compress_block;
pub use decompressor::decompress_block;
