use crate::dir_entry::{DirEntry, DirEntryData, DirEntryFileType, DirEntryTail};
use crate::error::Error;
use crate::extent::Extent;
use crate::file::File;
use crate::fs::FileSystem;
use crate::inode::{Inode, InodeFilePerm, InodeFileType, InodeFlags};
use crate::io::{ReadWriteSeek, SeekFrom};
use crate::utils::split_path;

pub struct Dir<'a, IO: ReadWriteSeek> {
  pub ino: u64,
  pub inode: Inode,
  pub fs: &'a FileSystem<IO>,
}

pub struct DirIter<'a, IO: ReadWriteSeek> {
  pub dir_ino: u64,
  pub dir_inode: Inode,
  pub fs: &'a FileSystem<IO>,
  pub extent_idx: usize,
  pub extent_offset: u64,
  pub tail_entry: Option<DirEntryData>,
}

impl<'a, IO: ReadWriteSeek> Dir<'a, IO> {
  pub fn new(ino: u64, inode: Inode, fs: &'a FileSystem<IO>) -> Self {
    Self { ino, inode, fs }
  }

  pub fn iter(&self) -> DirIter<'a, IO> {
    DirIter {
      dir_ino: self.ino,
      dir_inode: self.inode,
      fs: self.fs,
      extent_idx: 0,
      extent_offset: 0,
      tail_entry: None,
    }
  }

  pub fn add_dir_entry_and_sync(
    &mut self,
    ino: u32,
    name: &str,
    file_type: Option<DirEntryFileType>,
  ) -> Result<(), Error<IO::Error>> {
    trace!(
      "Dir::add_dir_entry_and_sync ino: {}, name: {}, file_type: {:?}",
      ino,
      name,
      file_type
    );
    let mut new_entry = DirEntryData::new(
      ino,
      name,
      file_type,
      self.fs.super_block.borrow().has_feature_incompat_filetype(),
    );
    trace!("Dir::add_dir_entry_and_sync new_entry: {:?}", new_entry);

    // 找到最后一个entry对应的extent
    let extents = {
      let mut disk = self.fs.disk.borrow_mut();
      self.inode.get_extents(&mut *disk)?
    };
    let (extent_idx, mut extent_offset, mut entries) = {
      let mut iter = self.iter();
      let mut entries = Vec::new();
      for entry in &mut iter {
        entries.push(entry.unwrap().data);
      }
      let mut extent_offset = iter.extent_offset;
      let mut extent_idx = iter.extent_idx;
      let last_entry = entries.last().unwrap();
      if extent_offset == 0 {
        extent_idx -= 1;
        extent_offset = self.fs.super_block.borrow().get_block_size() * extents[extent_idx].len as u64;
      }
      extent_offset -= last_entry.get_rec_len() as u64;

      // 检查checksum
      let tail_entry = iter.tail_entry.unwrap();
      let csum = tail_entry.get_checksum();
      let cmp_csum = DirEntryData::compute_dirblock_checksum(
        &entries,
        self.fs.super_block.borrow().get_block_size(),
        &self.fs.super_block.borrow().uuid,
        self.ino as u32,
        self.inode.generation,
      );
      assert_eq!(csum, cmp_csum);
      (extent_idx, extent_offset, entries)
    };
    let last_entry_idx = entries.len() - 1;
    trace!("Dir::add_dir_entry_and_sync last_entry: {:?}", entries[last_entry_idx]);
    trace!(
      "Dir::add_dir_entry_and_sync extent_idx: {}, extent_offset: {}",
      extent_idx,
      extent_offset
    );
    // 假定只有一个extent，且这一个extent仅有一个物理块, 这样方便计算checksum
    // TODO: 对于有多个物理块的extent（也就是dir entry分布在多个物理块），dir entry tail checksum如何计算？
    assert!(extent_idx == 0);
    let extent = extents[extent_idx];
    assert!(extent.len == 1);

    let mut disk = self.fs.disk.borrow_mut();
    // TODO: 这里不考虑分配新的extent，所以last entry需要足够大
    let last_entry_real_len = entries[last_entry_idx].get_real_rec_len();
    assert!(last_entry_real_len + new_entry.get_rec_len() <= entries[last_entry_idx].get_rec_len());
    // 更新last entry的rec_len并写入
    let original_rec_len = entries[last_entry_idx].get_rec_len();
    entries[last_entry_idx].set_rec_len(last_entry_real_len);
    extent.write_entrydata(
      self.fs.super_block.borrow().get_block_size(),
      &mut *disk,
      extent_offset,
      &entries[last_entry_idx],
    )?;
    // 写入新的entry
    extent_offset += last_entry_real_len as u64;
    new_entry.set_rec_len(original_rec_len - last_entry_real_len);
    extent.write_entrydata(
      self.fs.super_block.borrow().get_block_size(),
      &mut *disk,
      extent_offset,
      &new_entry,
    )?;
    entries.push(new_entry);
    // 计算checksum并写入tail entry
    let csum = DirEntryData::compute_dirblock_checksum(
      &entries,
      self.fs.super_block.borrow().get_block_size(),
      &self.fs.super_block.borrow().uuid,
      self.ino as u32,
      self.inode.generation,
    );
    let tail_entry = DirEntryData::DirEntryTail(DirEntryTail {
      reserved_zero1: 0,
      rec_len: 12,
      reserved_zero2: 0,
      reserved_ft: 0xDE,
      checksum: csum,
    });
    extent.write_entrydata(
      self.fs.super_block.borrow().get_block_size(),
      &mut *disk,
      extent_offset + new_entry.get_rec_len() as u64,
      &tail_entry,
    )?;

    // 更新link count
    if let Some(file_type) = file_type {
      if file_type == DirEntryFileType::DIR {
        trace!("Dir::add_dir_entry_and_sync: increment parent dir link count if new entry is a dir");
        self.inode.links_count += 1;
        self.inode.compute_and_set_checksum(
          self.ino as u32,
          self.fs.super_block.borrow().get_inode_size() as u16,
          &self.fs.super_block.borrow().uuid,
        );
        let pos = self.fs.get_inode_pos(self.ino);
        disk.seek(SeekFrom::Start(pos))?;
        self.inode.serialize(&mut *disk)?;
      }
    }

    Ok(())
  }
}

impl<'a, IO: ReadWriteSeek> Iterator for DirIter<'a, IO> {
  type Item = Result<DirEntry<'a, IO>, Error<IO::Error>>;

  fn next(&mut self) -> Option<Self::Item> {
    let inode = self.dir_inode;
    assert!(inode.use_extents(), "only support extents");

    let mut disk = self.fs.disk.borrow_mut();
    // TODO: 每次next都要读所有的extents，可以优化
    let extents = inode.get_extents(&mut *disk).unwrap();
    if self.extent_idx >= extents.len() {
      return None;
    }
    let extent = extents[self.extent_idx];
    let entrydata = extent
      .read_entrydata(
        self.fs.super_block.borrow().get_block_size(),
        self.fs.super_block.borrow().has_feature_incompat_filetype(),
        &mut *disk,
        self.extent_offset,
      )
      .unwrap();
    match entrydata {
      Some(entrydata) => {
        if let DirEntryData::DirEntryTail(tail_entry) = entrydata {
          self.tail_entry = Some(DirEntryData::DirEntryTail(tail_entry));
          None
        } else {
          self.extent_offset += entrydata.get_rec_len() as u64;
          if self.extent_offset >= extent.len as u64 * self.fs.super_block.borrow().get_block_size() {
            self.extent_offset = 0;
            self.extent_idx += 1;
          }
          Some(Ok(DirEntry {
            data: entrydata,
            fs: self.fs,
          }))
        }
      }
      None => None,
    }
  }
}

// 对外提供的接口
impl<'a, IO: ReadWriteSeek> Dir<'a, IO> {
  pub fn find_entry(&self, name: &str) -> Result<DirEntry<'a, IO>, Error<IO::Error>> {
    trace!("Dir::find_entry name: {}", name);
    for r in self.iter() {
      let e = r?;
      if e.data.get_name_str() == name {
        return Ok(e);
      }
    }
    Err(Error::NotFound)
  }

  pub fn is_exist(&self, name: &str) -> bool {
    trace!("Dir::is_exist name: {}", name);
    for r in self.iter() {
      let e = r.unwrap();
      if e.data.get_name_str() == name {
        return true;
      }
    }
    false
  }

  pub fn open_dir(&self, path: &str) -> Result<Self, Error<IO::Error>> {
    trace!("Dir::open_dir path: {}", path);
    let (name, rest_opt) = split_path(path);
    let e = self.find_entry(name)?;
    let dir = e.to_dir();
    match rest_opt {
      Some(rest) => dir.open_dir(rest),
      None => Ok(dir),
    }
  }

  pub fn open_file(&self, path: &str) -> Result<File<'a, IO>, Error<IO::Error>> {
    trace!("Dir::open_file path: {}", path);
    let (name, rest_opt) = split_path(path);
    if let Some(rest) = rest_opt {
      let e = self.find_entry(name)?;
      return e.to_dir().open_file(rest);
    }
    let e = self.find_entry(name)?;
    Ok(e.to_file())
  }

  pub fn create_dir(
    &mut self,
    path: &str,
    uid: u16,
    gid: u16,
    file_perm: InodeFilePerm,
    time: u32,
  ) -> Result<Self, Error<IO::Error>> {
    trace!(
      "Dir::create_dir path: {}, uid: {}, gid: {:?}, file_perm: {:?}",
      path,
      uid,
      gid,
      file_perm
    );
    let (name, rest_opt) = split_path(path);
    // 所有父目录都存在
    if let Some(rest) = rest_opt {
      return self
        .find_entry(name)?
        .to_dir()
        .create_dir(rest, uid, gid, file_perm, time);
    }

    if self.is_exist(name) {
      return Err(Error::AlreadyExists);
    }

    let new_ino = self.fs.alloc_inode(true)?;
    let new_mode = (InodeFileType::DIR.bits() & Inode::FILETYPE_MASK) | (file_perm.bits() & Inode::FILEPERM_MASK);
    let new_flags = InodeFlags::EXTENTS_FL;
    let mut new_inode = Inode {
      uid,
      gid,
      mode: new_mode,
      atime: time,
      ctime: time,
      mtime: time,
      crtime: time,
      links_count: 2,
      osd1: 1, // TODO: 为什么
      blocks_lo: self.fs.super_block.borrow().get_block_size() as u32 / Inode::INODE_BLOCK_SIZE as u32,
      extra_isize: self.fs.super_block.borrow().want_extra_isize,
      flags: new_flags.bits(),
      ..Inode::default()
    };
    new_inode.set_size(self.fs.super_block.borrow().get_block_size() as u64);

    // 分配一个block作为新目录的extent
    let bgd_id = (new_ino - 1) / (self.fs.super_block.borrow().inodes_per_group as u64);
    let new_block_start = self.fs.alloc_contiguous_blocks(1, bgd_id as usize)?;
    let new_extent = Extent::new(0, 1, new_block_start);
    new_inode.init_extent_tree(vec![new_extent]);
    new_inode.compute_and_set_checksum(
      new_ino as u32,
      self.fs.super_block.borrow().get_inode_size() as u16,
      &self.fs.super_block.borrow().uuid,
    );
    // 写入新的inode
    trace!("Dir::create_dir: write new inode to disk");
    {
      let pos = self.fs.get_inode_pos(new_ino as u64);
      let mut disk = self.fs.disk.borrow_mut();
      disk.seek(SeekFrom::Start(pos))?;
      new_inode.serialize(&mut *disk)?;
    }

    // 在新目录的block里写入dir_entry(.., ., tail)
    trace!("Dir::create_dir: create new dir entries(.., ., tail)");
    let new_entries = DirEntryData::new_dir_entries(
      new_ino as u32,
      self.ino as u32,
      self.fs.super_block.borrow().has_feature_incompat_filetype(),
      self.fs.super_block.borrow().get_block_size(),
      &self.fs.super_block.borrow().uuid,
      self.inode.generation,
    );
    trace!(
      "Dir::create_dir: write new dir entries(.., ., tail) to disk: {:?}",
      new_entries
    );
    {
      let mut disk = self.fs.disk.borrow_mut();
      let mut offset = 0;
      for entry in new_entries.iter() {
        new_extent.write_entrydata(self.fs.super_block.borrow().get_block_size(), &mut *disk, offset, entry)?;
        offset += entry.get_rec_len() as u64;
      }
    }

    // 在当前目录里写入新的entry
    self.add_dir_entry_and_sync(new_ino as u32, name, Some(DirEntryFileType::DIR))?;

    return Ok(Dir::new(new_ino as u64, new_inode, self.fs));
  }

  pub fn create_file(
    &mut self,
    path: &str,
    uid: u16,
    gid: u16,
    file_perm: InodeFilePerm,
    time: u32,
  ) -> Result<File<'a, IO>, Error<IO::Error>> {
    trace!(
      "Dir::create_file path: {}, uid: {}, gid: {:?}, file_perm: {:?}",
      path,
      uid,
      gid,
      file_perm
    );
    let (name, rest_opt) = split_path(path);
    // 所有父目录都存在
    if let Some(rest) = rest_opt {
      return self
        .find_entry(name)?
        .to_dir()
        .create_file(rest, uid, gid, file_perm, time);
    }

    if self.is_exist(name) {
      return Err(Error::AlreadyExists);
    }

    let new_ino = self.fs.alloc_inode(true)?;
    let new_mode = (InodeFileType::REG.bits() & Inode::FILETYPE_MASK) | (file_perm.bits() & Inode::FILEPERM_MASK);
    let new_flags = InodeFlags::EXTENTS_FL;
    let mut new_inode = Inode {
      uid,
      gid,
      mode: new_mode,
      atime: time,
      ctime: time,
      mtime: time,
      crtime: time,
      links_count: 1,
      osd1: 1, // TODO: 为什么
      blocks_lo: self.fs.super_block.borrow().get_block_size() as u32 / Inode::INODE_BLOCK_SIZE as u32,
      extra_isize: self.fs.super_block.borrow().want_extra_isize,
      flags: new_flags.bits(),
      ..Inode::default()
    };
    new_inode.set_size(self.fs.super_block.borrow().get_block_size() as u64);

    // 分配一个block作为新目录的extent
    let bgd_id = (new_ino - 1) / (self.fs.super_block.borrow().inodes_per_group as u64);
    let new_block_start = self.fs.alloc_contiguous_blocks(1, bgd_id as usize)?;
    let new_extent = Extent::new(0, 1, new_block_start);
    new_inode.init_extent_tree(vec![new_extent]);
    new_inode.compute_and_set_checksum(
      new_ino as u32,
      self.fs.super_block.borrow().get_inode_size() as u16,
      &self.fs.super_block.borrow().uuid,
    );
    // 写入新的inode
    trace!("Dir::create_file: write new inode to disk");
    {
      let pos = self.fs.get_inode_pos(new_ino as u64);
      let mut disk = self.fs.disk.borrow_mut();
      disk.seek(SeekFrom::Start(pos))?;
      new_inode.serialize(&mut *disk)?;
    }

    // 在当前目录里写入新的entry
    self.add_dir_entry_and_sync(new_ino as u32, name, Some(DirEntryFileType::REG_FILE))?;
    return Ok(File::new(new_ino as u64, new_inode, self.fs));
  }
}
