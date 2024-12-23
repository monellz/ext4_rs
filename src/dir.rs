use crate::inode::Inode;
use crate::fs::FileSystem;
use crate::io::ReadWriteSeek;

pub struct Dir<'a, IO: ReadWriteSeek> {
  pub inode: Inode,
  pub fs: &'a FileSystem<IO>,
}

impl <'a, IO: ReadWriteSeek> Dir<'a, IO> {
  pub fn new(inode: Inode, fs: &'a FileSystem<IO>) -> Self {
    Self { inode, fs }
  }
}
