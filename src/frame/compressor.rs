use std::{hash::Hasher, io::{BufRead, BufReader, BufWriter, Read, Write}};

use crossbeam_channel::bounded;
use twox_hash::XxHash32;

use crate::{block::compress_block, frame::{DATA_TYPE_FLAG, END_MARK, Error, header::FrameHeader}};

const WINDOW_SIZE: usize = 64 * 1024;

pub fn compress_frame<R: Read + Send, W: Write + Send>(input: &mut BufReader<R>, out: &mut BufWriter<W>, header: FrameHeader, num_workers: usize) -> Result<(), Error> {
    match num_workers {
        1..=2 => compress_frame_seq(input, out, header),
        _ => {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(num_workers).build().unwrap();
            pool.install(|| {
                compress_frame_par(input, out, header, num_workers)
            })
        }
    }
}

fn compress_frame_seq<R: Read, W: Write>(input: &mut BufReader<R>, out: &mut BufWriter<W>, header: FrameHeader) -> Result<(), Error> {
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

fn compress_frame_par<R: Read + Send, W: Write + Send>(input: &mut BufReader<R>, out: &mut BufWriter<W>, header: FrameHeader, num_workers: usize) -> Result<(), Error> {
    let mut hasher = XxHash32::with_seed(0);

    let max_size = header.maximum_size.get_bytes();

    let (raw_tx, raw_rx) = bounded::<(usize, Vec<u8>)>(num_workers * 2);
    let (comp_tx, comp_rx) = bounded::<(usize, Vec<u8>, Vec<u8>)>(num_workers * 2);

    out.write_all(&header.write())?;

    rayon::scope(|s| {
        // reader thread
        s.spawn(move |_| {
            let mut index = 0;
            let mut read_buf = vec![0u8; max_size];

            loop {
                let n = input.read(&mut read_buf).unwrap();
                if n == 0 { break; }
                raw_tx.send((index, read_buf[..n].to_vec())).unwrap();
                index += 1;
            }
        });

        // compressor workers
        for _ in 0..num_workers {
            let raw_rx = raw_rx.clone();
            let comp_tx = comp_tx.clone();
            let mut table = Box::new([0u32; WINDOW_SIZE]);
            let mut buf_compressed = vec![0u8; max_size * 2];
            s.spawn(move |_| {
                while let Ok((index, block)) = raw_rx.recv() {
                    let written = compress_block(&block, &mut buf_compressed, &mut table);
                    comp_tx.send((index, block, buf_compressed[..written].to_vec())).unwrap();
                }
            });
        }

        drop(comp_tx);
        drop(raw_rx);

        // writer thread
        let mut next_index = 0;
        let mut pending = std::collections::HashMap::new();

        for (index, block, compressed) in &comp_rx {
            pending.insert(index, (block, compressed));

            while let Some((block, compressed)) = pending.remove(&next_index) {
                if header.content_checksum {
                    hasher.write(&block);
                }

                // if uncompressable write raw block
                if compressed.len() > max_size {
                    let raw_size = (block.len() as u32) | DATA_TYPE_FLAG;

                    out.write_all(&u32::to_le_bytes(raw_size)).unwrap();
                    out.write_all(&block).unwrap();

                    if header.block_checksum {
                        let hash = XxHash32::oneshot(0, &block) as u32;
                        out.write_all(&u32::to_le_bytes(hash)).unwrap();
                    }
                } else {
                    out.write_all(&u32::to_le_bytes(compressed.len() as u32)).unwrap();
                    out.write_all(&compressed).unwrap();

                    if header.block_checksum {
                        let hash = XxHash32::oneshot(0, &compressed) as u32;
                        out.write_all(&u32::to_le_bytes(hash)).unwrap();
                    }
                }
                next_index += 1;
            }
        }
    });

    out.write_all(&END_MARK)?;

    if header.content_checksum {
        let hash = hasher.finish_32();
        out.write_all(&u32::to_le_bytes(hash))?;
    }

    Ok(())
}
