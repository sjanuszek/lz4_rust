use twox_hash::XxHash32;

use crate::{block::compress_block, frame::{END_MARK, header::FrameHeader}};

// should return Result
// needs a refactor
// temporary return
pub fn compress_frame(input: &[u8], header: FrameHeader) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();

    out.extend_from_slice(&header.write());

    input
        .chunks(header.maximum_size.get_bytes())
        .for_each(|block| {
            let mut buf = vec![0u8; block.len() * 2];
            let written = compress_block(block, &mut buf);

            buf.truncate(written);

            out.extend_from_slice(&u32::to_le_bytes(written.try_into().unwrap()));
            out.extend_from_slice(&buf);

            if header.block_checksum {
                let hash = XxHash32::oneshot(0, &buf) as u32;
                out.extend_from_slice(&u32::to_le_bytes(hash));
            }
        });

    out.extend_from_slice(&END_MARK);

    if header.content_checksum {
        let hash = XxHash32::oneshot(0, input) as u32;
        out.extend_from_slice(&u32::to_le_bytes(hash));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{decompress_frame, header::MaximumSize};

    #[test]
    fn thing() {
        let input = b"ABCD";
        let header = FrameHeader {
            block_independance: true,
            block_checksum: false,
            content_size: Some(input.len() as u64),
            content_checksum: true,
            dictionary_id: None,
            maximum_size: MaximumSize::KB64
        };

        let compressed = compress_frame(input, header);
        let decompressed = decompress_frame(&compressed);

        assert_eq!(input.to_vec(), decompressed)
    }
}
