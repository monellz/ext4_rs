use bitflags::bitflags;

use crate::io::{Read, Seek, SeekFrom, Write};
use crate::utils::{combine_u64, crc::crc32c};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SuperBlock {
  inodes_count: u32,         // 节点数
  blocks_count_lo: u32,      // 块数
  r_blocks_count_lo: u32,    // 保留块数
  free_blocks_count_lo: u32, // 空闲块数
  free_inodes_count: u32,    // 空闲节点数
  pub first_data_block: u32, // 第一个数据块
  log_block_size: u32,       // 块大小
  log_cluster_size: u32,     // 簇大小
  pub blocks_per_group: u32, // 每组块数
  clusters_per_group: u32,   // 每组簇数
  pub inodes_per_group: u32, // 每组节点数
  mtime: u32,                // 挂载时间
  wtime: u32,                // 写入时间
  mnt_count: u16,            // 挂载次数
  max_mnt_count: u16,        // 最大挂载次数
  magic: u16,                // 魔数, 0xEF53
  state: u16,                // 文件系统状态
  errors: u16,               // 检测到错误时的行为
  minor_rev_level: u16,      // 次版本号
  lastcheck: u32,            // 上次检查时间
  checkinterval: u32,        // 检查间隔
  creator_os: u32,           // 创建文件系统的操作系统
  rev_level: u32,            // 版本号
  def_resuid: u16,           // 保留块的默认uid
  def_resgid: u16,           // 保留块的默认gid

  // 仅适用于EXT4_DYNAMIC_REV超级块
  first_ino: u32,              // 第一个非保留inode
  inode_size: u16,             // inode结构大小
  block_group_nr: u16,         // 块组号
  feature_compat: u32,         // 兼容特性集
  feature_incompat: u32,       // 不兼容特性集
  feature_ro_compat: u32,      // 只读兼容特性集
  pub uuid: [u8; 16],          // 卷的128位UUID
  volume_name: [u8; 16],       // 卷名
  last_mounted: [u8; 64],      // 最后挂载点
  algorithm_usage_bitmap: u32, // 压缩算法

  // 性能提示
  // 只有当EXT4_FEATURE_COMPAT_DIR_PREALLOC特性被打开时，才进行目录预分配
  prealloc_blocks: u8,      // 预分配块数
  prealloc_dir_blocks: u8,  // 预分配目录块数
  reserved_gdt_blocks: u16, // 在线增长时魅族保留的描述符表块数

  // 如果EXT4_FEATURE_COMPAT_HAS_JOURNAL设置，支持日志
  journal_uuid: [u8; 16],  // 日志超级块的UUID
  journal_inum: u32,       // 日志文件的节点号
  journal_dev: u32,        // 日志设备
  last_orphan: u32,        // 待删除节点的头节点
  hash_seed: [u32; 4],     // HTREE hash种子
  def_hash_version: u8,    // 默认的散列版本
  jnl_backup_type: u8,     // 日志备份方法
  desc_size: u16,          // 描述符大小
  default_mount_opts: u32, // 默认挂载选项
  first_meta_bg: u32,      // 第一个元数据块组
  mkfs_time: u32,          // 创建文件系统时间
  jnl_blocks: [u32; 17],   // 日志节点的备份

  // 如果EXT4_FEATURE_COMPAT_64BIT设置，支持64位
  blocks_count_hi: u32,         // 块数
  r_blocks_count_hi: u32,       // 保留块数
  free_blocks_count_hi: u32,    // 空闲块数
  min_extra_isize: u16,         // 所有节点至少有#字节
  pub want_extra_isize: u16,    // 新节点应该保留#字节
  flags: u32,                   // 杂项标志
  raid_stride: u16,             // RAID步长
  mmp_interval: u16,            // MMP检查等待秒数
  mmp_block: u64,               // 多重挂载保护的块
  raid_stripe_width: u32,       // 所有数据磁盘上的块数 (N * 步长)
  log_groups_per_flex: u8,      // FLEX_BG组大小
  checksum_type: u8,            // 元数据校验用的算法
  reserved_pad: u16,            // 填充到下一个32bits
  kbytes_written: u64,          // 文件系统创建以来写入的KB数
  snapshot_inum: u32,           // 活动快照的节点号
  snapshot_id: u32,             // 活动快照的顺序ID
  snapshot_r_blocks_count: u64, // 为活动快照未来使用保留的块数
  snapshot_list: u32,           // 磁盘上快照链表的头节点
  error_count: u32,             // 文件系统创建以来的错误数
  first_error_time: u32,        // 第一个错误时间
  first_error_ino: u32,         // 第一个错误的节点号
  first_error_block: u64,       // 第一个错误的块号
  first_error_func: [u8; 32],   // 第一个错误的函数名
  first_error_line: u32,        // 第一个错误的行号
  last_error_time: u32,         // 最后一个错误时间
  last_error_ino: u32,          // 最后一个错误的节点号
  last_error_line: u32,         // 最后一个错误的行号
  last_error_block: u64,        // 最后一个错误的块号
  last_error_func: [u8; 32],    // 最后一个错误的函数名
  mount_opts: [u8; 64],         // 挂载选项字符
  usr_quota_inum: u32,          // 用于跟踪用户配额文件的节点号
  grp_quota_inum: u32,          // 用于跟踪组配额文件的节点号
  overhead_blocks: u32,         // 文件系统中的超额块/簇
  backup_bgs: [u32; 2],         // 有sparse_super2 SBs的组
  encrypt_algos: [u8; 4],       // 使用的加密算法
  encrypt_pw_salt: [u8; 16],    // 加密密码的盐
  lpf_ino: u32,                 // lost+found节点的位置
  prj_quota_inum: u32,          // 用于跟踪项目配额文件的节点号
  checksum_seed: u32,           // crc32c(uuid)校验种子
  wtime_hi: u8,                 // 写入时间高8位
  mtime_hi: u8,                 // 挂载时间高8位
  mkfs_time_hi: u8,             // 创建文件系统时间高8位
  lastcheck_hi: u8,             // 上次检查时间高8位
  first_error_time_hi: u8,      // 第一个错误时间高8位
  last_error_time_hi: u8,       // 最后一个错误时间高8位
  first_error_errcode: u8,      // 第一个错误的错误代码
  last_error_errcode: u8,       // 最后一个错误的错误代码
  encoding: u16,                // 文件名编码
  encoding_flags: u16,          // 文件名编码标志
  orphan_file_inum: u32,        // 用于跟踪孤儿节点的节点号
  reseved: [u32; 94],           // 保留
  checksum: u32,                // crc32c(superblock)校验和
}

bitflags! {
  #[derive(Debug, Copy, Clone)]
  pub struct FeatureCompat: u32 {
    const DIR_PREALLOC = 0x1; // 目录预分配
    const IMAGIC_INODES = 0x2; // IMAGIC inodes
    const HAS_JOURNAL = 0x4; // 日志
    const EXT_ATTR = 0x8; // 扩展属性
    const RESIZE_INODE = 0x10; // 调整inode大小
    const DIR_INDEX = 0x20; // 目录索引
    const SPARSE_SUPER2 = 0x200; // 稀疏超级块
    const FAST_COMMIT = 0x400; // 快速提交
    const STABLE_INODES = 0x800; // 稳定inode
    const ORPHAN_FILE = 0x1000; // 孤儿文件
  }

  #[derive(Debug, Copy, Clone)]
  pub struct FeatureIncompat: u32 {
    const COMPRESSION = 0x1; // 压缩
    const FILETYPE = 0x2; // 文件类型
    const RECOVER = 0x4; // 恢复
    const JOURNAL_DEV = 0x8; // 日志设备
    const META_BG = 0x10; // 元数据块组
    const EXTENTS = 0x40; // extents
    const _64BIT = 0x80; // 64位
    const MMP = 0x100; // MMP
    const FLEX_BG = 0x200; // FLEX_BG
    const EA_INODE = 0x400; // EA inode
    const DIRDATA = 0x1000; // 目录数据
    const CSUM_SEED = 0x2000; // 校验种子
    const LARGEDIR = 0x4000; // 大目录
    const INLINE_DATA = 0x8000; // 内联数据
    const ENCRYPT = 0x10000; // 加密
    const CASEFOLD = 0x20000; // 大小写折叠
  }

  #[derive(Debug, Copy, Clone)]
  pub struct FeatureROCompat: u32 {
    const SPARSE_SUPER = 0x1; // 稀疏超级块
    const LARGE_FILE = 0x2; // 大文件
    const BTREE_DIR = 0x4; // B树目录
    const HUGE_FILE = 0x8; // 巨大文件
    const GDT_CSUM = 0x10; // GDT校验和
    const DIR_NLINK = 0x20; // 目录链接计数
    const EXTRA_ISIZE = 0x40; // 额外inode大小
    const QUOTA = 0x100; // 配额
    const BIGALLOC = 0x200; // 大分配
    const METADATA_CSUM = 0x400; // 元数据校验和
    const READONLY = 0x1000; // 只读
    const PROJECT = 0x2000; // 项目
    const VERITY = 0x8000; // 完整性
    const ORPHAN_PRESENT = 0x10000; // 孤儿文件
  }
}

impl SuperBlock {
  pub fn get_feature_compat(&self) -> FeatureCompat {
    FeatureCompat::from_bits_truncate(self.feature_compat)
  }
  pub fn get_feature_incompat(&self) -> FeatureIncompat {
    FeatureIncompat::from_bits_truncate(self.feature_incompat)
  }
  pub fn get_feature_ro_compat(&self) -> FeatureROCompat {
    FeatureROCompat::from_bits_truncate(self.feature_ro_compat)
  }

  pub fn has_feature_incompat_filetype(&self) -> bool {
    self.get_feature_incompat().contains(FeatureIncompat::FILETYPE)
  }

  pub fn has_feature_ro_compat_metadata_csum(&self) -> bool {
    self.get_feature_ro_compat().contains(FeatureROCompat::METADATA_CSUM)
  }
}

impl SuperBlock {
  pub const PADDING_OFFSET: usize = 1024;
  pub fn deserialize<R: Read + Seek>(reader: &mut R) -> Result<Self, R::Error> {
    let mut buffer = [0u8; core::mem::size_of::<Self>()];
    reader.seek(SeekFrom::Start(Self::PADDING_OFFSET as u64))?;
    reader.read_exact(&mut buffer)?;
    let super_block: SuperBlock = unsafe {
      let ptr = buffer.as_ptr() as *const Self;
      ptr.read_unaligned()
    };
    Ok(super_block)
  }

  pub fn serialize<W: Write + Seek>(&self, writer: &mut W) -> Result<(), W::Error> {
    let self_bytes =
      unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Self>()) };
    writer.seek(SeekFrom::Start(Self::PADDING_OFFSET as u64))?;
    writer.write_all(self_bytes)?;
    Ok(())
  }

  pub fn get_block_size(&self) -> u64 {
    1024 << self.log_block_size
  }

  pub fn get_block_group_count(&self) -> u32 {
    let blocks_count = combine_u64(self.blocks_count_lo, self.blocks_count_hi);
    let blocks_per_group = self.blocks_per_group as u64;
    let block_group_count = (blocks_count + blocks_per_group - 1) / blocks_per_group;
    block_group_count as u32
  }

  pub fn get_inode_size(&self) -> u64 {
    self.inode_size as u64
  }

  pub fn get_desc_size(&self) -> u64 {
    self.desc_size as u64
  }

  pub fn get_free_inodes_count(&self) -> u32 {
    self.free_inodes_count
  }

  pub fn set_free_inodes_count(&mut self, count: u32) {
    self.free_inodes_count = count;
  }

  pub fn get_free_blocks_count(&self) -> u64 {
    combine_u64(self.free_blocks_count_lo, self.free_blocks_count_hi)
  }

  pub fn set_free_blocks_count(&mut self, count: u64) {
    self.free_blocks_count_lo = count as u32;
    self.free_blocks_count_hi = (count >> 32) as u32;
  }

  pub fn compute_checksum(&self) -> u32 {
    let data = unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Self>()) };
    // 除了checksum字段外的所有字段
    crc32c(!0, data, data.len() as u32 - core::mem::size_of::<u32>() as u32)
  }

  pub fn compute_and_set_checksum(&mut self) {
    self.checksum = self.compute_checksum();
  }

  pub fn get_checksum(&self) -> u32 {
    self.checksum
  }
}
