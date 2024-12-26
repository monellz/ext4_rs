#![cfg_attr(not(feature = "std"), no_std)]

extern crate log;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[macro_use]
mod log_macros;

pub mod descriptor;
pub mod dir;
pub mod dir_entry;
pub mod error;
pub mod extent;
pub mod fs;
pub mod inode;
pub mod io;
pub mod super_block;
pub mod utils;
