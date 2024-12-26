use crate::dir_entry::DirEntry;
use crate::error::Error;
use crate::fs::FileSystem;
use crate::inode::Inode;
use crate::io::ReadWriteSeek;

pub struct Dir<'a, IO: ReadWriteSeek> {
  pub inode: Inode,
  pub fs: &'a FileSystem<IO>,
}

pub struct DirIter<'a, IO: ReadWriteSeek> {
  pub dir: &'a Dir<'a, IO>,
  pub extent_idx: usize,
  pub extent_offset: u64,
}

impl<'a, IO: ReadWriteSeek> DirIter<'a, IO> {
  pub fn new(dir: &'a Dir<'a, IO>) -> Self {
    Self {
      dir,
      extent_idx: 0,
      extent_offset: 0,
    }
  }
}

impl<'a, IO: ReadWriteSeek> Dir<'a, IO> {
  pub fn new(inode: Inode, fs: &'a FileSystem<IO>) -> Self {
    Self { inode, fs }
  }

  pub fn iter(&'a self) -> DirIter<'a, IO> {
    DirIter::new(self)
  }

  pub fn find_entry(&'a self, name: &str) -> Result<DirEntry<'a, IO>, Error<IO::Error>> {
    for r in self.iter() {
      let e = r?;
      if e.data.get_name_str() == name {
        return Ok(e);
      }
    }
    Err(Error::NotFound)
  }
}

impl<'a, IO: ReadWriteSeek> Iterator for DirIter<'a, IO> {
  type Item = Result<DirEntry<'a, IO>, Error<IO::Error>>;

  fn next(&mut self) -> Option<Self::Item> {
    let inode = self.dir.inode;
    assert!(inode.use_extents(), "only support extents");

    let mut disk = self.dir.fs.disk.borrow_mut();
    let extents = inode.get_extents(&mut *disk).unwrap();
    if self.extent_idx >= extents.len() {
      return None;
    }
    let extent = extents[self.extent_idx];
    let entrydata = extent
      .read_entrydata(
        self.dir.fs.super_block.get_block_size(),
        self.dir.fs.super_block.has_feature_incompat_filetype(),
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
          fs: self.dir.fs,
        }))
      }
      None => None,
    }
  }
}
