use crate::block::{MIN_MATCHLENGTH, TOKEN_UPPERBOUND};

// 64kB as to fit into offset bytes
const WINDOW_SIZE: usize = 64 * 1024;
const LAST_SEQUENCE_LENGTH: usize = 5;
const PENULTIMATE_SEQUENCE_LENGTH: usize = 12;

const HASH_LOG: u32 = 12;
pub const HASH_TABLE_SIZE: usize = 1 << HASH_LOG;

#[inline(always)]
fn hash_4(sequence: u32) -> usize {
    ((sequence.wrapping_mul(2654435761u32)) >> (32 - HASH_LOG)) as usize
}

#[inline(always)]
fn read_u32(buf: &[u8], pos: usize) -> u32 {
    u32::from_le_bytes(buf[pos..pos + MIN_MATCHLENGTH].try_into().unwrap())
}

fn write_extra(mut extra: usize, out: &mut [u8], out_i: &mut usize) {
    let n = extra / 255;
    out[*out_i..*out_i + n].fill(255);
    *out_i += n;
    out[*out_i] = (extra % 255) as u8;
    *out_i += 1;
}

fn write_sequence(block: &[u8], out: &mut [u8], out_i: &mut usize, start_pos: usize, literals_count: usize, offset: u16, matchlength: usize) {
    let is_last = offset == 0;

    let lit_nibble = literals_count.min(TOKEN_UPPERBOUND) as u8;
    let mat_nibble = if is_last {
        0u8
    } else {
        (matchlength - MIN_MATCHLENGTH).min(TOKEN_UPPERBOUND) as u8
    };

    out[*out_i] = (lit_nibble << 4) | (mat_nibble);
    *out_i += 1;

    if literals_count >= TOKEN_UPPERBOUND {
        write_extra(literals_count - TOKEN_UPPERBOUND, out, out_i);
    }

    if literals_count <= 16 && start_pos + 16 <= block.len() && *out_i + 16 <= out.len() {
        out[*out_i..*out_i + 16].copy_from_slice(&block[start_pos..start_pos + 16]);
    } else {
        out[*out_i..*out_i + literals_count].copy_from_slice(&block[start_pos..start_pos + literals_count]);
    }
    *out_i += literals_count;

    if is_last { return; }

    let [low, high] = offset.to_le_bytes();

    out[*out_i] = low;
    out[*out_i + 1] = high;
    *out_i += 2;

    if matchlength - MIN_MATCHLENGTH >= TOKEN_UPPERBOUND {
        write_extra(matchlength - MIN_MATCHLENGTH - TOKEN_UPPERBOUND, out, out_i);
    }

}

#[inline(always)]
fn count_match(a: &[u8], b: &[u8]) -> usize {
    let limit = a.len().min(b.len());
    let (a, b) = (&a[..limit], &b[..limit]);
    let mut n = 0;

    while n + 8 <= limit {
        let x = u64::from_le_bytes(a[n..n + 8].try_into().unwrap());
        let y = u64::from_le_bytes(b[n..n + 8].try_into().unwrap());
        let diff = x ^ y;
        if diff != 0 {
            return n + (diff.trailing_zeros() / 8) as usize;
        }
        n += 8;
    }
    while n < limit && a[n] == b[n] { n += 1; }
    n
}

pub fn compress_block(block: &[u8], out: &mut [u8], table: &mut [u32; HASH_TABLE_SIZE]) -> usize {
    let mut i: usize = 0;
    let mut out_i: usize = 0;

    let mut lit_start_pos: usize = 0;

    let mut misses: usize =  0;
    // ensure we meet the end of block conditions
    while i < block.len().saturating_sub(PENULTIMATE_SEQUENCE_LENGTH) {
        let byte_sequence = read_u32(block, i);
        let hash = hash_4(byte_sequence);
        let j = table[hash] as usize;

        table[hash] = i as u32;

        if j < i && i - j < WINDOW_SIZE && read_u32(block, j) == byte_sequence {
            misses = 0;

            let end = block.len() - LAST_SEQUENCE_LENGTH;
            let matchlength = MIN_MATCHLENGTH + count_match(&block[i + MIN_MATCHLENGTH..end], &block[j + MIN_MATCHLENGTH..]);

            let offset: u16 = (i - j).try_into().unwrap();

            write_sequence(block, out, &mut out_i, lit_start_pos, i - lit_start_pos, offset, matchlength);

            i += matchlength;
            lit_start_pos = i;
        } else {
            i += 1 + (misses >> 6);
            misses += 1;
        }
    }

    // last sequence is pure literals
    write_sequence(block, out, &mut out_i, lit_start_pos, block.len() - lit_start_pos, 0, 0);

    out_i
}

#[cfg(test)]
mod tests {
    use crate::block::decompressor::decompress_block;
    use super::*;

    fn compress_block_to_vec(block: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; block.len() * 2];
        let mut table = Box::new([0u32; HASH_TABLE_SIZE]);
        let written = compress_block(block, &mut out, &mut table);
        out.truncate(written);
        out
    }

    fn round_trip(input: &[u8]) {
        let compressed = compress_block_to_vec(input);
        let mut decompressed = vec![0u8; input.len()];
        let _ = decompress_block(&compressed, &mut decompressed);
        assert_eq!(input, decompressed.as_slice());
    }

    #[test]
    fn compress_block_no_back_reference() {
        round_trip(b"AAAABBBBBBBBB");
    }

    #[test]
    fn compress_block_two_back_references() {
        round_trip(b"AAAAABBBBBAAAAACDEFG");
    }

    #[test]
    fn compress_block_multiple_references() {
        round_trip(b"ABCABCABCABCABCABCABCABCABCABC");
    }

    #[test]
    fn compress_block_long_run() {
        let mut input = vec![b'A'; 100];
        input.extend_from_slice(b"BCDE");
        round_trip(&input);
    }

    #[test]
    fn compress_block_short() {
        round_trip(b"ABCD");
    }
}
