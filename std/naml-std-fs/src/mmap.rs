///
/// Memory-mapped file support
///
/// The mmap API uses integer handles to reference memory-mapped regions.
/// Handles are stored in a global registry and must be explicitly closed.
///

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::sync::Mutex;

use memmap2::{Mmap, MmapMut, MmapOptions};
use naml_std_core::{naml_exception_set, naml_stack_capture, naml_string_new, NamlBytes, NamlString};

use crate::{naml_io_error_new, path_from_naml_string, throw_io_error};

/// Global registry for memory-mapped file handles
static MMAP_REGISTRY: std::sync::LazyLock<Mutex<MmapRegistry>> =
    std::sync::LazyLock::new(|| Mutex::new(MmapRegistry::new()));

/// Represents either a read-only or read-write mmap
#[allow(dead_code)]
enum MmapHandle {
    ReadOnly(Mmap, File),
    ReadWrite(MmapMut, File),
}

struct MmapRegistry {
    handles: HashMap<i64, MmapHandle>,
    next_id: i64,
}

impl MmapRegistry {
    fn new() -> Self {
        Self {
            handles: HashMap::new(),
            next_id: 1,
        }
    }

    fn insert(&mut self, handle: MmapHandle) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.handles.insert(id, handle);
        id
    }

    fn get(&self, id: i64) -> Option<&MmapHandle> {
        self.handles.get(&id)
    }

    fn get_mut(&mut self, id: i64) -> Option<&mut MmapHandle> {
        self.handles.get_mut(&id)
    }

    fn remove(&mut self, id: i64) -> Option<MmapHandle> {
        self.handles.remove(&id)
    }
}

/// Helper to throw an mmap-related IOError
fn throw_mmap_error(message: &str, handle: i64) -> *mut u8 {
    let path = format!("mmap handle {}", handle);

    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let path_ptr = naml_string_new(path.as_ptr(), path.len());
        let io_error = naml_io_error_new(message_ptr, path_ptr, -1);

        let stack = naml_stack_capture();
        *(io_error.add(8) as *mut *mut u8) = stack;

        naml_exception_set(io_error);
    }

    std::ptr::null_mut()
}

/// Open a memory-mapped file
/// Returns a handle (positive integer) on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_mmap_open(path: *const NamlString, writable: i64) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    let is_writable = writable != 0;

    let result = if is_writable {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path_str)
            .and_then(|file| {
                unsafe { MmapOptions::new().map_mut(&file) }
                    .map(|mmap| MmapHandle::ReadWrite(mmap, file))
            })
    } else {
        File::open(&path_str).and_then(|file| {
            unsafe { MmapOptions::new().map(&file) }.map(|mmap| MmapHandle::ReadOnly(mmap, file))
        })
    };

    match result {
        Ok(handle) => {
            let mut registry = MMAP_REGISTRY.lock().unwrap();
            registry.insert(handle)
        }
        Err(e) => {
            throw_io_error(e, &path_str);
            -1
        }
    }
}

/// Get the length of a memory-mapped region
/// Returns -1 and sets exception on invalid handle
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_mmap_len(handle: i64) -> i64 {
    let registry = MMAP_REGISTRY.lock().unwrap();
    match registry.get(handle) {
        Some(MmapHandle::ReadOnly(mmap, _)) => mmap.len() as i64,
        Some(MmapHandle::ReadWrite(mmap, _)) => mmap.len() as i64,
        None => {
            throw_mmap_error("Invalid mmap handle", handle);
            -1
        }
    }
}

/// Read a single byte from a memory-mapped region
/// Returns -1 and sets exception on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_mmap_read_byte(handle: i64, offset: i64) -> i64 {
    let registry = MMAP_REGISTRY.lock().unwrap();
    let mmap_ref = match registry.get(handle) {
        Some(MmapHandle::ReadOnly(mmap, _)) => mmap.as_ref(),
        Some(MmapHandle::ReadWrite(mmap, _)) => mmap.as_ref(),
        None => {
            throw_mmap_error("Invalid mmap handle", handle);
            return -1;
        }
    };

    let idx = offset as usize;
    if idx >= mmap_ref.len() {
        throw_mmap_error("Offset out of bounds", handle);
        return -1;
    }

    mmap_ref[idx] as i64
}

/// Write a single byte to a memory-mapped region
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_mmap_write_byte(handle: i64, offset: i64, value: i64) -> i64 {
    let mut registry = MMAP_REGISTRY.lock().unwrap();
    let mmap_ref = match registry.get_mut(handle) {
        Some(MmapHandle::ReadWrite(mmap, _)) => mmap,
        Some(MmapHandle::ReadOnly(_, _)) => {
            throw_mmap_error("Cannot write to read-only mmap", handle);
            return -1;
        }
        None => {
            throw_mmap_error("Invalid mmap handle", handle);
            return -1;
        }
    };

    let idx = offset as usize;
    if idx >= mmap_ref.len() {
        throw_mmap_error("Offset out of bounds", handle);
        return -1;
    }

    mmap_ref[idx] = value as u8;
    0
}

/// Read a range of bytes from a memory-mapped region
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_mmap_read(
    handle: i64,
    offset: i64,
    len: i64,
) -> *mut naml_std_core::NamlArray {
    let registry = MMAP_REGISTRY.lock().unwrap();
    let mmap_ref = match registry.get(handle) {
        Some(MmapHandle::ReadOnly(mmap, _)) => mmap.as_ref(),
        Some(MmapHandle::ReadWrite(mmap, _)) => mmap.as_ref(),
        None => {
            throw_mmap_error("Invalid mmap handle", handle);
            return std::ptr::null_mut();
        }
    };

    let start = offset as usize;
    let read_len = len as usize;
    let end = start.saturating_add(read_len);

    if start >= mmap_ref.len() || end > mmap_ref.len() {
        throw_mmap_error("Read range out of bounds", handle);
        return std::ptr::null_mut();
    }

    let arr = unsafe { naml_std_core::naml_array_new(read_len) };
    for i in start..end {
        unsafe { naml_std_core::naml_array_push(arr, mmap_ref[i] as i64) };
    }
    arr
}

/// Write bytes to a memory-mapped region
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_mmap_write(
    handle: i64,
    offset: i64,
    data: *const NamlBytes,
) -> i64 {
    if data.is_null() {
        return 0; // Nothing to write
    }

    let data_len = unsafe { (*data).len };
    let data_slice = unsafe { std::slice::from_raw_parts((*data).data.as_ptr(), data_len) };

    let mut registry = MMAP_REGISTRY.lock().unwrap();
    let mmap_ref = match registry.get_mut(handle) {
        Some(MmapHandle::ReadWrite(mmap, _)) => mmap,
        Some(MmapHandle::ReadOnly(_, _)) => {
            throw_mmap_error("Cannot write to read-only mmap", handle);
            return -1;
        }
        None => {
            throw_mmap_error("Invalid mmap handle", handle);
            return -1;
        }
    };

    let start = offset as usize;
    let end = start.saturating_add(data_len);

    if start >= mmap_ref.len() || end > mmap_ref.len() {
        throw_mmap_error("Write range out of bounds", handle);
        return -1;
    }

    mmap_ref[start..end].copy_from_slice(data_slice);
    0
}

/// Flush changes to disk
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_mmap_flush(handle: i64) -> i64 {
    let registry = MMAP_REGISTRY.lock().unwrap();
    match registry.get(handle) {
        Some(MmapHandle::ReadWrite(mmap, _)) => match mmap.flush() {
            Ok(()) => 0,
            Err(e) => {
                throw_io_error(e, &format!("mmap handle {}", handle));
                -1
            }
        },
        Some(MmapHandle::ReadOnly(_, _)) => 0, // No-op for read-only
        None => {
            throw_mmap_error("Invalid mmap handle", handle);
            -1
        }
    }
}

/// Close a memory-mapped region
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_mmap_close(handle: i64) -> i64 {
    let mut registry = MMAP_REGISTRY.lock().unwrap();
    match registry.remove(handle) {
        Some(_) => 0, // Drop will unmap
        None => {
            throw_mmap_error("Invalid mmap handle", handle);
            -1
        }
    }
}
