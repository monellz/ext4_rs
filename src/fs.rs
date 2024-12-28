use core::cell::RefCell;
extern crate alloc;
use alloc::vec::Vec;

use crate::error::Error;
use crate::io::{self, ReadWriteSeek, SeekFrom};

use crate::descriptor::BlockGroupDescriptor;
use crate::dir::Dir;
use crate::inode::Inode;
use crate::super_block::SuperBlock;
use crate::utils::bitmap::Bitmap;

pub struct FileSystem<IO: ReadWriteSeek> {
  pub disk: RefCell<IO>,
  pub super_block: RefCell<SuperBlock>,
  pub block_group_descriptors: RefCell<Vec<BlockGroupDescriptor>>,
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
      super_block: RefCell::new(super_block),
      block_group_descriptors: RefCell::new(descriptors),
    })
  }

  pub fn get_inode_pos(&self, ino: u64) -> u64 {
    let bgd_num = (ino - 1) / self.super_block.borrow().inodes_per_group as u64;
    let bdg = &self.block_group_descriptors.borrow()[bgd_num as usize];
    let inode_table_index = (ino - 1) % self.super_block.borrow().inodes_per_group as u64;

    let pos = bdg.get_inode_table_loc() * self.super_block.borrow().get_block_size()
      + inode_table_index * self.super_block.borrow().get_inode_size();
    pos
  }

  pub fn get_inode(&self, ino: u64) -> Result<Inode, Error<IO::Error>> {
    let pos = self.get_inode_pos(ino);
    let mut disk = self.disk.borrow_mut();
    disk.seek(SeekFrom::Start(pos))?;

    let inode = Inode::deserialize(&mut *disk)?;
    Ok(inode)
  }

  pub fn root_dir(&self) -> Dir<IO> {
    let inode = self.get_inode(Inode::ROOT_INO).unwrap();
    Dir::new(Inode::ROOT_INO as u64, inode, self)
  }
}

// alloc
impl<IO: ReadWriteSeek> FileSystem<IO> {
  pub fn alloc_contiguous_blocks(&self, count: u64, bgd_id: usize) -> Result<u64, Error<IO::Error>> {
    trace!(
      "FileSystem::alloc_contiguous_blocks count: {}, bgd_id: {}",
      count,
      bgd_id
    );
    let bgd = &mut self.block_group_descriptors.borrow_mut()[bgd_id];
    let mut free_blocks_count = bgd.get_free_blocks_count();
    if u64::from(free_blocks_count) < count {
      // 这一个block group没有足够的空间
      return Err(Error::NotEnoughSpace);
    }

    let block_bitmap_loc = bgd.get_block_bitmap_loc();
    let mut block_bitmap = {
      let mut disk = self.disk.borrow_mut();
      let offset = block_bitmap_loc * self.super_block.borrow().get_block_size();
      disk.seek(SeekFrom::Start(offset))?;
      let size = self.super_block.borrow().blocks_per_group as usize / Bitmap::BITS_PER_ITEM as usize;
      Bitmap::deserialize(&mut *disk, size)?
    };

    let start_block = block_bitmap.find_consecutive_unused_bits(count);
    if start_block.is_none() {
      // 这一个block group没有足够的连续空间
      return Err(Error::NotEnoughSpace);
    }
    let start_block = start_block.unwrap();
    block_bitmap.set_bits(start_block, count);
    // block bitmap写入disk
    trace!("FileSystem::alloc_contiguous_blocks: write block_bitmap to disk");
    {
      let mut disk = self.disk.borrow_mut();
      let offset = block_bitmap_loc * self.super_block.borrow().get_block_size();
      disk.seek(SeekFrom::Start(offset))?;
      block_bitmap.serialize(&mut *disk).unwrap();
    }

    // 更新block group descriptor
    bgd.set_block_bitmap_csum(&self.super_block.borrow(), &block_bitmap.data);
    free_blocks_count -= count as u32;
    bgd.set_free_blocks_count(free_blocks_count);
    bgd.set_checksum(bgd_id as u32, &self.super_block.borrow());

    // block group descriptor写入disk
    trace!("FileSystem::alloc_contiguous_blocks: write block group descriptor to disk");
    {
      let mut disk = self.disk.borrow_mut();
      let mut offset = SuperBlock::PADDING_OFFSET + core::mem::size_of::<SuperBlock>();
      offset += bgd_id * self.super_block.borrow().get_desc_size() as usize;
      disk.seek(SeekFrom::Start(offset as u64)).unwrap();
      bgd.serialize(&mut *disk).unwrap();
    }

    let start_block = bgd_id as u64 * self.super_block.borrow().blocks_per_group as u64 + start_block;
    trace!(
      "FileSystem::alloc_contiguous_blocks: start_block: {} count: {}",
      start_block,
      count
    );
    return Ok(start_block);
  }

  pub fn alloc_inode(&self, is_dir: bool) -> Result<u64, Error<IO::Error>> {
    trace!("FileSystem::alloc_inode is_dir: {}", is_dir);
    // TODO: 优化，让新的inode跟parent dir在同一个block group?
    let bgd_len = self.block_group_descriptors.borrow().len();
    for bgd_id in 0..bgd_len {
      let bgd = &mut self.block_group_descriptors.borrow_mut()[bgd_id];
      let mut free_inodes_count = bgd.get_free_inodes_count();
      if free_inodes_count > 0 {
        trace!(
          "FileSystem::alloc_inode: find bgd_id: {}, free_inodes_count: {}",
          bgd_id,
          bgd.get_free_inodes_count()
        );
        let inode_bitmap_loc = bgd.get_inode_bitmap_loc();
        let mut inode_bitmap = {
          let mut disk = self.disk.borrow_mut();
          disk
            .seek(SeekFrom::Start(
              inode_bitmap_loc * self.super_block.borrow().get_block_size(),
            ))
            .unwrap();
          let size = self.super_block.borrow().inodes_per_group as usize / Bitmap::BITS_PER_ITEM as usize;
          Bitmap::deserialize(&mut *disk, size).unwrap()
        };
        trace!("FileSystem::alloc_inode: inode_bitmap: {:?}", inode_bitmap);

        // +1/-1是因为inode从1开始
        let local_ino = inode_bitmap.find_unused_bit().unwrap();
        inode_bitmap.set_bit(local_ino);
        trace!("FileSystem::alloc_inode: new inode_bitmap: {:?}", inode_bitmap);

        // inode bitmap写入disk
        trace!("FileSystem::alloc_inode: write inode_bitmap to disk");
        {
          let mut disk = self.disk.borrow_mut();
          let offset = inode_bitmap_loc * self.super_block.borrow().get_block_size();
          disk.seek(SeekFrom::Start(offset)).unwrap();
          inode_bitmap.serialize(&mut *disk).unwrap();
        }

        // 更新block group descriptor
        trace!("FileSystem::alloc_inode: update block group descriptor");
        bgd.set_inode_bitmap_csum(&self.super_block.borrow(), &inode_bitmap.data);
        free_inodes_count -= 1;
        bgd.set_free_inodes_count(free_inodes_count);
        if is_dir {
          let used_dirs_count = bgd.get_used_dirs_count() + 1;
          bgd.set_used_dirs_count(used_dirs_count);
        }
        bgd.set_itable_unused(bgd.get_itable_unused() - 1);
        bgd.set_checksum(bgd_id as u32, &self.super_block.borrow());

        // block group descriptor写入disk
        trace!("FileSystem::alloc_inode: write block group descriptor to disk");
        {
          let mut disk = self.disk.borrow_mut();
          let mut offset = SuperBlock::PADDING_OFFSET + core::mem::size_of::<SuperBlock>();
          offset += bgd_id * self.super_block.borrow().get_desc_size() as usize;
          trace!("FileSystem::alloc_inode: offset: {}", offset);
          disk.seek(SeekFrom::Start(offset as u64)).unwrap();
          bgd.serialize(&mut *disk).unwrap();
        }

        // +1 是因为inode从1开始
        let new_ino = bgd_id as u64 * self.super_block.borrow().inodes_per_group as u64 + local_ino + 1;
        trace!("FileSystem::alloc_inode: new_ino: {}", new_ino);
        assert!((new_ino - 1) / self.super_block.borrow().inodes_per_group as u64 == bgd_id as u64);
        return Ok(new_ino);
      }
    }

    Err(Error::NotEnoughSpace)
  }
}
