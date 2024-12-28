use bitflags::bitflags;

use crate::io::{Read, Write};
use crate::super_block::SuperBlock;
use crate::utils::{combine_u32, combine_u64, crc::crc32c};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BlockGroupDescriptor {
  block_bitmap_lo: u32,
  inode_bitmap_lo: u32,
  inode_table_lo: u32,
  free_blocks_count_lo: u16,
  free_inodes_count_lo: u16,
  used_dirs_count_lo: u16,
  flags: u16,
  exclude_bitmap_lo: u32,
  block_bitmap_csum_lo: u16,
  inode_bitmap_csum_lo: u16,
  itable_unused_lo: u16,
  checksum: u16,

  // 64位支持
  block_bitmap_hi: u32,
  inode_bitmap_hi: u32,
  inode_table_hi: u32,
  free_blocks_count_hi: u16,
  free_inodes_count_hi: u16,
  used_dirs_count_hi: u16,
  itable_unused_hi: u16,
  exclude_bitmap_hi: u32,
  block_bitmap_csum_hi: u16,
  inode_bitmap_csum_hi: u16,
  reserved: u32,
}

bitflags! {
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub struct BGFlags: u16 {
    const INODE_UNINIT = 0x0001; // inode table/bitmap are not in use
    const BLOCK_UNINIT = 0x0002; // block bitmap not in use
    const INODE_ZEROED = 0x0004; // on-disk itable initizliaed to zero
  }
}

impl BlockGroupDescriptor {
  pub fn deserialize<R: Read>(reader: &mut R) -> Result<Self, R::Error> {
    let mut buffer = [0u8; core::mem::size_of::<Self>()];
    reader.read_exact(&mut buffer)?;
    let bgd: BlockGroupDescriptor = unsafe {
      let ptr = buffer.as_ptr() as *const Self;
      ptr.read_unaligned()
    };
    Ok(bgd)
  }

  pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), W::Error> {
    let self_bytes =
      unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Self>()) };
    writer.write_all(self_bytes)?;
    Ok(())
  }

  pub fn get_block_bitmap_loc(&self) -> u64 {
    combine_u64(self.block_bitmap_lo, self.block_bitmap_hi)
  }

  pub fn get_inode_bitmap_loc(&self) -> u64 {
    combine_u64(self.inode_bitmap_lo, self.inode_bitmap_hi)
  }

  pub fn get_inode_table_loc(&self) -> u64 {
    combine_u64(self.inode_table_lo, self.inode_table_hi)
  }

  pub fn get_flags(&self) -> BGFlags {
    BGFlags::from_bits_truncate(self.flags)
  }

  pub fn get_free_inodes_count(&self) -> u32 {
    combine_u32(self.free_inodes_count_lo as u16, self.free_inodes_count_hi as u16)
  }

  pub fn get_free_blocks_count(&self) -> u32 {
    combine_u32(self.free_blocks_count_lo as u16, self.free_blocks_count_hi as u16)
  }

  pub fn get_used_dirs_count(&self) -> u32 {
    combine_u32(self.used_dirs_count_lo as u16, self.used_dirs_count_hi as u16)
  }

  pub fn get_itable_unused(&self) -> u32 {
    combine_u32(self.itable_unused_lo as u16, self.itable_unused_hi as u16)
  }

  pub fn set_inode_bitmap_csum(&mut self, super_block: &SuperBlock, bitmap: &[u8]) {
    if !super_block.has_feature_ro_compat_metadata_csum() {
      return;
    }
    let inodes_per_group = super_block.inodes_per_group;
    let uuid = super_block.uuid;
    let mut csum = crc32c(!0 as u32, &uuid, uuid.len() as u32);
    csum = crc32c(csum, bitmap, (inodes_per_group + 7) / 8);

    let csum_lo = (csum & 0xFFFF).to_le();
    let csum_hi = (csum >> 16).to_le();
    self.inode_bitmap_csum_lo = csum_lo as u16;
    self.inode_bitmap_csum_hi = csum_hi as u16;
  }

  pub fn set_block_bitmap_csum(&mut self, super_block: &SuperBlock, bitmap: &[u8]) {
    if !super_block.has_feature_ro_compat_metadata_csum() {
      return;
    }
    let blocks_per_group = super_block.blocks_per_group;
    let uuid = super_block.uuid;
    let mut csum = crc32c(!0 as u32, &uuid, uuid.len() as u32);
    csum = crc32c(csum, bitmap, (blocks_per_group / 8) as u32);

    let csum_lo = (csum & 0xFFFF).to_le();
    let csum_hi = (csum >> 16).to_le();
    self.block_bitmap_csum_lo = csum_lo as u16;
    self.block_bitmap_csum_hi = csum_hi as u16;
  }

  pub fn set_free_inodes_count(&mut self, count: u32) {
    self.free_inodes_count_lo = (count & 0xFFFF) as u16;
    self.free_inodes_count_hi = (count >> 16) as u16;
  }

  pub fn set_free_blocks_count(&mut self, count: u32) {
    self.free_blocks_count_lo = (count & 0xFFFF) as u16;
    self.free_blocks_count_hi = (count >> 16) as u16;
  }

  pub fn set_used_dirs_count(&mut self, count: u32) {
    self.used_dirs_count_lo = (count & 0xFFFF) as u16;
    self.used_dirs_count_hi = (count >> 16) as u16;
  }

  pub fn set_itable_unused(&mut self, count: u32) {
    self.itable_unused_lo = (count & 0xFFFF) as u16;
    self.itable_unused_hi = (count >> 16) as u16;
  }

  pub fn compute_checksum(&mut self, bgd_id: u32, super_block: &SuperBlock) -> u16 {
    let original_csum = self.checksum;
    self.checksum = 0;
    // 计算
    let mut csum = crc32c(!0, &super_block.uuid, super_block.uuid.len() as u32);
    csum = crc32c(csum, &bgd_id.to_le_bytes(), 4);
    let self_bytes =
      unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Self>()) };
    assert_eq!(self_bytes.len(), super_block.get_desc_size() as usize);
    csum = crc32c(csum, self_bytes, self_bytes.len() as u32);

    self.checksum = original_csum;
    let csum = (csum & 0xFFFF) as u16;
    csum
  }

  pub fn set_checksum(&mut self, bgd_id: u32, super_block: &SuperBlock) {
    let csum = self.compute_checksum(bgd_id, super_block);
    self.checksum = csum;
  }
}
