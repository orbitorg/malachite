use core::fmt::{Debug, Display};
use core::hash::Hash;

/// Defines the requirements for a height type.
///
/// A height denotes the number of blocks (values) created since the chain began.
///
/// A height of 0 represents a chain which has not yet produced a block.
pub trait Height
where
    Self: Default
        + Copy
        + Clone
        + Debug
        + Display
        + PartialEq
        + Eq
        + Hash
        + PartialOrd
        + Ord
        + Send
        + Sync,
{
    /// Increment the height by one.
    fn increment(&self) -> Self;

    /// Convert the height to a `u64`.
    fn as_u64(&self) -> u64;
}
