#![no_std]

extern crate alloc;
use alloc::sync::Arc;

pub mod block_device;
pub mod super_block;

use block_device::{Block, BlockDevice};
use super_block::{SuperBlock, SUPERBLOCK_OFFSET};

pub struct Ext4 {
  pub block_device: Arc<dyn BlockDevice>,
  pub super_block: SuperBlock,
}

impl Ext4 {
  pub fn new(block_device: Arc<dyn BlockDevice>) -> Self {
    let block = Block::load(&block_device, SUPERBLOCK_OFFSET, core::mem::size_of::<SuperBlock>());
    let super_block: SuperBlock = block.read_as();

    Self {
      block_device,
      super_block,
    }
  }
}
