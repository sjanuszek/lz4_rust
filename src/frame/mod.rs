mod compressor;
mod decompressor;
mod hash;
mod header;

static END_MARK: [u8; 4] = [0,0,0,0];

pub use decompressor::decompress_frame;
pub use compressor::compress_frame;
pub use header::FrameHeader;
pub use header::MaximumSize;
