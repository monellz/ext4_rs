use crate::dir_entry::DirEntryData;
use crate::io::{Read, Seek, SeekFrom, Write};
use crate::utils::combine_u64;

// 12 bytes
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ExtentHeader {
  pub magic: u16,      // 魔数 0xF30A
  pub entries: u16,    // header后的合法条目数
  pub max: u16,        // header后的最大条目数
  pub depth: u16,      // 深度
  pub generation: u32, // 代数
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

  pub fn load_from_u8(data: &[u8]) -> Self {
    unsafe { core::ptr::read(data.as_ptr() as *const _) }
  }

  pub fn load_from_u8_mut(data: &mut [u8]) -> &mut Self {
    unsafe { &mut *(data.as_mut_ptr() as *mut _) }
  }

  pub fn load_from_u32(data: &[u32]) -> Self {
    unsafe { core::ptr::read(data.as_ptr() as *const _) }
  }

  pub fn load_from_u32_mut(data: &mut [u32]) -> &mut Self {
    unsafe { &mut *(data.as_mut_ptr() as *mut _) }
  }

  pub fn is_leaf(&self) -> bool {
    self.depth == 0
  }

  pub fn set_magic(&mut self) {
    self.magic = 0xF30A;
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
  pub fn load_from_u8(data: &[u8]) -> Self {
    unsafe { core::ptr::read(data.as_ptr() as *const _) }
  }

  pub fn load_from_u8_mut(data: &mut [u8]) -> &mut Self {
    unsafe { &mut *(data.as_mut_ptr() as *mut _) }
  }

  pub fn load_from_u32(data: &[u32]) -> Self {
    unsafe { core::ptr::read(data.as_ptr() as *const _) }
  }

  pub fn load_from_u32_mut(data: &mut [u32]) -> &mut Self {
    unsafe { &mut *(data.as_mut_ptr() as *mut _) }
  }
}

// 12 bytes
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Extent {
  pub block: u32,    // 逻辑块号
  pub len: u16,      // 逻辑块数
  pub start_hi: u16, // 物理块号高16位
  pub start_lo: u32, // 物理块号低32位
}

impl Extent {
  pub fn new(block: u32, len: u16, start: u64) -> Self {
    Self {
      block,
      len,
      start_hi: (start >> 32 & 0xFFFF) as u16,
      start_lo: (start & 0xFFFFFFFF) as u32,
    }
  }
  pub fn get_block_loc(&self) -> u64 {
    combine_u64(self.start_lo, self.start_hi as u32)
  }
  pub fn load_from_u8(data: &[u8]) -> Self {
    unsafe { core::ptr::read(data.as_ptr() as *const _) }
  }
  pub fn load_from_u8_mut(data: &mut [u8]) -> &mut Self {
    unsafe { &mut *(data.as_mut_ptr() as *mut _) }
  }
  pub fn load_from_u32(data: &[u32]) -> Self {
    unsafe { core::ptr::read(data.as_ptr() as *const _) }
  }
  pub fn load_from_u32_mut(data: &mut [u32]) -> &mut Self {
    unsafe { &mut *(data.as_mut_ptr() as *mut _) }
  }

  pub fn read_entrydata<R: Read + Seek>(
    &self,
    block_size: u64,
    feature_incompat_filetype: bool,
    reader: &mut R,
    offset: u64,
  ) -> Result<Option<DirEntryData>, R::Error> {
    let pos = self.get_block_loc() * block_size;
    let size = self.len as u64 * block_size;
    assert!(size >= offset);
    reader.seek(SeekFrom::Start(pos + offset))?;
    // FIXME: 是否可能会出现一个entry跨越两个extent的情况？
    let max_size = size - offset;
    let dir_entry_data = DirEntryData::deserialize(reader, feature_incompat_filetype, max_size as usize).unwrap();
    return Ok(Some(dir_entry_data));
  }

  pub fn read_bytes<R: Read + Seek>(
    &self,
    block_size: u64,
    reader: &mut R,
    offset: u64,
    buf: &mut [u8],
  ) -> Result<(), R::Error> {
    let pos = self.get_block_loc() * block_size;
    reader.seek(SeekFrom::Start(pos + offset))?;
    reader.read_exact(buf)?;
    Ok(())
  }

  pub fn write_entrydata<W: Write + Seek>(
    &self,
    block_size: u64,
    writer: &mut W,
    offset: u64,
    data: &DirEntryData,
  ) -> Result<(), W::Error> {
    let pos = self.get_block_loc() * block_size;
    writer.seek(SeekFrom::Start(pos + offset))?;
    data.serialize(writer)?;
    Ok(())
  }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ExtentTail {
  checksum: u32, // 校验和
}
