use crate::fs::FileSystem;
use crate::inode::Inode;
use crate::io::ReadWriteSeek;

pub struct File<'a, IO: ReadWriteSeek> {
  pub ino: u64,
  pub inode: Inode,
  pub fs: &'a FileSystem<IO>,
}

impl<'a, IO: ReadWriteSeek> File<'a, IO> {
  pub fn new(ino: u64, inode: Inode, fs: &'a FileSystem<IO>) -> Self {
    assert!(inode.is_file(), "only support file");
    Self { ino, inode, fs }
  }

  pub fn read(&self, mut offset: u64, mut buf: &mut [u8]) -> Result<usize, IO::Error> {
    trace!("File::read offset: {}, buf.len: {}", offset, buf.len());
    if offset >= self.inode.get_size() {
      return Ok(0);
    }
    let bytes_left_in_file = self.inode.get_size() - offset;
    let read_bytes = if bytes_left_in_file < buf.len() as u64 {
      bytes_left_in_file as usize
    } else {
      buf.len()
    };
    trace!("read_bytes: {}", read_bytes);
    if read_bytes == 0 {
      return Ok(0);
    }

    let mut disk = self.fs.disk.borrow_mut();
    let extents = self.inode.get_extents(&mut *disk).unwrap();
    let block_size = self.fs.super_block.borrow().get_block_size() as u64;

    let mut left_bytes = read_bytes;
    for extent in extents {
      if offset >= extent.len as u64 * block_size {
        offset -= extent.len as u64 * block_size;
        continue;
      }

      let read_bytes_in_extent =
        core::cmp::min(left_bytes as u64, (extent.len as u64 * block_size - offset) as u64) as usize;

      extent.read_bytes(block_size, &mut *disk, offset, &mut buf[..read_bytes_in_extent])?;

      offset = 0;
      left_bytes -= read_bytes_in_extent;
      buf = &mut buf[read_bytes_in_extent..];

      if left_bytes == 0 {
        break;
      }
    }

    Ok(read_bytes)
  }
}
