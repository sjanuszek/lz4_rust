# lz4_rust

A Rust implementation of LZ4 block and frame compression/decompression. It supports configurable block sizes, optional block checksums, content checksums, and sequential or parallel processing.

## Build

```sh
cargo build --release
```

## Usage

```sh
./lz4_rust <input-file>                   # Compress; creates <input-file>.lz4
./lz4_rust --decompress <input-file>.lz4  # Decompress; removes the .lz4 suffix for output
./lz4_rust --jobs 4 --block-size 6 <file> # Request 4 workers and 1 MB blocks
```

Compression is the default. Decompression expects the input filename to end in `.lz4`. The program chooses the output filename automatically and creates or overwrites that file.

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

## Limitations

- Supports LZ4 frames with independent blocks and block sizes from 64 KB to 4 MB; block-dependent frames and dictionaries are unsupported.
- Parallel decompression currently does not validate block checksums. Its malformed-input handling is also less robust than the sequential path and may panic on invalid or corrupted data.
- `--content-size` only writes the supplied value to the header; it is not automatically calculated or validated.
- Decompression requires a filename ending in `.lz4`; the output name is derived by removing that suffix.
