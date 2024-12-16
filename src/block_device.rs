extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;

pub trait BlockDevice: Send + Sync + Any {
  fn read(&self, offset: usize, size: usize) -> Vec<u8>;
  fn write(&self, offset: usize, data: &[u8]);
}

pub struct Block {
  pub data: Vec<u8>,
  pub offset: usize,
}

impl Block {
  pub fn load(block_device: &Arc<dyn BlockDevice>, offset: usize, size: usize) -> Self {
    let data = block_device.read(offset, size);
    Self { data, offset }
  }

  pub fn read_as<T: Copy>(&self) -> T {
    assert!(self.data.len() >= core::mem::size_of::<T>());
    let ptr = self.data.as_ptr() as *const T;
    unsafe { ptr.read_unaligned() }
  }
}
