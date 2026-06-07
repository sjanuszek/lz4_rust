use crate::block::{MIN_MATCHLENGTH, TOKEN_UPPERBOUND};

// 64kB as to fit into offset bytes
const WINDOW_SIZE: usize = 64 * 1024;
const LAST_SEQUENCE_LENGTH: usize = 5;
const PENULTIMATE_SEQUENCE_LENGTH: usize = 12;

fn hash_4(sequence: u32) -> u32 {
    (sequence.wrapping_mul(2654435761u32)) >> 16
}

fn write_extra(mut extra: usize, out: &mut [u8], out_i: &mut usize) {
    while extra >= 255 {
        out[*out_i] = 255;
        *out_i += 1;
        extra -= 255
    } 

    out[*out_i] = extra as u8;
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

    out[*out_i..*out_i + literals_count].copy_from_slice(&block[start_pos..start_pos + literals_count]);
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

pub fn compress_block(block: &[u8], out: &mut [u8], table: &mut [u32; WINDOW_SIZE]) -> usize {
    let mut i: usize = 0;
    let mut out_i: usize = 0;

    let mut lit_start_pos: usize = 0;

    // ensure we meet the end of block conditions
    while i < block.len().saturating_sub(PENULTIMATE_SEQUENCE_LENGTH) {
        let byte_sequence = u32::from_le_bytes(block[i..i+MIN_MATCHLENGTH].try_into().unwrap());
        let hash = hash_4(byte_sequence) as usize;
        let j = table[hash] as usize;

        let is_valid_match = j < i
            && i - j < WINDOW_SIZE
            && block[i..i+MIN_MATCHLENGTH] == block[j..j+MIN_MATCHLENGTH];

        table[hash] = i as u32;

        if is_valid_match {
            let mut matchlength = MIN_MATCHLENGTH;

            while i + matchlength < block.len() - LAST_SEQUENCE_LENGTH && j + matchlength < block.len() && block[i + matchlength] == block[j + matchlength] {
                matchlength += 1
            }

            let offset: u16 = (i - j).try_into().unwrap();

            write_sequence(block, out, &mut out_i, lit_start_pos, i - lit_start_pos, offset, matchlength);

            i += matchlength;
            lit_start_pos = i;
        } else {
            i += 1;
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
        let mut table = Box::new([0u32; WINDOW_SIZE]);
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
