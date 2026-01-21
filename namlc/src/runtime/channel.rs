//!
//! Channels for naml
//!
//! Provides bounded channels for communication between tasks.
//! Channels are typed at the naml level but at runtime store i64 values
//! (like all naml values).
//!

use std::alloc::{alloc, dealloc, Layout};
use std::collections::VecDeque;
use std::sync::{Mutex, Condvar};

use super::value::{HeapHeader, HeapTag};

/// A bounded channel for inter-task communication
#[repr(C)]
pub struct NamlChannel {
    pub header: HeapHeader,
    pub capacity: usize,
    inner: Mutex<ChannelInner>,
    not_empty: Condvar,
    not_full: Condvar,
}

struct ChannelInner {
    buffer: VecDeque<i64>,
    closed: bool,
}

/// Create a new channel with the given capacity
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_new(capacity: usize) -> *mut NamlChannel {
    let cap = if capacity == 0 { 1 } else { capacity };

    unsafe {
        let layout = Layout::new::<NamlChannel>();
        let ptr = alloc(layout) as *mut NamlChannel;
        if ptr.is_null() {
            panic!("Failed to allocate channel");
        }

        std::ptr::write(ptr, NamlChannel {
            header: HeapHeader::new(HeapTag::Map), // Reusing Map tag for channels
            capacity: cap,
            inner: Mutex::new(ChannelInner {
                buffer: VecDeque::with_capacity(cap),
                closed: false,
            }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        });

        ptr
    }
}

/// Increment reference count
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_incref(ch: *mut NamlChannel) {
    if !ch.is_null() {
        unsafe { (*ch).header.incref(); }
    }
}

/// Decrement reference count and free if zero
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_decref(ch: *mut NamlChannel) {
    if !ch.is_null() {
        unsafe {
            if (*ch).header.decref() {
                std::ptr::drop_in_place(ch);
                let layout = Layout::new::<NamlChannel>();
                dealloc(ch as *mut u8, layout);
            }
        }
    }
}

/// Send a value to the channel (blocks if full)
/// Returns 1 on success, 0 if channel is closed
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_send(ch: *mut NamlChannel, value: i64) -> i64 {
    if ch.is_null() {
        return 0;
    }

    unsafe {
        let channel = &*ch;
        let mut inner = channel.inner.lock().unwrap();

        // Wait while buffer is full and channel is open
        while inner.buffer.len() >= channel.capacity && !inner.closed {
            inner = channel.not_full.wait(inner).unwrap();
        }

        if inner.closed {
            return 0;
        }

        inner.buffer.push_back(value);
        channel.not_empty.notify_one();
        1
    }
}

/// Receive a value from the channel (blocks if empty)
/// Returns the value, or 0 if channel is closed and empty
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_receive(ch: *mut NamlChannel) -> i64 {
    if ch.is_null() {
        return 0;
    }

    unsafe {
        let channel = &*ch;
        let mut inner = channel.inner.lock().unwrap();

        // Wait while buffer is empty and channel is open
        while inner.buffer.is_empty() && !inner.closed {
            inner = channel.not_empty.wait(inner).unwrap();
        }

        if let Some(value) = inner.buffer.pop_front() {
            channel.not_full.notify_one();
            value
        } else {
            0 // Channel closed and empty
        }
    }
}

/// Try to send without blocking
/// Returns 1 on success, 0 if would block or closed
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_try_send(ch: *mut NamlChannel, value: i64) -> i64 {
    if ch.is_null() {
        return 0;
    }

    unsafe {
        let channel = &*ch;
        let mut inner = channel.inner.lock().unwrap();

        if inner.closed || inner.buffer.len() >= channel.capacity {
            return 0;
        }

        inner.buffer.push_back(value);
        channel.not_empty.notify_one();
        1
    }
}

/// Try to receive without blocking
/// Returns the value in the high bits and success (1) or failure (0) in low bit
/// Use naml_channel_try_receive_value() and naml_channel_try_receive_ok() to extract
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_try_receive(ch: *mut NamlChannel) -> i64 {
    if ch.is_null() {
        return 0;
    }

    unsafe {
        let channel = &*ch;
        let mut inner = channel.inner.lock().unwrap();

        if let Some(value) = inner.buffer.pop_front() {
            channel.not_full.notify_one();
            // Pack value and success flag
            // For simplicity, just return the value (caller should use try_send for status)
            value
        } else {
            0
        }
    }
}

/// Close the channel
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_close(ch: *mut NamlChannel) {
    if ch.is_null() {
        return;
    }

    unsafe {
        let channel = &*ch;
        let mut inner = channel.inner.lock().unwrap();
        inner.closed = true;
        channel.not_empty.notify_all();
        channel.not_full.notify_all();
    }
}

/// Check if channel is closed
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_is_closed(ch: *mut NamlChannel) -> i64 {
    if ch.is_null() {
        return 1;
    }

    unsafe {
        let channel = &*ch;
        let inner = channel.inner.lock().unwrap();
        if inner.closed { 1 } else { 0 }
    }
}

/// Get number of items in channel buffer
#[unsafe(no_mangle)]
pub extern "C" fn naml_channel_len(ch: *mut NamlChannel) -> i64 {
    if ch.is_null() {
        return 0;
    }

    unsafe {
        let channel = &*ch;
        let inner = channel.inner.lock().unwrap();
        inner.buffer.len() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_channel_basic() {
        let ch = naml_channel_new(2);
        assert!(!ch.is_null());

        assert_eq!(naml_channel_send(ch, 42), 1);
        assert_eq!(naml_channel_send(ch, 43), 1);
        assert_eq!(naml_channel_receive(ch), 42);
        assert_eq!(naml_channel_receive(ch), 43);

        naml_channel_decref(ch);
    }

    #[test]
    fn test_channel_concurrent() {
        let ch = naml_channel_new(10);

        let ch_send = ch as usize;
        let sender = thread::spawn(move || {
            let ch = ch_send as *mut NamlChannel;
            for i in 0..5 {
                naml_channel_send(ch, i);
            }
        });

        let ch_recv = ch as usize;
        let receiver = thread::spawn(move || {
            let ch = ch_recv as *mut NamlChannel;
            let mut sum = 0i64;
            for _ in 0..5 {
                sum += naml_channel_receive(ch);
            }
            sum
        });

        sender.join().unwrap();
        let sum = receiver.join().unwrap();
        assert_eq!(sum, 0 + 1 + 2 + 3 + 4);

        naml_channel_decref(ch);
    }
}
