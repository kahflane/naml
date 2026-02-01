///
/// NamlBytes - Core Bytes Type
///
/// Provides the heap-allocated byte array type shared across all std crates.
/// Similar to strings but for raw binary data.
///

use crate::HeapHeader;

/// A heap-allocated byte array
#[repr(C)]
pub struct NamlBytes {
    pub header: HeapHeader,
    pub len: usize,
    pub capacity: usize,
    pub data: [u8; 0],
}
