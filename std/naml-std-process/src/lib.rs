///
/// naml-std-process - Process Management
///
/// Provides process spawning, management, and signal operations.
///
/// ## Process Operations (Issue #131)
///
/// - `getpid() -> int` - Get current process ID
/// - `getppid() -> int` - Get parent process ID
/// - `exit(code: int)` - Exit program with code (no return)
/// - `pipe() -> (int, int) throws ProcessError` - Create pipe (read_fd, write_fd)
/// - `start_process(name: string, args: [string]) -> int throws ProcessError` - Spawn child
/// - `find_process(pid: int) -> int throws ProcessError` - Handle to existing process by PID
///
/// ## Process Handle Methods (Issue #132)
///
/// - `wait(handle: int) -> ProcessStatus throws ProcessError` - Wait for exit
/// - `signal(handle: int, sig: int) throws ProcessError` - Send signal
/// - `kill(handle: int) throws ProcessError` - Kill process (SIGKILL)
/// - `release(handle: int)` - Release process handle resources
///
/// ## Signal Constants
///
/// SIGHUP=1, SIGINT=2, SIGQUIT=3, SIGKILL=9, SIGTERM=15, SIGSTOP=17, SIGCONT=19
///
/// ## ProcessStatus Struct
///
/// Fields: pid (int), code (int), exited (bool), success (bool), signal (int)
///
/// ## Platform Notes
///
/// - `getppid` uses libc on Unix, returns -1 on non-Unix
/// - `pipe` uses libc::pipe on Unix
/// - `signal` uses libc::kill on Unix
/// - Process handles are integer indices into a global process table
///

use naml_std_core::{
    naml_array_len, naml_array_get, naml_array_new, naml_array_push,
    naml_exception_set_typed, naml_stack_capture,
    naml_string_new, naml_struct_new, naml_struct_set_field, NamlArray, NamlString, NamlStruct,
};
use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::sync::LazyLock;

const EXCEPTION_TYPE_PROCESS_ERROR: i64 = 9;
const PROCESS_ERROR_STRUCT_TYPE_ID: u32 = 0xFFFF_0009;
const PROCESS_STATUS_STRUCT_TYPE_ID: u32 = 0xFFFF_000A;

struct ProcessTable {
    entries: HashMap<i64, ProcessEntry>,
    next_id: i64,
}

enum ProcessEntry {
    Owned(Child),
    External(u32),
}

static PROCESS_TABLE: LazyLock<Mutex<ProcessTable>> = LazyLock::new(|| {
    Mutex::new(ProcessTable {
        entries: HashMap::new(),
        next_id: 1,
    })
});

unsafe fn naml_from_string(s: &str) -> *mut NamlString {
    unsafe { naml_string_new(s.as_ptr(), s.len()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_process_error_new(
    message: *const NamlString,
    code: i64,
) -> *mut NamlStruct {
    unsafe {
        let exc = naml_struct_new(PROCESS_ERROR_STRUCT_TYPE_ID, 2);
        naml_struct_set_field(exc, 0, message as i64);
        naml_struct_set_field(exc, 1, code);
        exc
    }
}

fn throw_process_error(message: &str, code: i32) {
    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let exc = naml_process_error_new(message_ptr, code as i64);

        let stack = naml_stack_capture();
        *(exc as *mut u8).add(8).cast::<*mut u8>() = stack;

        naml_exception_set_typed(exc as *mut u8, EXCEPTION_TYPE_PROCESS_ERROR);
    }
}

fn make_process_status(pid: i64, code: i64, exited: bool, success: bool, sig: i64) -> *mut NamlArray {
    unsafe {
        let arr = naml_array_new(5);
        naml_array_push(arr, pid);
        naml_array_push(arr, code);
        naml_array_push(arr, if exited { 1 } else { 0 });
        naml_array_push(arr, if success { 1 } else { 0 });
        naml_array_push(arr, sig);
        arr
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_getpid() -> i64 {
    std::process::id() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_getppid() -> i64 {
    #[cfg(unix)]
    {
        unsafe { libc::getppid() as i64 }
    }
    #[cfg(not(unix))]
    {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_exit(code: i64) {
    std::process::exit(code as i32);
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_pipe_read() -> i64 {
    #[cfg(unix)]
    {
        let mut fds = [0i32; 2];
        let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if rc != 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            throw_process_error("pipe() failed", errno);
            return -1;
        }
        PIPE_WRITE_FD.with(|cell| cell.set(fds[1] as i64));
        fds[0] as i64
    }
    #[cfg(not(unix))]
    {
        throw_process_error("pipe() not supported on this platform", -1);
        -1
    }
}

thread_local! {
    static PIPE_WRITE_FD: std::cell::Cell<i64> = const { std::cell::Cell::new(-1) };
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_pipe_write() -> i64 {
    PIPE_WRITE_FD.with(|cell| cell.get())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_process_start(
    name: *const NamlString,
    args: *mut NamlArray,
) -> i64 {
    let name_str = unsafe {
        let slice = std::slice::from_raw_parts((*name).data.as_ptr(), (*name).len);
        String::from_utf8_lossy(slice).into_owned()
    };

    let arg_count = unsafe { naml_array_len(args) } as usize;
    let mut arg_vec: Vec<String> = Vec::with_capacity(arg_count);
    for i in 0..arg_count {
        let s_ptr = unsafe { naml_array_get(args, i as i64) } as *const NamlString;
        if !s_ptr.is_null() {
            let s = unsafe {
                let slice = std::slice::from_raw_parts((*s_ptr).data.as_ptr(), (*s_ptr).len);
                String::from_utf8_lossy(slice).into_owned()
            };
            arg_vec.push(s);
        }
    }

    match Command::new(&name_str).args(&arg_vec).spawn() {
        Ok(child) => {
            let mut table = PROCESS_TABLE.lock().unwrap();
            let id = table.next_id;
            table.next_id += 1;
            table.entries.insert(id, ProcessEntry::Owned(child));
            id
        }
        Err(e) => {
            let msg = format!("failed to start process '{}': {}", name_str, e);
            let code = e.raw_os_error().unwrap_or(-1);
            throw_process_error(&msg, code);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_find(pid: i64) -> i64 {
    if pid <= 0 {
        throw_process_error("invalid pid", -1);
        return -1;
    }

    #[cfg(unix)]
    {
        let rc = unsafe { libc::kill(pid as i32, 0) };
        if rc != 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            throw_process_error(&format!("process {} not found or not accessible", pid), errno);
            return -1;
        }
    }

    let mut table = PROCESS_TABLE.lock().unwrap();
    let id = table.next_id;
    table.next_id += 1;
    table.entries.insert(id, ProcessEntry::External(pid as u32));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_wait(handle: i64) -> *mut NamlArray {
    let mut table = PROCESS_TABLE.lock().unwrap();
    let entry = match table.entries.get_mut(&handle) {
        Some(e) => e,
        None => {
            throw_process_error("invalid process handle", -1);
            return make_process_status(-1, -1, false, false, 0);
        }
    };

    match entry {
        ProcessEntry::Owned(child) => {
            let pid = child.id() as i64;
            match child.wait() {
                Ok(status) => {
                    let code = status.code().unwrap_or(-1) as i64;
                    let exited = status.code().is_some();
                    let success = status.success();
                    #[cfg(unix)]
                    let sig = {
                        use std::os::unix::process::ExitStatusExt;
                        status.signal().unwrap_or(0) as i64
                    };
                    #[cfg(not(unix))]
                    let sig = 0i64;
                    make_process_status(pid, code, exited, success, sig)
                }
                Err(e) => {
                    let msg = format!("wait failed: {}", e);
                    let code = e.raw_os_error().unwrap_or(-1);
                    throw_process_error(&msg, code);
                    make_process_status(pid, -1, false, false, 0)
                }
            }
        }
        ProcessEntry::External(pid) => {
            #[cfg(unix)]
            {
                let mut status: libc::c_int = 0;
                let rc = unsafe { libc::waitpid(*pid as i32, &mut status, 0) };
                if rc < 0 {
                    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                    throw_process_error("waitpid failed (process may not be a child)", errno);
                    return make_process_status(*pid as i64, -1, false, false, 0);
                }
                let exited = libc::WIFEXITED(status);
                let code = if exited { libc::WEXITSTATUS(status) as i64 } else { -1 };
                let signaled = libc::WIFSIGNALED(status);
                let sig = if signaled { libc::WTERMSIG(status) as i64 } else { 0 };
                let success = exited && code == 0;
                make_process_status(*pid as i64, code, exited, success, sig)
            }
            #[cfg(not(unix))]
            {
                throw_process_error("wait on external process not supported on this platform", -1);
                make_process_status(*pid as i64, -1, false, false, 0)
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_signal(handle: i64, sig: i64) {
    let table = PROCESS_TABLE.lock().unwrap();
    let entry = match table.entries.get(&handle) {
        Some(e) => e,
        None => {
            throw_process_error("invalid process handle", -1);
            return;
        }
    };

    let pid = match entry {
        ProcessEntry::Owned(child) => child.id() as i32,
        ProcessEntry::External(pid) => *pid as i32,
    };

    #[cfg(unix)]
    {
        let rc = unsafe { libc::kill(pid, sig as i32) };
        if rc != 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            throw_process_error(&format!("signal({}) failed for pid {}", sig, pid), errno);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, sig);
        throw_process_error("signal() not supported on this platform", -1);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_kill(handle: i64) {
    let mut table = PROCESS_TABLE.lock().unwrap();
    let entry = match table.entries.get_mut(&handle) {
        Some(e) => e,
        None => {
            throw_process_error("invalid process handle", -1);
            return;
        }
    };

    match entry {
        ProcessEntry::Owned(child) => {
            if let Err(e) = child.kill() {
                let msg = format!("kill failed: {}", e);
                let code = e.raw_os_error().unwrap_or(-1);
                throw_process_error(&msg, code);
            }
        }
        ProcessEntry::External(pid) => {
            #[cfg(unix)]
            {
                let rc = unsafe { libc::kill(*pid as i32, libc::SIGKILL) };
                if rc != 0 {
                    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                    throw_process_error(&format!("kill failed for pid {}", pid), errno);
                }
            }
            #[cfg(not(unix))]
            {
                let _ = pid;
                throw_process_error("kill() not supported on this platform", -1);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_release(handle: i64) {
    let mut table = PROCESS_TABLE.lock().unwrap();
    table.entries.remove(&handle);
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_sighup() -> i64 { 1 }

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_sigint() -> i64 { 2 }

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_sigquit() -> i64 { 3 }

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_sigkill() -> i64 { 9 }

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_sigterm() -> i64 { 15 }

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_sigstop() -> i64 { 17 }

#[unsafe(no_mangle)]
pub extern "C" fn naml_process_sigcont() -> i64 { 19 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_getpid() {
        let pid = naml_process_getpid();
        assert!(pid > 0);
    }

    #[test]
    #[cfg(unix)]
    fn test_getppid() {
        let ppid = naml_process_getppid();
        assert!(ppid > 0);
    }

    #[test]
    fn test_signal_constants() {
        assert_eq!(naml_process_sighup(), 1);
        assert_eq!(naml_process_sigint(), 2);
        assert_eq!(naml_process_sigquit(), 3);
        assert_eq!(naml_process_sigkill(), 9);
        assert_eq!(naml_process_sigterm(), 15);
        assert_eq!(naml_process_sigstop(), 17);
        assert_eq!(naml_process_sigcont(), 19);
    }

    #[test]
    #[cfg(unix)]
    fn test_pipe() {
        let read_fd = naml_process_pipe_read();
        assert!(read_fd >= 0);
        let write_fd = naml_process_pipe_write();
        assert!(write_fd >= 0);
        assert_ne!(read_fd, write_fd);
        unsafe {
            libc::close(read_fd as i32);
            libc::close(write_fd as i32);
        }
    }
}
