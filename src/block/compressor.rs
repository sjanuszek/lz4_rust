use crate::block::{MIN_MATCHLENGTH, TOKEN_UPPERBOUND};

// 64kB as to fit into offset bytes
static WINDOW_SIZE: usize = 64 * 1024;
static LAST_SEQUENCE_LENGTH: usize = 5;
static PENULTIMATE_SEQUENCE_LENGTH: usize = 12;

struct Sequence {
    // right now we're just moving the literals
    // but could be a lifetime
    literals: Vec<u8>,
    matchlength: Option<usize>,
    offset: Option<u16>
}

impl Sequence {
    fn write_to_output(&self, out: &mut [u8], out_i: &mut usize) {
        let (token, extra_literals, extra_matchlength) = build_token(self.literals.len(), self.matchlength);
        out[*out_i] = token;
        *out_i += 1;

        if let Some(extra_literals) = extra_literals {
            extra_literals
                .iter()
                .for_each(|val| {
                    out[*out_i] = *val;
                    *out_i += 1;
                });
        }

        self.literals
            .iter()
            .for_each(|val| {
                out[*out_i] = *val;
                *out_i += 1;
            });

        if let Some(offset) = self.offset {
            let [low, high] = offset.to_le_bytes();

            out[*out_i] = low;
            out[*out_i + 1] = high;
            *out_i += 2;

            if let Some(extra_matchlength) = extra_matchlength{
                extra_matchlength
                    .iter()
                    .for_each(|val| {
                        out[*out_i] = *val;
                        *out_i += 1;
                    });
            }

        }
    }
}

// needs a refactor, probably should write directly into a buffer
fn build_token(literal_count: usize, matchlength: Option<usize>) -> (u8, Option<Vec<u8>>, Option<Vec<u8>>) {
    let mut extra_literals_res: Option<Vec<u8>> = None;
    let mut extra_matchlength_res: Option<Vec<u8>> = None;

    let lit_nibble: u8;
    let mat_nibble: u8;

    if literal_count >= TOKEN_UPPERBOUND{
        lit_nibble = TOKEN_UPPERBOUND as u8;

        let num_full_groups = (literal_count - TOKEN_UPPERBOUND) / 255;
        let mut extra_literals = vec![255; num_full_groups];
        let remainder = (literal_count - TOKEN_UPPERBOUND) - (num_full_groups * 255);

        extra_literals.push(remainder as u8);
        extra_literals_res = Some(extra_literals);
    } else {
        lit_nibble = literal_count as u8;
    }

    match matchlength {
        None => mat_nibble = 0,
        Some(ml) => {
            if ml - MIN_MATCHLENGTH >= TOKEN_UPPERBOUND {
                mat_nibble = TOKEN_UPPERBOUND as u8;

                let num_full_groups = (ml - MIN_MATCHLENGTH - TOKEN_UPPERBOUND) / 255;
                let mut extra_matchlength = vec![255; num_full_groups];
                let remainder = (ml - MIN_MATCHLENGTH - TOKEN_UPPERBOUND) - (num_full_groups * 255);

                extra_matchlength.push(remainder as u8);
                extra_matchlength_res = Some(extra_matchlength);
            } else {
                mat_nibble = (ml - MIN_MATCHLENGTH) as u8;
            }
        }
    }

    let token = (lit_nibble << 4) | (mat_nibble);

    (token, extra_literals_res, extra_matchlength_res)
}

fn hash_4(sequence: u32) -> u32 {
    (sequence.wrapping_mul(2654435761u32)) >> 16
}

pub fn compress_block(block: &[u8], out: &mut [u8]) -> usize {
    let mut i: usize = 0;
    let mut out_i: usize = 0;

    let mut table: Vec<usize> = vec![0; WINDOW_SIZE];
    let mut buf: Vec<u8> = Vec::new();

    // ensure we meet the end of block conditions
    while i < block.len().saturating_sub(PENULTIMATE_SEQUENCE_LENGTH) {
        let byte_sequence = u32::from_le_bytes(block[i..i+MIN_MATCHLENGTH].try_into().unwrap());
        let hash = hash_4(byte_sequence) as usize;
        let j = table[hash];

        let is_valid_match = j < i
            && i - j < WINDOW_SIZE
            && block[i..i+MIN_MATCHLENGTH] == block[j..j+MIN_MATCHLENGTH];

        table[hash] = i;

        if is_valid_match {
            let mut matchlength = MIN_MATCHLENGTH;

            while i + matchlength < block.len() - LAST_SEQUENCE_LENGTH && j + matchlength < block.len() && block[i + matchlength] == block[j + matchlength] {
                matchlength += 1
            }

            let offset: u16 = (i - j).try_into().unwrap();
            let match_sequence = Sequence{literals: std::mem::take(&mut buf), matchlength: Some(matchlength), offset: Some(offset)};
            match_sequence.write_to_output(out, &mut out_i);

            i += matchlength;
            buf = Vec::new()
        } else {
            buf.push(block[i]);
            i += 1;
        }
        
    }

    // last sequence is pure literals
    buf.extend_from_slice(&block[i..]);
    let block_sequence = Sequence {literals: buf.to_vec(), matchlength: None, offset: None};
    block_sequence.write_to_output(out, &mut out_i);

    out_i
}

#[cfg(test)]
mod tests {
    use crate::block::decompressor::decompress_block;
    use super::*;

    fn compress_block_to_vec(block: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; block.len() * 2];
        let written = compress_block(block, &mut out);
        out.truncate(written);
        out
    }

    fn round_trip(input: &[u8]) {
        let compressed = compress_block_to_vec(input);
        let mut decompressed = vec![0u8; input.len()];
        decompress_block(&compressed, &mut decompressed);
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
