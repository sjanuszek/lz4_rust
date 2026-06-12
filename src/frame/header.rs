use std::{io::{BufReader, Read}};

use twox_hash::XxHash32;

use crate::frame::Error;

const MAGIC_NUMBER: u32 = 0x184D2204;

const VERSION_MASK: u8 = 0b11000000;
const SUPPORTED_VERSION_MASK: u8 = 0b01000000;

const BLOCK_INDEPENDANCE_MASK: u8 = 0b00100000;
const BLOCK_CHECKSUM_MASK: u8 = 0b00010000;
const CONTENT_SIZE_MASK: u8 = 0b00001000;
const CONTENT_CHECKSUM_MASK: u8 = 0b00000100;
const FLAG_RESERVED_MASK: u8 = 0b00000010;
const DICT_ID_MASK: u8 = 0b00000001;

const BD_RESERVED_MASK: u8 = 0b10001111;
const BLOCK_MAXIMUM_SIZE_MASK: u8 = 0b01110000;
const BLOCK_MAXIMUM_SIZE_BITSHIFT: u8 = 4;

const MINIMAL_FLAG: u8 = 0b01000000;
const SET_INDEPENDANCE: u8 = 0b00100000;
const SET_BLOCK_CHECKSUM: u8 = 0b00010000;
const SET_CONTENT_SIZE: u8 = 0b00001000;
const SET_CONTENT_CHECKSUM: u8 = 0b00000100;
const SET_DICTIONARY_ID: u8 = 0b00000001;

const MINIMAL_BD: u8 = 0b00000000;

#[derive(Debug)]
pub enum MaximumSize {
    KB64,
    KB256,
    MB1,
    MB4
}

impl MaximumSize {
    pub fn get_bitmask(&self) -> u8 {
        match self {
            Self::KB64 => 0b01000000,
            Self::KB256 => 0b01010000,
            Self::MB1 => 0b01100000,
            Self::MB4 => 0b01110000,
        }
    }
    pub fn get_bytes(&self) -> usize {
        match self {
            Self::KB64 => 64 * 1024,
            Self::KB256 => 256 * 1024,
            Self::MB1 => 1024 * 1024,
            Self::MB4 => 4 * 1024 * 1024,
        }
    }
} 

#[derive(Debug)]
pub struct FrameHeader {
    pub block_independance: bool,
    pub block_checksum: bool,
    pub content_size: Option<u64>,
    pub content_checksum: bool,
    pub dictionary_id: Option<u32>,
    pub maximum_size: MaximumSize,
}

impl FrameHeader {
    pub fn write(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();

        out.extend_from_slice(&u32::to_le_bytes(MAGIC_NUMBER));

        let mut flag = MINIMAL_FLAG;

        if self.block_independance {
            flag |= SET_INDEPENDANCE;
        }

        if self.block_checksum {
            flag |= SET_BLOCK_CHECKSUM;
        }

        if let Some(_) = self.content_size {
            flag |= SET_CONTENT_SIZE;
        }

        if self.content_checksum {
            flag |= SET_CONTENT_CHECKSUM;
        }

        if let Some(_) = self.dictionary_id {
            flag |= SET_DICTIONARY_ID;
        }

        out.push(flag);

        let bd = MINIMAL_BD | self.maximum_size.get_bitmask();

        out.push(bd);

        if let Some(content_size) = self.content_size {
            out.extend_from_slice(&u64::to_le_bytes(content_size));
        }

        if let Some(dictionary_id) = self.dictionary_id {
            out.extend_from_slice(&u32::to_le_bytes(dictionary_id));
        }

        let hash = (XxHash32::oneshot(0, &out[4..]) >> 8) as u8;
        out.push(hash);

        out
    }

    pub fn read<R: Read>(input: &mut BufReader<R>) -> Result<FrameHeader, Error> {
        let magic_num = {
            let mut buf = [0u8; 4];
            input.read_exact(&mut buf)?;
            u32::from_le_bytes(buf)
        };

        // should handle legacy
        if magic_num != MAGIC_NUMBER {
            return Err(Error::WrongMagicNumber);
        };

        let [flg, bd] = {
            let mut buf = [0u8; 2];
            input.read_exact(&mut buf)?;
            buf
        };

        if flg & VERSION_MASK != SUPPORTED_VERSION_MASK {
            return Err(Error::WrongVersion(flg & VERSION_MASK));
        }

        if flg & FLAG_RESERVED_MASK != 0 {
            return Err(Error::FlagReservedBitsSet);
        }

        if bd & BD_RESERVED_MASK != 0 {
            return Err(Error::BDReseverBitsSet);
        }

        let block_independance = flg & BLOCK_INDEPENDANCE_MASK != 0;
        let block_checksum = flg & BLOCK_CHECKSUM_MASK != 0;
        let content_size_flag = flg & CONTENT_SIZE_MASK != 0;
        let content_checksum = flg & CONTENT_CHECKSUM_MASK != 0;
        let dictionary_id_flag = flg & DICT_ID_MASK != 0;

        let maximum_size = match (bd & BLOCK_MAXIMUM_SIZE_MASK) >> BLOCK_MAXIMUM_SIZE_BITSHIFT {
            i @ 0..=3 => return Err(Error::UndefinedBlockMaximumSize(i)),
            4 => MaximumSize::KB64,
            5 => MaximumSize::KB256,
            6 => MaximumSize::MB1,
            7 => MaximumSize::MB4,
            _ => unreachable!()
        };

        let mut to_hash = vec![flg, bd];

        let content_size = if content_size_flag {
            let mut buf = [0u8; 8];
            input.read_exact(&mut buf)?;
            to_hash.extend_from_slice(&buf);
            Some(u64::from_le_bytes(buf))
        } else {
            None
        };

        let dictionary_id = if dictionary_id_flag {
            let mut buf = [0u8; 4];
            input.read_exact(&mut buf)?;
            to_hash.extend_from_slice(&buf);
            Some(u32::from_le_bytes(buf))
        } else {
            None
        };

        let hc = {
            let mut buf = [0u8; 1];
            input.read_exact(&mut buf)?;
            buf[0]
        };

        let hash = (XxHash32::oneshot(0, &to_hash) >> 8) as u8;

        if hash != hc {
            return Err(Error::WrongHeaderChecksum);
        }

        Ok(FrameHeader {
            block_independance,
            block_checksum,
            content_size,
            content_checksum,
            dictionary_id,
            maximum_size
        })
    }
}
