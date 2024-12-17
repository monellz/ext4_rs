use core::cell::RefCell;

use crate::error::Error;
use crate::io::{self, Read, Seek, Write};

pub trait ReadWriteSeek: Read + Write + Seek {}
impl<T: Read + Write + Seek> ReadWriteSeek for T {}

use crate::super_block::SuperBlock;

pub struct FileSystem<IO: ReadWriteSeek> {
  pub disk: RefCell<IO>,
  pub super_block: SuperBlock,
}

pub trait IntoStorage<T: Read + Write + Seek> {
  fn into_storage(self) -> T;
}

impl<T: Read + Write + Seek> IntoStorage<T> for T {
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

    Ok(Self {
      disk: RefCell::new(disk),
      super_block,
    })
  }
}
