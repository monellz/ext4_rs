use clap::Parser;
use log::*;
use simplelog::*;

extern crate alloc;
use alloc::sync::Arc;

use ext4_rs::Ext4;
use ext4_rs::block_device::BlockDevice;

#[derive(Debug)]
pub struct Disk {
  image: String,
}
impl BlockDevice for Disk {
  fn read(&self, offset: usize, size: usize) -> Vec<u8> {
    use std::fs::OpenOptions;
    use std::io::{Read, Seek};
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .open(&self.image)
      .unwrap();
    let mut buf = vec![0u8; size];
    file.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
    file.read_exact(&mut buf).unwrap();

    buf
  }

  fn write(&self, offset: usize, data: &[u8]) {
    use std::fs::OpenOptions;
    use std::io::{Seek, Write};
    let mut file = OpenOptions::new()
      .read(true)
      .write(true)
      .open(&self.image)
      .unwrap();

    file.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
    file.write_all(&data).unwrap();
  }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  #[arg(short, long, default_value = "ext4.img")]
  image: String,
}

fn main() {
  TermLogger::init(
    LevelFilter::Trace,
    Config::default(),
    TerminalMode::Mixed,
    ColorChoice::Auto,
  )
  .unwrap();

  let args = Args::parse();
  info!("{:?}", args);

  let disk = Disk {
    image: args.image,
  };
  let ext4 = Ext4::new(Arc::new(disk));

  info!("{:?}", ext4.super_block);
}
