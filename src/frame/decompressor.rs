use std::{hash::Hasher, io::{BufReader, BufWriter, Read, Write}};

use crossbeam_channel::bounded;
use twox_hash::XxHash32;

use crate::{block::decompress_block, frame::{DATA_TYPE_FLAG, END_MARK, Error, header::FrameHeader}};

static BLOCK_SIZE_FLAG: u32 = 0x7FFFFFFF;

pub fn decompress_frame<R: Read + Send, W: Write + Send>(input: &mut BufReader<R>, out: &mut BufWriter<W>, num_workers: usize) -> Result<(), Error> {
    match num_workers {
        1..=2 => decompress_frame_seq(input, out),
        _ => {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(num_workers).build().unwrap();
            pool.install(|| {
                decompress_frame_par(input, out, num_workers)
            })
        }
    }
}

pub fn decompress_frame_par<R: Read + Send, W: Write + Send>(input: &mut BufReader<R>, out: &mut BufWriter<W>, num_workers: usize) -> Result<(), Error> {
    let header = FrameHeader::read(input)?;

    if let Some(_) = header.dictionary_id {
        return Err(Error::UnsupportedDictionaryId);
    } else if !header.block_independance {
        return Err(Error::UnsupportedBlockDependance);
    }

    let mut hasher = XxHash32::with_seed(0);

    let max_size = header.maximum_size.get_bytes();

    let (block_tx, block_rx) = bounded::<(usize, Vec<u8>)>(num_workers * 2);
    let (dcomp_tx, dcomp_rx) = bounded::<(usize, Vec<u8>)>(num_workers * 2);

    let (check_tx, check_rx) = bounded::<u32>(1);

    rayon::scope(|s| {
        // reader thread
        let dcomp_tx_clone = dcomp_tx.clone();
        s.spawn(move |_| {
            let mut block_buf = vec![0u8; max_size];
            let mut index = 0;
            let mut raw_size_buf = [0u8; 4];
            let mut block_checksum_buf = [0u8; 4];
            let mut content_checksum_buf = [0u8; 4];
            loop {
                input.read_exact(&mut raw_size_buf).unwrap();
                
                if raw_size_buf == END_MARK { break; }

                let raw_size = u32::from_le_bytes(raw_size_buf);

                let block_size = (raw_size & BLOCK_SIZE_FLAG) as usize;

                //if block_size > header.maximum_size.get_bytes() {
                //    return Err(Error::WrongBlockSize { expected: header.maximum_size.get_bytes(), gotten: block_size });
                //}

                let block_slice = &mut block_buf[..block_size];
                input.read_exact(block_slice).unwrap();

                if header.block_checksum {
                    input.read_exact(&mut block_checksum_buf).unwrap();

                    let block_checksum = u32::from_le_bytes(block_checksum_buf);

                    let hash = XxHash32::oneshot(0, block_slice) as u32;

                    //if block_checksum != hash {
                    //    return Err(Error::WrongBlockChecksum);
                    //}
                }
                if raw_size & DATA_TYPE_FLAG != 0 {
                    //if header.content_checksum {
                    //    hasher.write(block_slice);
                    //}
                    dcomp_tx_clone.send((index, block_slice.to_vec())).unwrap()
                } else {
                    block_tx.send((index, block_slice.to_vec())).unwrap();
                }
                index += 1;
            }
            if header.content_checksum {
                input.read_exact(&mut content_checksum_buf).unwrap();

                let content_checksum = u32::from_le_bytes(content_checksum_buf);
                check_tx.send(content_checksum).unwrap();
            }
        });

        // decompression workers
        for _ in 0..num_workers {
            let block_rx = block_rx.clone();
            let dcomp_tx = dcomp_tx.clone();
            let mut data_buf = vec![0u8; max_size];
            s.spawn(move |_| {
                while let Ok((index, block)) = block_rx.recv() {
                    let written = decompress_block(&block, &mut data_buf).unwrap();
                    dcomp_tx.send((index, data_buf[..written].to_vec())).unwrap()
                }
            });
        }

        drop(dcomp_tx);
        drop(block_rx);

        // writer thread
        let mut next_index = 0;
        let mut pending = std::collections::HashMap::new();

        for (index, decompressed) in &dcomp_rx {
            pending.insert(index, decompressed);

            while let Some(decompressed) = pending.remove(&next_index) {
                if header.content_checksum {
                    hasher.write(&decompressed);
                }
                out.write_all(&decompressed).unwrap();
                next_index += 1;
            }
        }
    });

    if header.content_checksum {
        while let Ok(content_checksum) = check_rx.recv() {
            let hash = hasher.finish_32();
        
            if content_checksum != hash {
                return Err(Error::WrongContentChecksum);
            }
        }

    }

    Ok(())
}
fn decompress_frame_seq<R: Read, W: Write>(input: &mut BufReader<R>, out: &mut BufWriter<W>) -> Result<(), Error> {
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
        let _ = decompress_frame(&mut reader, &mut writer, 1);
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
