use std::{fs::{self, File}, io::{BufReader, BufWriter, Write}, time::Instant};
use clap::Parser;

use crate::frame::{FrameHeader, MaximumSize, compress_frame, decompress_frame};

mod block;
mod frame;

#[derive(Parser)]
#[command(version, about = "lz4 file compression and decompression written in rust", long_about = None)]
struct Cli {
    /// Compress. This is the default operation mode when no operation mode parameter is provided
    #[arg(short, long, conflicts_with = "decompress")]
    compress: bool,

    /// Decompress
    #[arg(short, long)]
    decompress: bool,

    input_file: String,

    /// Number of threads to use for compression or decompression (minimum 3 due to pipeline architecture: reader + workers + writer)
    #[arg(short, long = "jobs", default_value_t = 1)]
    jobs: usize,

    /// Produces independent blocks (default)
    #[arg(long = "BI", visible_alias = "block-independance", default_value_t = true)]
    block_independence: bool,

    /// Generates block checksums (default:disabled)
    #[arg(long = "BC", visible_alias = "block-checksum", default_value_t = false)]
    block_checksum: bool,

    /// Header includes original size (default:not present)
    #[arg(long = "CS", visible_alias = "content-size")]
    content_size: Option<u64>,

    /// Generates content checksum (default:enabled)
    #[arg(long = "CC", visible_alias = "content-checksum", default_value_t = true)]
    content_checksum: bool,

    /// Compress or decompress using dictionary set with dictionary id (not implemented)
    #[arg(long = "DI", visible_alias = "dictionary-id")]
    dictionary_id: Option<u32>,

    /// Block size [4-7] 
    ///-b 4 = 64KB ; -b 5 = 256KB ; -b 6 = 1MB ; -b 7 = 4MB
    #[clap(verbatim_doc_comment)]
    #[arg(short, long = "block-size", default_value_t = 7, value_parser = clap::value_parser!(u8).range(4..=7))]
    block_size: u8
}


fn main() {
    let cli = Cli::parse();

    let path = &cli.input_file;

    let input_stream = File::open(path).unwrap();

    if cli.decompress{
        let threads = rayon::current_num_threads();
        let num_workers = if cli.jobs == 0 {
            threads
        } else {
            cli.jobs.min(threads)
        };

        let name = path.strip_suffix(".lz4").unwrap();
        let mut out = BufWriter::new(File::create(name).unwrap());
        let mut input = BufReader::new(input_stream);
        
        let start = Instant::now();
        let res = decompress_frame(&mut input, &mut out, num_workers);
        let elapsed = start.elapsed();
        match res {
            Ok(_) => {
                out.flush().unwrap();
                let input_size = fs::metadata(path).unwrap().len();
                let output_size = fs::metadata(&name).unwrap().len();
                let percentage = (input_size as f64 / output_size as f64) * 100.;
                let throughput = output_size as f64 / elapsed.as_secs_f64() / 1000000.0;
                println!("{} => {} ({} bytes => {} bytes, {:.2}%) in {:.2?} ({:.1} MB/s)",path, name, input_size, output_size, percentage, elapsed, throughput);
            },
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        drop(out);
    } else {
        let header = FrameHeader {
            block_independance: cli.block_independence,
            block_checksum: cli.block_checksum,
            content_size: cli.content_size,
            content_checksum: cli.content_checksum,
            dictionary_id: None,
            maximum_size: match cli.block_size {
                4 => MaximumSize::KB64,
                5 => MaximumSize::KB256,
                6 => MaximumSize::MB1,
                7 => MaximumSize::MB4,
                _ => unreachable!(),
            },
        };

        let threads = rayon::current_num_threads();
        let num_workers = if cli.jobs == 0 {
            threads
        } else {
            cli.jobs.min(threads)
        };

        let name = format!("{}.lz4", path);
        let mut out = BufWriter::new(File::create(&name).unwrap());
        let mut input = BufReader::with_capacity(header.maximum_size.get_bytes(), input_stream);

        let start = Instant::now();
        let res = compress_frame(&mut input, &mut out, header, num_workers);
        let elapsed = start.elapsed();
        match res {
            Ok(_) => {
                out.flush().unwrap();
                let input_size = fs::metadata(path).unwrap().len();
                let output_size = fs::metadata(&name).unwrap().len();
                let percentage = (output_size as f64 / input_size as f64) * 100.;
                let throughput = input_size as f64 / elapsed.as_secs_f64() / 1000000.0;
                println!("{} => {} ({} bytes => {} bytes, {:.2}%) in {:.2?} ({:.1} MB/s)",path, name, input_size, output_size, percentage, elapsed, throughput);
            },
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        drop(out);
    }
}
