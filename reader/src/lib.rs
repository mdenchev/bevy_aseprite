#![doc = include_str!("../README.MD")]

/// Errors used in this crate
pub mod error;

/// Raw data types
///
/// These are used to then construct the main [`Aseprite`] type.
pub mod raw;

mod computed;

pub use computed::*;
