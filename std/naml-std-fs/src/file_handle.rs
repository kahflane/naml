///
/// File Handle/Stream Operations
///
/// Provides C-style file cursor operations for sequential reading and random access.
/// Uses a global handle registry pattern similar to mmap.
///
/// Modes:
/// - "r"  - read only (file must exist)
/// - "w"  - write only (creates/truncates)
/// - "a"  - append only (creates if needed)
/// - "r+" - read/write (file must exist)
/// - "w+" - read/write (creates/truncates)
/// - "a+" - read/append (creates if needed)
///

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read as IoRead, Seek, SeekFrom, Write as IoWrite};
use std::sync::Mutex;

use naml_std_core::{naml_exception_set, naml_stack_capture, naml_string_new, NamlString};

use crate::{naml_io_error_new, path_from_naml_string, throw_io_error};

/// Seek constants
pub const SEEK_SET: i64 = 0;
pub const SEEK_CUR: i64 = 1;
pub const SEEK_END: i64 = 2;

/// File open mode
#[derive(Clone, Copy, Debug, PartialEq)]
enum FileMode {
    Read,
    Write,
    Append,
    ReadWrite,
    WritePlus,
    AppendPlus,
}

impl FileMode {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "r" => Some(FileMode::Read),
            "w" => Some(FileMode::Write),
            "a" => Some(FileMode::Append),
            "r+" => Some(FileMode::ReadWrite),
            "w+" => Some(FileMode::WritePlus),
            "a+" => Some(FileMode::AppendPlus),
            _ => None,
        }
    }

    fn can_read(&self) -> bool {
        matches!(
            self,
            FileMode::Read | FileMode::ReadWrite | FileMode::WritePlus | FileMode::AppendPlus
        )
    }

    fn can_write(&self) -> bool {
        matches!(
            self,
            FileMode::Write
                | FileMode::Append
                | FileMode::ReadWrite
                | FileMode::WritePlus
                | FileMode::AppendPlus
        )
    }
}

/// Buffered file handle supporting read/write operations
struct FileHandle {
    file: File,
    reader: Option<BufReader<File>>,
    writer: Option<BufWriter<File>>,
    path: String,
    mode: FileMode,
    eof: bool,
}

impl FileHandle {
    fn new(file: File, path: String, mode: FileMode) -> std::io::Result<Self> {
        let reader = if mode.can_read() {
            Some(BufReader::new(file.try_clone()?))
        } else {
            None
        };
        let writer = if mode.can_write() {
            Some(BufWriter::new(file.try_clone()?))
        } else {
            None
        };
        Ok(Self {
            file,
            reader,
            writer,
            path,
            mode,
            eof: false,
        })
    }
}

struct FileRegistry {
    handles: HashMap<i64, FileHandle>,
    next_id: i64,
}

impl FileRegistry {
    fn new() -> Self {
        Self {
            handles: HashMap::new(),
            next_id: 1,
        }
    }

    fn insert(&mut self, handle: FileHandle) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.handles.insert(id, handle);
        id
    }

    fn get(&self, id: i64) -> Option<&FileHandle> {
        self.handles.get(&id)
    }

    fn get_mut(&mut self, id: i64) -> Option<&mut FileHandle> {
        self.handles.get_mut(&id)
    }

    fn remove(&mut self, id: i64) -> Option<FileHandle> {
        self.handles.remove(&id)
    }
}

static FILE_REGISTRY: std::sync::LazyLock<Mutex<FileRegistry>> =
    std::sync::LazyLock::new(|| Mutex::new(FileRegistry::new()));

/// Helper to throw a file handle error
fn throw_file_error(message: &str, handle: i64) {
    let path = format!("file handle {}", handle);
    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let path_ptr = naml_string_new(path.as_ptr(), path.len());
        let io_error = naml_io_error_new(message_ptr, path_ptr, -1);
        let stack = naml_stack_capture();
        *(io_error.add(8) as *mut *mut u8) = stack;
        naml_exception_set(io_error);
    }
}

/// Open a file with the specified mode
/// Returns a handle (positive integer) on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_open(
    path: *const NamlString,
    mode: *const NamlString,
) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    let mode_str = unsafe { path_from_naml_string(mode) };

    let file_mode = match FileMode::from_str(&mode_str) {
        Some(m) => m,
        None => {
            let msg = format!("Invalid file mode: {}", mode_str);
            unsafe {
                let message_ptr = naml_string_new(msg.as_ptr(), msg.len());
                let path_ptr = naml_string_new(path_str.as_ptr(), path_str.len());
                let io_error = naml_io_error_new(message_ptr, path_ptr, -1);
                let stack = naml_stack_capture();
                *(io_error.add(8) as *mut *mut u8) = stack;
                naml_exception_set(io_error);
            }
            return -1;
        }
    };

    let file_result = match file_mode {
        FileMode::Read => OpenOptions::new().read(true).open(&path_str),
        FileMode::Write => OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path_str),
        FileMode::Append => OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path_str),
        FileMode::ReadWrite => OpenOptions::new().read(true).write(true).open(&path_str),
        FileMode::WritePlus => OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path_str),
        FileMode::AppendPlus => OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path_str),
    };

    match file_result {
        Ok(file) => match FileHandle::new(file, path_str.clone(), file_mode) {
            Ok(handle) => {
                let mut registry = FILE_REGISTRY.lock().unwrap();
                registry.insert(handle)
            }
            Err(e) => {
                throw_io_error(e, &path_str);
                -1
            }
        },
        Err(e) => {
            throw_io_error(e, &path_str);
            -1
        }
    }
}

/// Close a file handle
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_close(handle: i64) -> i64 {
    let mut registry = FILE_REGISTRY.lock().unwrap();
    match registry.remove(handle) {
        Some(mut fh) => {
            if let Some(ref mut writer) = fh.writer {
                let _ = writer.flush();
            }
            0
        }
        None => {
            throw_file_error("Invalid file handle", handle);
            -1
        }
    }
}

/// Read up to `count` bytes from file
/// Returns string with bytes read, or null on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_read(handle: i64, count: i64) -> *mut NamlString {
    let mut registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get_mut(handle) {
        Some(h) => h,
        None => {
            throw_file_error("Invalid file handle", handle);
            return std::ptr::null_mut();
        }
    };

    if !fh.mode.can_read() {
        throw_file_error("File not opened for reading", handle);
        return std::ptr::null_mut();
    }

    let reader = match fh.reader.as_mut() {
        Some(r) => r,
        None => {
            throw_file_error("No reader available", handle);
            return std::ptr::null_mut();
        }
    };

    let mut buf = vec![0u8; count as usize];
    match reader.read(&mut buf) {
        Ok(0) => {
            fh.eof = true;
            unsafe { naml_string_new(std::ptr::null(), 0) }
        }
        Ok(n) => {
            buf.truncate(n);
            unsafe { naml_string_new(buf.as_ptr(), n) }
        }
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            std::ptr::null_mut()
        }
    }
}

/// Read a single line from file (including newline if present)
/// Returns the line, or null on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_read_line(handle: i64) -> *mut NamlString {
    let mut registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get_mut(handle) {
        Some(h) => h,
        None => {
            throw_file_error("Invalid file handle", handle);
            return std::ptr::null_mut();
        }
    };

    if !fh.mode.can_read() {
        throw_file_error("File not opened for reading", handle);
        return std::ptr::null_mut();
    }

    let reader = match fh.reader.as_mut() {
        Some(r) => r,
        None => {
            throw_file_error("No reader available", handle);
            return std::ptr::null_mut();
        }
    };

    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => {
            fh.eof = true;
            unsafe { naml_string_new(std::ptr::null(), 0) }
        }
        Ok(_) => unsafe { naml_string_new(line.as_ptr(), line.len()) },
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            std::ptr::null_mut()
        }
    }
}

/// Read all remaining content from file
/// Returns the content, or null on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_read_all(handle: i64) -> *mut NamlString {
    let mut registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get_mut(handle) {
        Some(h) => h,
        None => {
            throw_file_error("Invalid file handle", handle);
            return std::ptr::null_mut();
        }
    };

    if !fh.mode.can_read() {
        throw_file_error("File not opened for reading", handle);
        return std::ptr::null_mut();
    }

    let reader = match fh.reader.as_mut() {
        Some(r) => r,
        None => {
            throw_file_error("No reader available", handle);
            return std::ptr::null_mut();
        }
    };

    let mut content = String::new();
    match reader.read_to_string(&mut content) {
        Ok(_) => {
            fh.eof = true;
            unsafe { naml_string_new(content.as_ptr(), content.len()) }
        }
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            std::ptr::null_mut()
        }
    }
}

/// Get current file position
/// Returns position, or -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_tell(handle: i64) -> i64 {
    let mut registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get_mut(handle) {
        Some(h) => h,
        None => {
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    match fh.file.stream_position() {
        Ok(pos) => pos as i64,
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

/// Seek to position in file
/// whence: 0 = SEEK_SET, 1 = SEEK_CUR, 2 = SEEK_END
/// Returns new position, or -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_seek(handle: i64, offset: i64, whence: i64) -> i64 {
    let mut registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get_mut(handle) {
        Some(h) => h,
        None => {
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    let seek_from = match whence {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => {
            throw_file_error("Invalid seek whence value", handle);
            return -1;
        }
    };

    // Flush writer before seeking if present
    if let Some(ref mut writer) = fh.writer {
        let _ = writer.flush();
    }

    // Seek the underlying file
    match fh.file.seek(seek_from) {
        Ok(new_pos) => {
            // Recreate buffered reader/writer at new position
            if let Some(ref mut reader) = fh.reader {
                if let Ok(cloned) = fh.file.try_clone() {
                    let mut new_reader = BufReader::new(cloned);
                    let _ = new_reader.seek(SeekFrom::Start(new_pos));
                    *reader = new_reader;
                }
            }
            if let Some(ref mut writer) = fh.writer {
                if let Ok(cloned) = fh.file.try_clone() {
                    let mut new_writer = BufWriter::new(cloned);
                    let _ = new_writer.seek(SeekFrom::Start(new_pos));
                    *writer = new_writer;
                }
            }
            fh.eof = false;
            new_pos as i64
        }
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

/// Write string to file
/// Returns number of bytes written, or -1 on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_write(handle: i64, content: *const NamlString) -> i64 {
    let content_str = unsafe { path_from_naml_string(content) };

    let mut registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get_mut(handle) {
        Some(h) => h,
        None => {
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    if !fh.mode.can_write() {
        throw_file_error("File not opened for writing", handle);
        return -1;
    }

    let writer = match fh.writer.as_mut() {
        Some(w) => w,
        None => {
            throw_file_error("No writer available", handle);
            return -1;
        }
    };

    match writer.write(content_str.as_bytes()) {
        Ok(n) => n as i64,
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

/// Write string with newline to file
/// Returns number of bytes written, or -1 on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_write_line(handle: i64, content: *const NamlString) -> i64 {
    let content_str = unsafe { path_from_naml_string(content) };

    let mut registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get_mut(handle) {
        Some(h) => h,
        None => {
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    if !fh.mode.can_write() {
        throw_file_error("File not opened for writing", handle);
        return -1;
    }

    let writer = match fh.writer.as_mut() {
        Some(w) => w,
        None => {
            throw_file_error("No writer available", handle);
            return -1;
        }
    };

    let line = format!("{}\n", content_str);
    match writer.write(line.as_bytes()) {
        Ok(n) => n as i64,
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

/// Flush buffered writes to disk
/// Returns 0 on success, -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_flush(handle: i64) -> i64 {
    let mut registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get_mut(handle) {
        Some(h) => h,
        None => {
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    if let Some(ref mut writer) = fh.writer {
        match writer.flush() {
            Ok(()) => 0,
            Err(e) => {
                let path = fh.path.clone();
                drop(registry);
                throw_io_error(e, &path);
                -1
            }
        }
    } else {
        0
    }
}

/// Check if end of file has been reached
/// Returns 1 if EOF, 0 otherwise, -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_eof(handle: i64) -> i64 {
    let registry = FILE_REGISTRY.lock().unwrap();
    match registry.get(handle) {
        Some(fh) => {
            if fh.eof {
                1
            } else {
                0
            }
        }
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            -1
        }
    }
}

/// Get file size
/// Returns size in bytes, or -1 on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_size(handle: i64) -> i64 {
    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    match fh.file.metadata() {
        Ok(meta) => meta.len() as i64,
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

/// Read bytes from file at a specific offset without changing the file cursor
/// Returns string with bytes read, or null on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_read_at(
    handle: i64,
    buf_size: i64,
    offset: i64,
) -> *mut NamlString {
    use std::os::unix::fs::FileExt;

    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return std::ptr::null_mut();
        }
    };

    let mut buf = vec![0u8; buf_size as usize];
    match fh.file.read_at(&mut buf, offset as u64) {
        Ok(0) => unsafe { naml_string_new(std::ptr::null(), 0) },
        Ok(n) => {
            buf.truncate(n);
            unsafe { naml_string_new(buf.as_ptr(), n) }
        }
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            std::ptr::null_mut()
        }
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_read_at(
    handle: i64,
    buf_size: i64,
    offset: i64,
) -> *mut NamlString {
    use std::os::windows::fs::FileExt;

    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return std::ptr::null_mut();
        }
    };

    let mut buf = vec![0u8; buf_size as usize];
    match fh.file.seek_read(&mut buf, offset as u64) {
        Ok(0) => unsafe { naml_string_new(std::ptr::null(), 0) },
        Ok(n) => {
            buf.truncate(n);
            unsafe { naml_string_new(buf.as_ptr(), n) }
        }
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            std::ptr::null_mut()
        }
    }
}

/// Write bytes to file at a specific offset without changing the file cursor
/// Returns number of bytes written, or -1 on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_write_at(
    handle: i64,
    content: *const NamlString,
    offset: i64,
) -> i64 {
    use std::os::unix::fs::FileExt;

    let content_str = unsafe { crate::path_from_naml_string(content) };

    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    match fh.file.write_at(content_str.as_bytes(), offset as u64) {
        Ok(n) => n as i64,
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_write_at(
    handle: i64,
    content: *const NamlString,
    offset: i64,
) -> i64 {
    use std::os::windows::fs::FileExt;

    let content_str = unsafe { crate::path_from_naml_string(content) };

    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    match fh.file.seek_write(content_str.as_bytes(), offset as u64) {
        Ok(n) => n as i64,
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

/// Get the path associated with a file handle
/// Returns the path string, or null on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_name(handle: i64) -> *mut NamlString {
    let registry = FILE_REGISTRY.lock().unwrap();
    match registry.get(handle) {
        Some(fh) => unsafe { naml_string_new(fh.path.as_ptr(), fh.path.len()) },
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            std::ptr::null_mut()
        }
    }
}

/// Get file metadata from an open handle
/// Returns an array with: [size, mode, modified, created, is_dir, is_file, is_symlink]
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_file_stat(handle: i64) -> *mut naml_std_core::NamlArray {
    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return std::ptr::null_mut();
        }
    };

    match fh.file.metadata() {
        Ok(meta) => crate::metadata_to_array(&meta),
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            std::ptr::null_mut()
        }
    }
}

/// Truncate file to specified size via handle
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_truncate(handle: i64, size: i64) -> i64 {
    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    match fh.file.set_len(size as u64) {
        Ok(()) => 0,
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

/// Change file permissions via handle (Unix mode bits)
/// Returns 0 on success, sets exception on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_chmod(handle: i64, mode: i64) -> i64 {
    use std::os::unix::fs::PermissionsExt;

    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    let permissions = std::fs::Permissions::from_mode(mode as u32);
    match fh.file.set_permissions(permissions) {
        Ok(()) => 0,
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_chmod(handle: i64, mode: i64) -> i64 {
    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    let readonly = (mode & 0o200) == 0;
    match fh.file.metadata() {
        Ok(meta) => {
            let mut perms = meta.permissions();
            perms.set_readonly(readonly);
            match fh.file.set_permissions(perms) {
                Ok(()) => 0,
                Err(e) => {
                    let path = fh.path.clone();
                    drop(registry);
                    throw_io_error(e, &path);
                    -1
                }
            }
        }
        Err(e) => {
            let path = fh.path.clone();
            drop(registry);
            throw_io_error(e, &path);
            -1
        }
    }
}

/// Change file ownership via handle (Unix only)
/// Returns 0 on success, sets exception on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_chown(handle: i64, uid: i64, gid: i64) -> i64 {
    use std::os::unix::io::AsRawFd;

    let registry = FILE_REGISTRY.lock().unwrap();
    let fh = match registry.get(handle) {
        Some(h) => h,
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            return -1;
        }
    };

    let fd = fh.file.as_raw_fd();
    let result = unsafe { libc::fchown(fd, uid as libc::uid_t, gid as libc::gid_t) };
    if result == 0 {
        0
    } else {
        let e = std::io::Error::last_os_error();
        let path = fh.path.clone();
        drop(registry);
        throw_io_error(e, &path);
        -1
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn naml_fs_file_chown(handle: i64, _uid: i64, _gid: i64) -> i64 {
    let registry = FILE_REGISTRY.lock().unwrap();
    match registry.get(handle) {
        Some(fh) => {
            let path = fh.path.clone();
            drop(registry);
            let e = std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "file_chown is not supported on this platform",
            );
            throw_io_error(e, &path);
            -1
        }
        None => {
            drop(registry);
            throw_file_error("Invalid file handle", handle);
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_mode_parsing() {
        assert_eq!(FileMode::from_str("r"), Some(FileMode::Read));
        assert_eq!(FileMode::from_str("w"), Some(FileMode::Write));
        assert_eq!(FileMode::from_str("a"), Some(FileMode::Append));
        assert_eq!(FileMode::from_str("r+"), Some(FileMode::ReadWrite));
        assert_eq!(FileMode::from_str("w+"), Some(FileMode::WritePlus));
        assert_eq!(FileMode::from_str("a+"), Some(FileMode::AppendPlus));
        assert_eq!(FileMode::from_str("invalid"), None);
    }

    #[test]
    fn test_file_mode_capabilities() {
        assert!(FileMode::Read.can_read());
        assert!(!FileMode::Read.can_write());

        assert!(!FileMode::Write.can_read());
        assert!(FileMode::Write.can_write());

        assert!(FileMode::ReadWrite.can_read());
        assert!(FileMode::ReadWrite.can_write());
    }
}
