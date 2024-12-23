use crate::io::Read;
use crate::utils::combine_u64;

// 12 bytes
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ExtentHeader {
  magic: u16,      // 魔数 0xF30A
  entries: u16,    // header后的合法条目数
  max: u16,        // header后的最大条目数
  depth: u16,      // 深度
  generation: u32, // 代数
}

impl ExtentHeader {
  pub fn deserialize<R: Read>(reader: &mut R) -> Result<Self, R::Error> {
    let mut buffer = [0u8; core::mem::size_of::<Self>()];
    reader.read_exact(&mut buffer)?;
    let header: ExtentHeader = unsafe {
      let ptr = buffer.as_ptr() as *const Self;
      ptr.read_unaligned()
    };
    Ok(header)
  }
}

// 12 bytes
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ExtentIdx {
  block: u32,   // 逻辑块号
  leaf_lo: u32, // 叶子节点低32位
  leaf_hi: u16, // 叶子节点高16位
  unused: u16,  // 未使用
}

impl ExtentIdx {
  pub fn get_extent_idx(&self) -> u64 {
    combine_u64(self.leaf_lo, self.leaf_hi as u32)
  }
}

// 12 bytes
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Extent {
  block: u32,    // 逻辑块号
  len: u16,      // 逻辑块数
  start_hi: u16, // 逻辑块号高16位
  start_lo: u32, // 逻辑块号低32位
}

impl Extent {
  pub fn get_block_loc(&self) -> u64 {
    combine_u64(self.start_lo, self.start_hi as u32)
  }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ExtentTail {
  checksum: u32, // 校验和
}
