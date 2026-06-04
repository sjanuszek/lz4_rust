use twox_hash::XxHash32;

use crate::{block::decompress_block, frame::{END_MARK, header::FrameHeader}};

static DATA_TYPE_FLAG: u32 = 0x80000000;
static BLOCK_SIZE_FLAG: u32 = 0x7FFFFFFF;

// this is shit
// should do with read_exact
// still need to handle block dependancy
pub fn decompress_frame(frame: &[u8]) -> Vec<u8> {
    let header = FrameHeader::read(frame);
    let mut pos = header.get_length();
    let mut out: Vec<u8> = Vec::new();
    
    // will want to do it in parallel
    while frame[pos..pos+4] != END_MARK{
        let raw_size = u32::from_le_bytes(frame[pos..pos+4].try_into().unwrap());
        pos += 4;

        if raw_size & DATA_TYPE_FLAG != 0 {
            panic!("UNCOMPRESSED IDK WHAT TO DO")
        }

        let block_size = (raw_size & BLOCK_SIZE_FLAG) as usize;

        if block_size > header.maximum_size.get_bytes() {
            panic!("SOMETHING IS FUCKY WUCKY")
        }

        let block = &frame[pos..pos+block_size];

        pos += block_size;

        if header.block_checksum {
            let block_checksum = u32::from_le_bytes(frame[pos..pos+4].try_into().unwrap());

            let hash = XxHash32::oneshot(0, block) as u32;

            if block_checksum != hash {
                panic!("INCORRECT BLOCK CHECKSUM")
            }
            pos += 4;
        }

        let mut buf: Vec<u8>;

        if let Some(content_size) = header.content_size {
            buf = vec![0u8; content_size as usize];
        } else {
            buf = vec![0u8; header.maximum_size.get_bytes()];
        }

        // temporary return
        let written = decompress_block(block, &mut buf);

        // temporary truncate
        buf.truncate(written);

        out.extend_from_slice(&buf);
    }

    if header.content_checksum {
        pos += 4;
        let content_checksum = u32::from_le_bytes(frame[pos..pos+4].try_into().unwrap());

        let hash = XxHash32::oneshot(0, &out) as u32;

        if content_checksum != hash {
            panic!("INCORRECT CONTENT CHECKSUM")
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use twox_hash::XxHash32;
    use super::*;

    #[test]
    fn simplest_frame() {
        let tohash = [0b01100000, 0b01110000];
        let test = [0x04u8, 0x22, 0x4D, 0x18, 0b01100000, 0b01110000, (XxHash32::oneshot(0, &tohash) >> 8) as u8, 0x05, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43, 0x44, 0x00, 0x00, 0x00, 0x00];

        let expected = b"ABCD";
        let result = decompress_frame(&test);

        assert_eq!(expected.to_vec(), result);
    }

    #[test]
    fn long_frame() {
        let test = [4, 34, 77, 24, 100, 64, 167, 10, 0, 0, 0, 26, 65, 1, 0, 80, 65, 65, 65, 65, 65, 0, 0, 0, 0, 102, 67, 159, 94];
        let expected= b"AAAAAAAAAAAAAAAAAAAA";

        let result = decompress_frame(&test);

        assert_eq!(expected.to_vec(), result);
    }

    #[test]
    fn name() {
        let test = [4, 34, 77, 24, 100, 64, 167, 16, 0, 0, 0, 63, 65, 66, 67, 3, 0, 255, 255, 255, 207, 80, 66, 67, 65, 66, 67, 0, 0, 0, 0, 16, 214, 175, 74];
        let expected: String = (0..333)
            .map(|_| "ABC")
            .collect();

        let result = decompress_frame(&test);

        assert_eq!(expected.as_bytes(), result)
    }
}
