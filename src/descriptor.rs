use crate::io::Read;
use crate::utils::combine_u64;

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

  pub fn get_block_bitmap_loc(&self) -> u64 {
    combine_u64(self.block_bitmap_lo, self.block_bitmap_hi)
  }

  pub fn get_inode_bitmap_loc(&self) -> u64 {
    combine_u64(self.inode_bitmap_lo, self.inode_bitmap_hi)
  }

  pub fn get_inode_table_loc(&self) -> u64 {
    combine_u64(self.inode_table_lo, self.inode_table_hi)
  }
}
