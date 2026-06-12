use crate::block::{Error, MIN_MATCHLENGTH, TOKEN_UPPERBOUND};

fn parse_token(token: u8) -> (usize, usize) {
    let length = (token >> 4) as usize;
    let matchlength = (token & 0x0F) as usize;

    (length, matchlength + 4)
}

pub fn decompress_block(block: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let mut block_pos = 0;
    let mut out_pos = 0;

    // all blocks start with a token
    let (mut length, mut matchlength) = parse_token(block[block_pos]);

    block_pos += 1;

    while block_pos < block.len() {
        // if length == 15 get next byte
        if length == TOKEN_UPPERBOUND {
            loop {
                let next_byte = block[block_pos];
                length += next_byte as usize;
                block_pos += 1;

                if next_byte != 255 {
                    break;
                }
            }

        }

        // get literals after token
        out[out_pos..out_pos + length as usize].copy_from_slice(&block[block_pos..block_pos + length as usize]);

        out_pos += length as usize;
        block_pos += length as usize;

        // reached end of block - last literals, no offset
        if block_pos == block.len() {
            break;
        }

        // get offset
        let offset = u16::from_le_bytes([block[block_pos], block[block_pos + 1]]);

        if offset == 0 {
            return Err(Error::OffsetZero);
        }

        block_pos += 2;

        // if matchlength == 19 get next byte
        if matchlength == TOKEN_UPPERBOUND + MIN_MATCHLENGTH {
            loop {
                let next_byte = block[block_pos];
                matchlength += next_byte as usize;
                block_pos += 1;

                if next_byte != 255 {
                    break;
                }
            }

        }

        let match_pos = out_pos - offset as usize;

        if offset as usize >= matchlength {
            out.copy_within(match_pos..match_pos + matchlength, out_pos);
        } else {
            for i in 0..matchlength {
                out[out_pos + i] = out[match_pos + i];
            }
        }
        out_pos += matchlength;

        // we have already moved block_pos onto the next token
        (length, matchlength) = parse_token(block[block_pos]);

        // move to the next byte after token
        block_pos += 1;
    }

    Ok(out_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_block_simple_no_back_references() {
        let block = vec![0x40, 0x41, 0x42, 0x43, 0x44];
        let expected = "ABCD";

        let mut buf = vec![0u8; 4];
        let _ = decompress_block(&block, &mut buf);

        assert_eq!(expected, str::from_utf8(&buf).unwrap().to_string());
    }

    #[test]
    fn decompress_block_simple_back_reference() {
        let block = vec![0x40, 0x41, 0x41, 0x42, 0x42, 0x04, 0x00, 0x00];
        let expected = "AABBAABB";

        let mut buf = vec![0u8; 8];
        let _ = decompress_block(&block, &mut buf);

        assert_eq!(expected, str::from_utf8(&buf).unwrap().to_string());
    }

    #[test]
    fn decompress_block_self_references() {
        let block = vec![0x15, 0x41, 0x01, 0x00, 0x00];
        let expected = "AAAAAAAAAA";

        let mut buf = vec![0u8; 10];
        let _ = decompress_block(&block, &mut buf);

        assert_eq!(expected, str::from_utf8(&buf).unwrap().to_string());
    }

    #[test]
    fn decompress_block_variable_lenght_literal_count() {
        let block = vec![
            0xF0, 0x05,                   
            0x41, 0x41, 0x41, 0x41, 0x41,    
            0x41, 0x41, 0x41, 0x41, 0x41,
            0x41, 0x41, 0x41, 0x41, 0x41,
            0x41, 0x41, 0x41, 0x41, 0x41,
            0x01, 0x00,                 
            0x00,                      
        ];
        let expected = "AAAAAAAAAAAAAAAAAAAAAAAA";

        let mut buf = vec![0u8; 24];
        let _ = decompress_block(&block, &mut buf);

        assert_eq!(expected, str::from_utf8(&buf).unwrap().to_string());
    }

    #[test]
    fn decompress_block_multiple_references() {
        let block = vec![0x32, 0x41, 0x42, 0x43, 0x03, 0x00, 0x00];
        let expected = "ABCABCABC";

        let mut buf = vec![0u8; 9];
        let _ = decompress_block(&block, &mut buf);

        assert_eq!(expected, str::from_utf8(&buf).unwrap().to_string());
    }

    #[test]
    fn decompress_block_multiple_sequences() {
        let block = vec![0x10, 0x41, 0x01, 0x00, 0x10, 0x42, 0x01, 0x00, 0x00];
        let expected = "AAAAABBBBB";

        let mut buf = vec![0u8; 10];
        let _ = decompress_block(&block, &mut buf);

        assert_eq!(expected, str::from_utf8(&buf).unwrap().to_string());
    }

    #[test]
    fn decompress_block_multiple_sequences_strict_spec() {
        let block = vec![
            0x40,        
            0x41, 0x42, 0x43, 0x44, 
            0x04, 0x00,            

            0x24,                 
            0x45, 0x46,          
            0x0A, 0x00,         

            0x70,              
            0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D
        ];
        let expected = "ABCDABCDEFABCDABCDGHIJKLM";

        let mut buf = vec![0u8; 25];
        let _ = decompress_block(&block, &mut buf);

        assert_eq!(expected, str::from_utf8(&buf).unwrap().to_string());
    }
}
