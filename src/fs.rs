use core::cell::RefCell;
extern crate alloc;
use alloc::vec::Vec;

use crate::error::Error;
use crate::io::{self, ReadWriteSeek};

use crate::descriptor::BlockGroupDescriptor;
use crate::dir::Dir;
use crate::inode::Inode;
use crate::super_block::SuperBlock;

pub struct FileSystem<IO: ReadWriteSeek> {
  pub disk: RefCell<IO>,
  pub super_block: SuperBlock,
  pub block_group_descriptors: Vec<BlockGroupDescriptor>,
}

pub trait IntoStorage<T: ReadWriteSeek> {
  fn into_storage(self) -> T;
}

impl<T: ReadWriteSeek> IntoStorage<T> for T {
  fn into_storage(self) -> Self {
    self
  }
}

#[cfg(feature = "std")]
impl<T: std::io::Read + std::io::Write + std::io::Seek> IntoStorage<io::StdIoWrapper<T>> for T {
  fn into_storage(self) -> io::StdIoWrapper<Self> {
    io::StdIoWrapper::new(self)
  }
}

impl<IO: ReadWriteSeek> FileSystem<IO> {
  pub fn new<T: IntoStorage<IO>>(storage: T) -> Result<Self, Error<IO::Error>> {
    let mut disk = storage.into_storage();
    trace!("FileSystem::new");

    // read super block
    let super_block = SuperBlock::deserialize(&mut disk)?;
    trace!("super_block: {:?}", super_block);
    let mut descriptors = Vec::with_capacity(super_block.get_block_group_count() as usize);
    for _ in 0..super_block.get_block_group_count() {
      let bgd = BlockGroupDescriptor::deserialize(&mut disk)?;
      descriptors.push(bgd);
      trace!("block_group_descriptor: {:?}", bgd);
    }

    Ok(Self {
      disk: RefCell::new(disk),
      super_block,
      block_group_descriptors: descriptors,
    })
  }

  pub fn get_inode(&mut self, ino: u64) -> Result<Inode, Error<IO::Error>> {
    let bgd_num = (ino - 1) / self.super_block.inodes_per_group as u64;
    let bdg = &self.block_group_descriptors[bgd_num as usize];
    let inode_table_index = (ino - 1) % self.super_block.inodes_per_group as u64;

    let pos = bdg.get_inode_table_loc() * self.super_block.get_block_size()
      + inode_table_index * self.super_block.get_inode_size();

    let mut disk = self.disk.borrow_mut();
    disk.seek(io::SeekFrom::Start(pos))?;

    let inode = Inode::deserialize(&mut *disk)?;
    Ok(inode)
  }

  pub fn root_dir(&mut self) -> Dir<IO> {
    let inode = self.get_inode(Inode::ROOT_INO).unwrap();
    Dir::new(inode, self)
  }
}
