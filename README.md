# lz4_rust

A Rust implementation of LZ4 block and frame compression, with file compression/decompression, optional checksums, configurable block sizes, and parallel processing.

## Build

```bash
cargo build --release
```

## Usage

```bash
cargo run --release -- <input-file>                 # Compress to <input-file>.lz4
cargo run --release -- -d <input-file>.lz4           # Decompress to the name without .lz4
cargo run --release -- -j 4 --block-size 6 <file>    # Use parallel processing and 1 MB blocks
```

Block sizes: `4` = 64 KB, `5` = 256 KB, `6` = 1 MB, `7` = 4 MB. Content checksums are enabled by default. Use `--BC` to enable block checksums and `--CS <bytes>` to include the original content size. Compression is the default mode.

## Parameters

| Option | Description | Default / behavior |
|---|---|---|
| `<input-file>` | Required input file path. | Required. |
| `-c`, `--compress` | Select compression mode. | Compression is already the default. Conflicts with `-d`. |
| `-d`, `--decompress` | Decompress an LZ4 frame. | Disabled. |
| `-j`, `--jobs <N>` | Requested worker/thread count. | `1`. `0` uses Rayon’s current thread count; positive values are capped at that count. An effective count of 1–2 uses the sequential path; more than 2 uses the parallel path. |
| `-b`, `--block-size <4-7>` | Maximum uncompressed block size for compression. | `7` (4 MB). `4` = 64 KB, `5` = 256 KB, `6` = 1 MB, `7` = 4 MB. |
| `--BI`, `--block-independance` | Use independent blocks. The long alias is spelled this way in the current CLI. | Enabled by default. Block-dependent frames are not supported by the decoder. |
| `--BC`, `--block-checksum` | Include a checksum for each block. | Disabled. |
| `--CS <BYTES>`, `--content-size <BYTES>` | Store the original uncompressed size in the frame header. | Omitted. The value is supplied manually; it is not calculated from the input file or checked against the actual size. |
| `--CC`, `--content-checksum` | Include a checksum for the uncompressed content. | Enabled by default |
| `--DI <ID>`, `--dictionary-id <ID>` | Dictionary ID option. | Not implemented; the CLI does not use this value, and dictionary-based frames are unsupported. |
| `-h`, `--help` | Show CLI help. | Built in. |
| `-V`, `--version` | Show program version. | Built in. |

## Benchmark
Benchmark performed against the refrenece `lz4` implementation on the Silesia corpus using [Hyperfine](https://github.com/sharkdp/hyperfine).
| File | Rust time (ms) | Reference `lz4` time (ms) | Runtime vs. reference | Rust peak memory (MiB) | Reference `lz4` peak memory (MiB) | Memory vs. reference |
|:---|---:|---:|---:|---:|---:|---:|
| dickens | 46.5 ± 1.0 | 40.1 ± 0.5 | 15.9% slower | 10.9 ± 0.1 | 16.5 ± 0.6 | 33.7% less |
| mozilla | 141.9 ± 5.1 | 123.7 ± 1.9 | 14.7% slower | 10.9 ± 0.1 | 18.1 ± 0.8 | 39.7% less |
| mr | 31.3 ± 0.4 | 27.9 ± 5.0 | 11.9% slower | 10.9 ± 0.1 | 16.2 ± 0.5 | 32.6% less |
| nci | 55.8 ± 1.0 | 56.1 ± 0.6 | 0.6% faster | 8.9 ± 0.1 | 14.0 ± 0.0 | 36.1% less |
| ooffice | 23.5 ± 1.0 | 18.7 ± 0.4 | 25.6% slower | 10.9 ± 0.1 | 12.0 ± 0.0 | 8.5% less |
| osdb | 30.8 ± 0.4 | 30.8 ± 0.4 | ~0.0% (similar) | 10.9 ± 0.4 | 16.1 ± 0.5 | 32.5% less |
| reymont | 25.7 ± 0.4 | 24.7 ± 0.3 | 4.1% slower | 10.8 ± 0.5 | 10.3 ± 0.3 | 5.2% more |
| samba | 55.7 ± 0.7 | 52.6 ± 0.8 | 5.9% slower | 10.9 ± 0.4 | 14.0 ± 0.4 | 22.6% less |
| sao | 27.1 ± 0.4 | 21.8 ± 0.4 | 24.3% slower | 10.9 ± 0.1 | 12.9 ± 0.3 | 15.1% less |
| webster | 155.5 ± 3.0 | 135.6 ± 0.7 | 14.7% slower | 10.8 ± 0.4 | 14.0 ± 0.1 | 22.8% less |
| x-ray | 18.8 ± 0.4 | 14.8 ± 2.8 | 27.6% slower | 10.9 ± 0.1 | 17.6 ± 0.9 | 37.9% less |
| xml | 13.7 ± 0.5 | 12.2 ± 0.7 | 12.4% slower | 9.0 ± 0.1 | 10.0 ± 0.3 | 10.4% less |

**Memory summary:** Rust uses less peak memory on 11/12 files. Median per-file memory reduction: 32.5%.

## Limitations

- Supports independent blocks only; block-dependent frames and dictionary IDs are unsupported.
- Parallel processing is selected only when the worker count is greater than two; `-j 1` is sequential.
- Parallel decompression does not currently validate block checksums and has less robust malformed-input/error handling than the sequential path.
- Decompression expects the input filename to end in `.lz4`.
