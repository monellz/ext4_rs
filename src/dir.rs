use crate::dir_entry::DirEntry;
use crate::error::Error;
use crate::file::File;
use crate::fs::FileSystem;
use crate::inode::Inode;
use crate::io::ReadWriteSeek;
use crate::utils::split_path;

pub struct Dir<'a, IO: ReadWriteSeek> {
  pub inode: Inode,
  pub fs: &'a FileSystem<IO>,
}

pub struct DirIter<'a, IO: ReadWriteSeek> {
  pub dir_inode: Inode,
  pub fs: &'a FileSystem<IO>,
  pub extent_idx: usize,
  pub extent_offset: u64,
}

impl<'a, IO: ReadWriteSeek> Dir<'a, IO> {
  pub fn new(inode: Inode, fs: &'a FileSystem<IO>) -> Self {
    Self { inode, fs }
  }

  pub fn iter(&self) -> DirIter<'a, IO> {
    DirIter {
      dir_inode: self.inode,
      fs: self.fs,
      extent_idx: 0,
      extent_offset: 0,
    }
  }
}

impl<'a, IO: ReadWriteSeek> Iterator for DirIter<'a, IO> {
  type Item = Result<DirEntry<'a, IO>, Error<IO::Error>>;

  fn next(&mut self) -> Option<Self::Item> {
    let inode = self.dir_inode;
    assert!(inode.use_extents(), "only support extents");

    let mut disk = self.fs.disk.borrow_mut();
    let extents = inode.get_extents(&mut *disk).unwrap();
    if self.extent_idx >= extents.len() {
      return None;
    }
    let extent = extents[self.extent_idx];
    let entrydata = extent
      .read_entrydata(
        self.fs.super_block.get_block_size(),
        self.fs.super_block.has_feature_incompat_filetype(),
        &mut *disk,
        &mut self.extent_offset,
      )
      .unwrap();
    match entrydata {
      Some(entrydata) => {
        if extent.len as u64 >= self.extent_offset {
          self.extent_offset = 0;
          self.extent_idx += 1;
        }
        Some(Ok(DirEntry {
          data: entrydata,
          fs: self.fs,
        }))
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
}
