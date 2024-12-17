#![cfg_attr(not(feature = "std"), no_std)]

extern crate log;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[macro_use]
mod log_macros;

pub mod block_device;
pub mod error;
pub mod fs;
pub mod io;
pub mod super_block;
