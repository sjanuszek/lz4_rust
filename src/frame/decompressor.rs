use std::{hash::Hasher, io::{BufReader, BufWriter, Read, Write}};

use twox_hash::XxHash32;

use crate::{block::decompress_block, frame::{DATA_TYPE_FLAG, END_MARK, Error, header::FrameHeader}};

static BLOCK_SIZE_FLAG: u32 = 0x7FFFFFFF;

pub fn decompress_frame<R: Read, W: Write>(input: &mut BufReader<R>, out: &mut BufWriter<W>) -> Result<(), Error> {
    let header = FrameHeader::read(input)?;

    if let Some(_) = header.dictionary_id {
        return Err(Error::UnsupportedDictionaryId);
    } else if !header.block_independance {
        return Err(Error::UnsupportedBlockDependance);
    }

    let mut hasher = XxHash32::with_seed(0);

    let mut block_buf = vec![0u8; header.maximum_size.get_bytes()];
    let mut data_buf = vec![0u8; header.maximum_size.get_bytes()];

    loop {
        let mut raw_size_buf = [0u8; 4];
        input.read_exact(&mut raw_size_buf)?;
        
        if raw_size_buf == END_MARK { break; }

        let raw_size = u32::from_le_bytes(raw_size_buf);

        let block_size = (raw_size & BLOCK_SIZE_FLAG) as usize;

        if block_size > header.maximum_size.get_bytes() {
            return Err(Error::WrongBlockSize { expected: header.maximum_size.get_bytes(), gotten: block_size });
        }

        let block_slice = &mut block_buf[..block_size];
        input.read_exact(block_slice)?;

        if header.block_checksum {
            let mut block_checksum_buf = [0u8; 4];
            input.read_exact(&mut block_checksum_buf)?;

            let block_checksum = u32::from_le_bytes(block_checksum_buf);

            let hash = XxHash32::oneshot(0, block_slice) as u32;

            if block_checksum != hash {
                return Err(Error::WrongBlockChecksum);
            }
        }

        if raw_size & DATA_TYPE_FLAG != 0 {
            if header.content_checksum {
                hasher.write(block_slice);
            }
            out.write_all(block_slice)?;
        } else {
            match decompress_block(block_slice, &mut data_buf) {
                Ok(written) => {
                    let decompressed_slice = &data_buf[..written];

                    if header.content_checksum {
                        hasher.write(decompressed_slice);
                    }
                    out.write_all(decompressed_slice)?;
                },
                Err(e) => return Err(Error::BlockDecompressorError(e))
            }

        }
    }

    if header.content_checksum {
        let mut content_checksum_buf = [0u8; 4];
        input.read_exact(&mut content_checksum_buf)?;

        let content_checksum = u32::from_le_bytes(content_checksum_buf);

        let hash = hasher.finish_32();

        if content_checksum != hash {
            return Err(Error::WrongContentChecksum);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use twox_hash::XxHash32;

    fn decompress_to_vec(input: &[u8]) -> Vec<u8> {
        let mut reader = BufReader::new(Cursor::new(input));
        let mut out = Vec::new();
        let mut writer = BufWriter::new(&mut out);
        let _ = decompress_frame(&mut reader, &mut writer);
        drop(writer);
        out
    }

    #[test]
    fn simplest_frame() {
        let tohash = [0b01100000, 0b01110000];
        let test = [0x04u8, 0x22, 0x4D, 0x18, 0b01100000, 0b01110000, (XxHash32::oneshot(0, &tohash) >> 8) as u8, 0x05, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43, 0x44, 0x00, 0x00, 0x00, 0x00];

        let expected = b"ABCD";
        let result = decompress_to_vec(&test);

        assert_eq!(expected.to_vec(), result);
    }

    #[test]
    fn long_frame() {
        let test = [4, 34, 77, 24, 100, 64, 167, 10, 0, 0, 0, 26, 65, 1, 0, 80, 65, 65, 65, 65, 65, 0, 0, 0, 0, 102, 67, 159, 94];
        let expected= b"AAAAAAAAAAAAAAAAAAAA";

        let result = decompress_to_vec(&test);

        assert_eq!(expected.to_vec(), result);
    }

    #[test]
    fn name() {
        let test = [4, 34, 77, 24, 100, 64, 167, 16, 0, 0, 0, 63, 65, 66, 67, 3, 0, 255, 255, 255, 207, 80, 66, 67, 65, 66, 67, 0, 0, 0, 0, 16, 214, 175, 74];
        let expected: String = (0..333)
            .map(|_| "ABC")
            .collect();

        let result = decompress_to_vec(&test);

        assert_eq!(expected.as_bytes(), result)
    }
}
