mod compressor;
mod decompressor;
mod hash;
mod header;

const END_MARK: [u8; 4] = [0,0,0,0];
const DATA_TYPE_FLAG: u32 = 0x80000000;

use core::fmt;

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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            WrongMagicNumber => write!(f, "incorrect magic number, possibly a legacy frame"),
            WrongVersion(ver) => write!(f, "incorrect version bits: {:b}, should be 01", ver),
            FlagReservedBitsSet => write!(f, "reserved bits in the FLG byte are set"),
            BDReseverBitsSet => write!(f, "reserved bits in the BD byte are set"),
            UndefinedBlockMaximumSize(size) => write!(f, "undefined Block Maximum Size value: {}, needs to be between 4-7", size),
            WrongHeaderChecksum => write!(f, "incorrect header checksum, possibly corrupted header"),
            WrongBlockChecksum => write!(f, "incorrect block checksum, possibly corrupted block"),
            WrongContentChecksum => write!(f, "incorrect content checksum, possibly corrupted content after decompression"),
            IOError(err) => write!(f, "IO error: {}", err),
            WrongBlockSize{expected, gotten} => write!(f, "incorrect block size, expected: {}, gotten: {}", expected, gotten),
            UnsupportedDictionaryId => write!(f, "dictionary id paramater is not supported"),
            UnsupportedBlockDependance => write!(f, "block dependancy is not supported"),
            BlockDecompressorError(err) => write!(f, "block decompression error: {}", err)
        }
    }
}
