use std::{hash::Hasher, io::{BufRead, BufReader, BufWriter, Read, Write}};

use twox_hash::XxHash32;

use crate::{block::compress_block, frame::{DATA_TYPE_FLAG, END_MARK, Error, header::FrameHeader}};

const WINDOW_SIZE: usize = 64 * 1024;

pub fn compress_frame<R: Read, W: Write>(input: &mut BufReader<R>, out: &mut BufWriter<W>, header: FrameHeader) -> Result<(), Error> {
    let mut hasher = XxHash32::with_seed(0);
    let mut read_buf = vec![0u8; header.maximum_size.get_bytes()];
    let mut buf_compressed = vec![0u8; header.maximum_size.get_bytes() * 2];
    let mut table = Box::new([0u32; WINDOW_SIZE]);

    out.write_all(&header.write())?;

    loop {
        let n = input.read(&mut read_buf)?;
        if n == 0 { break; }
        let buf = &read_buf[..n];

        if header.content_checksum {
            hasher.write(buf);
        }

        let written = compress_block(buf, &mut buf_compressed, &mut table);

        if written > header.maximum_size.get_bytes() {
            let raw_size = (n as u32) | DATA_TYPE_FLAG;

            out.write_all(&u32::to_le_bytes(raw_size))?;
            out.write_all(buf).unwrap();

            if header.block_checksum {
                let hash = XxHash32::oneshot(0, buf) as u32;
                out.write_all(&u32::to_le_bytes(hash))?;
            }
        } else {
            out.write_all(&u32::to_le_bytes(written as u32))?;
            out.write_all(&buf_compressed[..written])?;

            if header.block_checksum {
                let hash = XxHash32::oneshot(0, &buf_compressed) as u32;
                out.write_all(&u32::to_le_bytes(hash))?;
            }
        }

        input.consume(n);
    }

    out.write_all(&END_MARK)?;

    if header.content_checksum {
        let hash = hasher.finish_32();
        out.write_all(&u32::to_le_bytes(hash))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::header::MaximumSize;
    use std::io::Cursor;

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
        
        let mut compressed = Vec::new();
        let mut reader = BufReader::with_capacity(header.maximum_size.get_bytes(), Cursor::new(input));
        let mut writer = BufWriter::new(&mut compressed);
        
        let _ = compress_frame(&mut reader, &mut writer, header);
        drop(writer);
        
        // let decompressed = decompress_frame(&compressed);
        // assert_eq!(input, decompressed.as_slice());
    }
}
