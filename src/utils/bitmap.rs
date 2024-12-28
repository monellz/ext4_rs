extern crate alloc;
use alloc::vec::Vec;

use crate::io::{Read, Write};

#[derive(Debug)]
pub struct Bitmap {
  pub data: Vec<u8>,
}

impl Bitmap {
  pub const BITS_PER_ITEM: usize = 8;

  pub fn deserialize<R: Read>(reader: &mut R, size: usize) -> Result<Self, R::Error> {
    let mut buffer = vec![0u8; size];
    reader.read_exact(&mut buffer)?;
    Ok(Self { data: buffer })
  }

  pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), W::Error> {
    writer.write_all(&self.data)
  }

  pub fn set_bit(&mut self, bit: u64) {
    let byte = bit / 8;
    let bit = bit % 8;
    self.data[byte as usize] |= 1 << bit;
  }

  pub fn set_bits(&mut self, start: u64, count: u64) {
    for i in 0..count {
      self.set_bit(start + i);
    }
  }

  pub fn clear_bit(&mut self, bit: u64) {
    let byte = bit / 8;
    let bit = bit % 8;
    self.data[byte as usize] &= !(1 << bit);
  }

  pub fn get_bit(&self, bit: u64) -> bool {
    let byte = bit / 8;
    let bit = bit % 8;
    (self.data[byte as usize] & (1 << bit)) != 0
  }

  pub fn find_unused_bit(&self) -> Option<u64> {
    for (byte_idx, byte) in self.data.iter().enumerate() {
      if *byte != 0xFF {
        for bit_idx in 0..8 {
          if (byte & (1 << bit_idx)) == 0 {
            return Some((byte_idx * 8 + bit_idx) as u64);
          }
        }
      }
    }
    None
  }

  pub fn find_consecutive_unused_bits(&self, count: u64) -> Option<u64> {
    let mut consecutive = 0;
    for (byte_idx, byte) in self.data.iter().enumerate() {
      if *byte != 0xFF {
        for bit_idx in 0..8 {
          if (byte & (1 << bit_idx)) == 0 {
            consecutive += 1;
            if consecutive == count {
              return Some((byte_idx * 8 + bit_idx - (count as usize) + 1) as u64);
            }
          } else {
            consecutive = 0;
          }
        }
      } else {
        consecutive = 0;
      }
    }
    None
  }

  pub fn size(&self) -> u64 {
    self.data.len() as u64 * 8
  }
}
