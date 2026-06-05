mod compressor;
mod decompressor;
mod hash;
mod header;

const END_MARK: [u8; 4] = [0,0,0,0];
const DATA_TYPE_FLAG: u32 = 0x80000000;

pub use decompressor::decompress_frame;
pub use compressor::compress_frame;
pub use header::FrameHeader;
pub use header::MaximumSize;

#[derive(Debug)]
pub enum Error {
    WrongMagicNumber,
    WrongVersion(u8),
    FlagReservedBitsSet,
    BDReseverBitsSet,
    UndefinedBlockMaximumSize(u8),
    WrongHeaderChecksum,
    WrongBlockChecksum,
    WrongContentChecksum,
    IOError(std::io::Error),
    WrongBlockSize{expected: usize, gotten: usize},
    UnsupportedDictionaryId,
    UnsupportedBlockDependance,
    BlockDecompressorError(crate::block::Error)
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}
