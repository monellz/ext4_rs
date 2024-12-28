use bitflags::bitflags;

extern crate alloc;
use crate::extent::{Extent, ExtentHeader};
use crate::io::{Read, Write};
use crate::utils::{combine_u64, crc::crc32c};
use alloc::vec::Vec;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Inode {
  pub mode: u16,         // 文件类型和访问权限
  pub uid: u16,          // 所有者ID
  pub size_lo: u32,      // 文件大小
  pub atime: u32,        // 最后访问时间
  pub ctime: u32,        // 创建时间
  pub mtime: u32,        // 最后修改时间
  pub dtime: u32,        // 删除时间
  pub gid: u16,          // 组ID
  pub links_count: u16,  // 链接数
  pub blocks_lo: u32,    // 块数
  pub flags: u32,        // 扩展属性标志
  pub osd1: u32,         // 操作系统相关
  pub block: [u32; 15],  // 数据块指针
  pub generation: u32,   // 文件版本
  pub file_acl_lo: u32,  // 文件ACL
  pub size_hi: u32,      // 文件大小高32位
  pub obso_faddr: u32,   // 文件碎片地址
  pub osd2: Linux2,      // 操作系统相关
  pub extra_isize: u16,  // 扩展inode大小
  pub checksum_hi: u16,  // inode校验和高16位
  pub ctime_extra: u32,  // 额外创建时间(高精度部分)
  pub mtime_extra: u32,  // 额外修改时间
  pub atime_extra: u32,  // 额外访问时间
  pub crtime: u32,       // 创建时间
  pub crtime_extra: u32, // 额外创建时间
  pub version_hi: u32,   // inode版本高32位
  pub projid: u32,       // 项目ID
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Linux2 {
  pub blocks_high: u16,   // 高 16 位已分配块数
  pub file_acl_high: u16, // 高 16 位文件 ACL
  pub uid_high: u16,      // 高 16 位用户 ID
  pub gid_high: u16,      // 高 16 位组 ID
  pub checksum_lo: u16,   // 低位校验和
  pub reserved: u16,      // 保留字段
}

bitflags! {
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub struct InodeFilePerm: u16 {
    const IXOTH = 0x1; // 其他用户可执行
    const IWOTH = 0x2; // 其他用户可写
    const IROTH = 0x4; // 其他用户可读
    const IXGRP = 0x8; // 组用户可执行
    const IWGRP = 0x10; // 组用户可写
    const IRGRP = 0x20; // 组用户可读
    const IXUSR = 0x40; // 所有者可执行
    const IWUSR = 0x80; // 所有者可写
    const IRUSR = 0x100; // 所有者可读
    const ISVTX = 0x200; // 粘着位
    const ISGID = 0x400; // 组ID位
    const ISUID = 0x800; // 用户ID位
  }

  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub struct InodeFileType: u16 {
    const FIFO = 0x1000; // 管道
    const CHR = 0x2000; // 字符设备
    const DIR = 0x4000; // 目录
    const BLK = 0x6000; // 块设备
    const REG = 0x8000; // 普通文件
    const LNK = 0xA000; // 符号链接
    const SOCK = 0xC000; // 套接字
  }

  #[derive(Debug, Copy, Clone)]
  pub struct InodeFlags: u32 {
    const SECRM_FL = 0x1; // 安全删除
    const UNRM_FL = 0x2; // 不可删除
    const COMPR_FL = 0x4; // 压缩文件
    const SYNC_FL = 0x8; // 同步更新
    const IMMUTABLE_FL = 0x10; // 不可修改
    const APPEND_FL = 0x20; // 只能追加
    const NODUMP_FL = 0x40; // 不备份
    const NOATIME_FL = 0x80; // 不更新访问时间
    const DIRTY_FL = 0x100; // 已修改
    const COMPRBLK_FL = 0x200; // 块压缩
    const NOCOMPR_FL = 0x400; // 不压缩
    const ENCRYPT_FL = 0x800; // 加密
    const INDEX_FL = 0x1000; // hash索引目录
    const IMAGIC_FL = 0x2000; // AFS目录
    const JOURNAL_DATA_FL = 0x4000; // 日志数据
    const NOTAIL_FL = 0x8000; // 不追加
    const DIRSYNC_FL = 0x10000; // 目录同步
    const TOPDIR_FL = 0x20000; // 顶层目录
    const HUGE_FILE_FL = 0x40000; // 大文件
    const EXTENTS_FL = 0x80000; // inode使用extents
    const VERITY_FL = 0x100000; // verity文件
    const EA_INODE_FL = 0x200000; // 用于large EA的inode
    const DAX_FL = 0x2000000; // 直接访问
    const INLINE_DATA_FL = 0x10000000; // inode有inline data
    const PROJINHERIT_FL = 0x20000000; // create with parents projid
    const CASEFOLD_FL = 0x40000000; // casefolded file
    const RESERVED = 0x80000000;
  }
}

impl InodeFilePerm {
  pub fn default_file_perm() -> InodeFilePerm {
    InodeFilePerm::IRUSR
      | InodeFilePerm::IWUSR
      | InodeFilePerm::IXUSR
      | InodeFilePerm::IRGRP
      | InodeFilePerm::IXGRP
      | InodeFilePerm::IROTH
      | InodeFilePerm::IXOTH
  }

  pub fn default_dir_perm() -> InodeFilePerm {
    InodeFilePerm::IRUSR | InodeFilePerm::IWUSR | InodeFilePerm::IRGRP | InodeFilePerm::IROTH
  }
}

// constants
impl Inode {
  // special inodes
  pub const BAD_INO: u64 = 1; // 错误inode
  pub const ROOT_INO: u64 = 2; // 根目录inode
  pub const USER_QUOTA_INO: u64 = 3; // 用户配额inode
  pub const GROUP_QUOTA_INO: u64 = 4; // 组配额inode
  pub const BOOT_LOADER_INO: u64 = 5; // 引导加载程序inode
  pub const UNDEL_DIR_INO: u64 = 6; // 未删除目录inode
  pub const RESIZE_INO: u64 = 7; // 保留inode
  pub const JOURNAL_INO: u64 = 8; // 日志inode

  // mode掩码
  pub const FILEPERM_MASK: u16 = 0x0FFF; // 权限掩码
  pub const FILETYPE_MASK: u16 = 0xF000; // 文件类型掩码

  // inode.block_lo/block_hi统计块数的时候用这个
  // FIXME: 为什么
  // FIXME: ref: https://github.com/yuoo655/ext4_rs/blob/7b601d2b5e110737cfccd1570235bd3218cc537e/src/ext4_defs/consts.rs
  pub const INODE_BLOCK_SIZE: usize = 512;
}

impl Inode {
  pub fn deserialize<R: Read>(reader: &mut R) -> Result<Self, R::Error> {
    let mut buffer = [0u8; core::mem::size_of::<Self>()];
    reader.read_exact(&mut buffer)?;
    let inode: Inode = unsafe {
      let ptr = buffer.as_ptr() as *const Self;
      ptr.read_unaligned()
    };
    Ok(inode)
  }

  pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), W::Error> {
    let self_bytes =
      unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Self>()) };
    writer.write_all(self_bytes)?;
    Ok(())
  }

  pub fn get_size(&self) -> u64 {
    combine_u64(self.size_lo, self.size_hi)
  }

  pub fn set_size(&mut self, size: u64) {
    self.size_lo = size as u32;
    self.size_hi = (size >> 32) as u32;
  }

  pub fn get_file_perm(&self) -> InodeFilePerm {
    InodeFilePerm::from_bits_truncate(self.mode & Inode::FILEPERM_MASK)
  }

  pub fn get_file_type(&self) -> InodeFileType {
    InodeFileType::from_bits_truncate(self.mode & Inode::FILETYPE_MASK)
  }

  pub fn get_flags(&self) -> InodeFlags {
    InodeFlags::from_bits_truncate(self.flags)
  }

  pub fn set_flags(&mut self, flags: InodeFlags) {
    self.flags = flags.bits();
  }

  pub fn is_dir(&self) -> bool {
    self.get_file_type() == InodeFileType::DIR
  }

  pub fn is_file(&self) -> bool {
    self.get_file_type() == InodeFileType::REG
  }

  pub fn is_symlink(&self) -> bool {
    self.get_file_type() == InodeFileType::LNK
  }

  pub fn use_extents(&self) -> bool {
    self.get_flags().contains(InodeFlags::EXTENTS_FL)
  }

  pub fn get_extents<R: Read>(&self, _reader: &mut R) -> Result<Vec<Extent>, R::Error> {
    let mut extents = Vec::new();
    assert!(self.use_extents());

    let mut root_node_offset = 0;
    let root_eh = {
      let buffer = unsafe { &mut *(self.block.as_ptr() as *mut [u8; 60]) };
      ExtentHeader::load_from_u8(&buffer[root_node_offset..])
    };
    root_node_offset += core::mem::size_of::<ExtentHeader>();
    if root_eh.is_leaf() {
      for _ in 0..root_eh.entries {
        let extent = {
          let buffer = unsafe { &mut *(self.block.as_ptr() as *mut [u8; 60]) };
          Extent::load_from_u8(&buffer[root_node_offset..])
        };
        root_node_offset += core::mem::size_of::<Extent>();
        extents.push(extent);
      }
    } else {
      unimplemented!();
    }

    extents.sort_by(|a, b| a.block.cmp(&b.block));
    Ok(extents)
  }

  pub fn init_extent_tree(&mut self, extents: Vec<Extent>) {
    trace!("Inode::init_extent_tree: extents: {:?}", extents);
    let header = self.block.as_mut_ptr() as *mut ExtentHeader;
    unsafe {
      (*header).set_magic();
      (*header).entries = 1;
      (*header).max = 4;
      (*header).depth = 0;
      (*header).generation = 0;
    }
    assert_eq!(extents.len(), 1);
    let extent = extents[0];
    unsafe {
      // let extent_ptr = self.block.as_mut_ptr().add(core::mem::size_of::<ExtentHeader>()) as *mut Extent;
      let block_ptr = self.block.as_mut_ptr() as *mut u8;
      let extent_ptr = block_ptr.add(core::mem::size_of::<ExtentHeader>()) as *mut Extent;
      (*extent_ptr).block = extent.block;
      (*extent_ptr).len = extent.len;
      (*extent_ptr).start_hi = extent.start_hi;
      (*extent_ptr).start_lo = extent.start_lo;
    }
  }

  pub fn get_checksum(&self) -> u32 {
    let mut csum = self.osd2.checksum_lo as u32;
    csum |= (self.checksum_hi as u32) << 16;
    csum
  }

  pub fn compute_checksum(&mut self, ino: u32, inode_size: u16, uuid: &[u8]) -> u32 {
    let original_checksum_lo = self.osd2.checksum_lo;
    let original_checksum_hi = self.checksum_hi;
    self.osd2.checksum_lo = 0;
    self.checksum_hi = 0;

    let mut csum = crc32c(!0, uuid, uuid.len() as u32);
    csum = crc32c(csum, &ino.to_le_bytes(), 4);
    csum = crc32c(csum, &self.generation.to_le_bytes(), 4);

    let mut inode_data = vec![0u8; inode_size as usize];
    unsafe {
      let inode_data_ptr = self as *const Inode as *const u8;
      core::ptr::copy_nonoverlapping(inode_data_ptr, inode_data.as_mut_ptr(), core::mem::size_of::<Inode>());
    }
    csum = crc32c(csum, &inode_data, inode_size as u32);

    self.osd2.checksum_lo = original_checksum_lo;
    self.checksum_hi = original_checksum_hi;
    csum
  }

  pub fn compute_and_set_checksum(&mut self, ino: u32, inode_size: u16, uuid: &[u8]) {
    // 计算checksum
    let csum = self.compute_checksum(ino, inode_size, uuid);
    // 设置checksum
    self.osd2.checksum_lo = (csum & 0xFFFF) as u16;
    // TODO: hard code
    if inode_size > 128 {
      self.checksum_hi = (csum >> 16) as u16;
    }
  }
}
