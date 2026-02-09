///
/// std::encoding::binary — Binary Buffer Read/Write Operations
///
/// Provides multi-byte integer and floating-point read/write operations on byte buffers,
/// plus buffer manipulation (alloc, slice, concat, search, etc.).
/// All integer reads/writes support both big-endian and little-endian byte orders.
///
/// **Read operations:** read_u8, read_i8, read_u16_be/le, read_i16_be/le,
///   read_u32_be/le, read_i32_be/le, read_u64_be/le, read_i64_be/le,
///   read_f32_be/le, read_f64_be/le
///
/// **Write operations:** write_u8, write_i8, write_u16_be/le, write_i16_be/le,
///   write_u32_be/le, write_i32_be/le, write_u64_be/le, write_i64_be/le,
///   write_f32_be/le, write_f64_be/le
///
/// **Buffer operations:** alloc, from_string, len, capacity, slice, concat,
///   append, copy_within, clear, resize, fill
///
/// **Search operations:** index_of, contains, starts_with, ends_with, equals
///

use naml_std_core::bytes::NamlBytes;
use naml_std_core::value::NamlString;
use std::alloc::Layout;

fn buf_data(buf: *const NamlBytes) -> &'static [u8] {
    unsafe {
        let len = (*buf).len;
        std::slice::from_raw_parts((*buf).data.as_ptr(), len)
    }
}

fn buf_data_mut(buf: *mut NamlBytes) -> &'static mut [u8] {
    unsafe {
        let len = (*buf).len;
        std::slice::from_raw_parts_mut((*buf).data.as_mut_ptr(), len)
    }
}

fn create_bytes_with_capacity(cap: usize) -> *mut NamlBytes {
    let cap = if cap == 0 { 8 } else { cap };
    unsafe {
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + cap,
            std::mem::align_of::<NamlBytes>(),
        )
        .unwrap();
        let ptr = std::alloc::alloc_zeroed(layout) as *mut NamlBytes;
        (*ptr).header = naml_std_core::HeapHeader::new(naml_std_core::HeapTag::Bytes);
        (*ptr).len = 0;
        (*ptr).capacity = cap;
        ptr
    }
}

fn create_bytes_from_slice(data: &[u8]) -> *mut NamlBytes {
    let cap = if data.is_empty() { 8 } else { data.len() };
    unsafe {
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + cap,
            std::mem::align_of::<NamlBytes>(),
        )
        .unwrap();
        let ptr = std::alloc::alloc(layout) as *mut NamlBytes;
        (*ptr).header = naml_std_core::HeapHeader::new(naml_std_core::HeapTag::Bytes);
        (*ptr).len = data.len();
        (*ptr).capacity = cap;
        if !data.is_empty() {
            std::ptr::copy_nonoverlapping(data.as_ptr(), (*ptr).data.as_mut_ptr(), data.len());
        }
        ptr
    }
}

fn realloc_bytes(buf: *mut NamlBytes, new_cap: usize) -> *mut NamlBytes {
    unsafe {
        let old_cap = (*buf).capacity;
        let old_layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + old_cap,
            std::mem::align_of::<NamlBytes>(),
        )
        .unwrap();
        let new_layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + new_cap,
            std::mem::align_of::<NamlBytes>(),
        )
        .unwrap();
        let new_ptr = std::alloc::realloc(buf as *mut u8, old_layout, new_layout.size()) as *mut NamlBytes;
        (*new_ptr).capacity = new_cap;
        new_ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_u8(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off >= data.len() { return 0; }
    data[off] as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_i8(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off >= data.len() { return 0; }
    (data[off] as i8) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_u16_be(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 2 > data.len() { return 0; }
    u16::from_be_bytes([data[off], data[off + 1]]) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_u16_le(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 2 > data.len() { return 0; }
    u16::from_le_bytes([data[off], data[off + 1]]) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_i16_be(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 2 > data.len() { return 0; }
    i16::from_be_bytes([data[off], data[off + 1]]) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_i16_le(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 2 > data.len() { return 0; }
    i16::from_le_bytes([data[off], data[off + 1]]) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_u32_be(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return 0; }
    u32::from_be_bytes(data[off..off + 4].try_into().unwrap()) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_u32_le(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return 0; }
    u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_i32_be(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return 0; }
    i32::from_be_bytes(data[off..off + 4].try_into().unwrap()) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_i32_le(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return 0; }
    i32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_u64_be(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return 0; }
    u64::from_be_bytes(data[off..off + 8].try_into().unwrap()) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_u64_le(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return 0; }
    u64::from_le_bytes(data[off..off + 8].try_into().unwrap()) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_i64_be(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return 0; }
    i64::from_be_bytes(data[off..off + 8].try_into().unwrap())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_i64_le(buf: *const NamlBytes, offset: i64) -> i64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return 0; }
    i64::from_le_bytes(data[off..off + 8].try_into().unwrap())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_f32_be(buf: *const NamlBytes, offset: i64) -> f64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return 0.0; }
    f32::from_be_bytes(data[off..off + 4].try_into().unwrap()) as f64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_f32_le(buf: *const NamlBytes, offset: i64) -> f64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return 0.0; }
    f32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as f64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_f64_be(buf: *const NamlBytes, offset: i64) -> f64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return 0.0; }
    f64::from_be_bytes(data[off..off + 8].try_into().unwrap())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_read_f64_le(buf: *const NamlBytes, offset: i64) -> f64 {
    let data = buf_data(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return 0.0; }
    f64::from_le_bytes(data[off..off + 8].try_into().unwrap())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_u8(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off >= data.len() { return; }
    data[off] = value as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_i8(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off >= data.len() { return; }
    data[off] = (value as i8) as u8;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_u16_be(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 2 > data.len() { return; }
    let bytes = (value as u16).to_be_bytes();
    data[off..off + 2].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_u16_le(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 2 > data.len() { return; }
    let bytes = (value as u16).to_le_bytes();
    data[off..off + 2].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_i16_be(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 2 > data.len() { return; }
    let bytes = (value as i16).to_be_bytes();
    data[off..off + 2].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_i16_le(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 2 > data.len() { return; }
    let bytes = (value as i16).to_le_bytes();
    data[off..off + 2].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_u32_be(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return; }
    let bytes = (value as u32).to_be_bytes();
    data[off..off + 4].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_u32_le(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return; }
    let bytes = (value as u32).to_le_bytes();
    data[off..off + 4].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_i32_be(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return; }
    let bytes = (value as i32).to_be_bytes();
    data[off..off + 4].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_i32_le(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return; }
    let bytes = (value as i32).to_le_bytes();
    data[off..off + 4].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_u64_be(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return; }
    let bytes = (value as u64).to_be_bytes();
    data[off..off + 8].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_u64_le(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return; }
    let bytes = (value as u64).to_le_bytes();
    data[off..off + 8].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_i64_be(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return; }
    let bytes = value.to_be_bytes();
    data[off..off + 8].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_i64_le(buf: *mut NamlBytes, offset: i64, value: i64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return; }
    let bytes = value.to_le_bytes();
    data[off..off + 8].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_f32_be(buf: *mut NamlBytes, offset: i64, value: f64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return; }
    let bytes = (value as f32).to_be_bytes();
    data[off..off + 4].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_f32_le(buf: *mut NamlBytes, offset: i64, value: f64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 4 > data.len() { return; }
    let bytes = (value as f32).to_le_bytes();
    data[off..off + 4].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_f64_be(buf: *mut NamlBytes, offset: i64, value: f64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return; }
    let bytes = value.to_be_bytes();
    data[off..off + 8].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_write_f64_le(buf: *mut NamlBytes, offset: i64, value: f64) {
    let data = buf_data_mut(buf);
    let off = offset as usize;
    if off + 8 > data.len() { return; }
    let bytes = value.to_le_bytes();
    data[off..off + 8].copy_from_slice(&bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_alloc(capacity: i64) -> *mut NamlBytes {
    let cap = if capacity <= 0 { 8 } else { capacity as usize };
    let ptr = create_bytes_with_capacity(cap);
    unsafe { (*ptr).len = cap; }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_from_string(s: *const NamlString) -> *mut NamlBytes {
    if s.is_null() {
        return create_bytes_with_capacity(8);
    }
    unsafe {
        let len = (*s).len;
        let data = std::slice::from_raw_parts((*s).data.as_ptr(), len);
        create_bytes_from_slice(data)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_len(buf: *const NamlBytes) -> i64 {
    if buf.is_null() { return 0; }
    unsafe { (*buf).len as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_capacity(buf: *const NamlBytes) -> i64 {
    if buf.is_null() { return 0; }
    unsafe { (*buf).capacity as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_slice(
    buf: *const NamlBytes,
    start: i64,
    end: i64,
) -> *mut NamlBytes {
    if buf.is_null() { return create_bytes_with_capacity(8); }
    let data = buf_data(buf);
    let s = (start as usize).min(data.len());
    let e = (end as usize).min(data.len());
    if s >= e { return create_bytes_with_capacity(8); }
    create_bytes_from_slice(&data[s..e])
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_concat(
    a: *const NamlBytes,
    b: *const NamlBytes,
) -> *mut NamlBytes {
    let da = if a.is_null() { &[] as &[u8] } else { buf_data(a) };
    let db = if b.is_null() { &[] as &[u8] } else { buf_data(b) };
    let total = da.len() + db.len();
    let result = create_bytes_with_capacity(total);
    unsafe {
        (*result).len = total;
        if !da.is_empty() {
            std::ptr::copy_nonoverlapping(da.as_ptr(), (*result).data.as_mut_ptr(), da.len());
        }
        if !db.is_empty() {
            std::ptr::copy_nonoverlapping(
                db.as_ptr(),
                (*result).data.as_mut_ptr().add(da.len()),
                db.len(),
            );
        }
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_append(
    dst: *mut NamlBytes,
    src: *const NamlBytes,
) {
    if dst.is_null() || src.is_null() { return; }
    unsafe {
        let src_len = (*src).len;
        if src_len == 0 { return; }
        let dst_len = (*dst).len;
        let needed = dst_len + src_len;
        if needed > (*dst).capacity {
            let new_cap = needed.next_power_of_two().max(needed);
            let _ = realloc_bytes(dst, new_cap);
        }
        std::ptr::copy_nonoverlapping(
            (*src).data.as_ptr(),
            (*dst).data.as_mut_ptr().add(dst_len),
            src_len,
        );
        (*dst).len = needed;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_copy_within(
    buf: *mut NamlBytes,
    src_start: i64,
    src_end: i64,
    dst_start: i64,
) {
    if buf.is_null() { return; }
    unsafe {
        let len = (*buf).len;
        let ss = (src_start as usize).min(len);
        let se = (src_end as usize).min(len);
        let ds = (dst_start as usize).min(len);
        if ss >= se { return; }
        let copy_len = se - ss;
        if ds + copy_len > len { return; }
        let ptr = (*buf).data.as_mut_ptr();
        std::ptr::copy(ptr.add(ss), ptr.add(ds), copy_len);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_clear(buf: *mut NamlBytes) {
    if buf.is_null() { return; }
    unsafe {
        let len = (*buf).len;
        std::ptr::write_bytes((*buf).data.as_mut_ptr(), 0, len);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_resize(buf: *mut NamlBytes, new_len: i64) {
    if buf.is_null() { return; }
    let new_len = if new_len < 0 { 0usize } else { new_len as usize };
    unsafe {
        let old_len = (*buf).len;
        if new_len > (*buf).capacity {
            let new_cap = new_len.next_power_of_two().max(new_len);
            let _ = realloc_bytes(buf, new_cap);
        }
        if new_len > old_len {
            std::ptr::write_bytes((*buf).data.as_mut_ptr().add(old_len), 0, new_len - old_len);
        }
        (*buf).len = new_len;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_fill(buf: *mut NamlBytes, value: i64) {
    if buf.is_null() { return; }
    unsafe {
        let len = (*buf).len;
        std::ptr::write_bytes((*buf).data.as_mut_ptr(), value as u8, len);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_index_of(
    haystack: *const NamlBytes,
    needle: *const NamlBytes,
) -> i64 {
    if haystack.is_null() || needle.is_null() { return -1; }
    let h = buf_data(haystack);
    let n = buf_data(needle);
    if n.is_empty() { return 0; }
    if n.len() > h.len() { return -1; }
    for i in 0..=(h.len() - n.len()) {
        if h[i..i + n.len()] == *n {
            return i as i64;
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_contains(
    haystack: *const NamlBytes,
    needle: *const NamlBytes,
) -> i32 {
    if unsafe { naml_encoding_binary_index_of(haystack, needle) } >= 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_starts_with(
    buf: *const NamlBytes,
    prefix: *const NamlBytes,
) -> i32 {
    if buf.is_null() || prefix.is_null() { return 0; }
    let b = buf_data(buf);
    let p = buf_data(prefix);
    if p.len() > b.len() { return 0; }
    if b[..p.len()] == *p { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_ends_with(
    buf: *const NamlBytes,
    suffix: *const NamlBytes,
) -> i32 {
    if buf.is_null() || suffix.is_null() { return 0; }
    let b = buf_data(buf);
    let s = buf_data(suffix);
    if s.len() > b.len() { return 0; }
    if b[b.len() - s.len()..] == *s { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_binary_equals(
    a: *const NamlBytes,
    b: *const NamlBytes,
) -> i32 {
    if a.is_null() && b.is_null() { return 1; }
    if a.is_null() || b.is_null() { return 0; }
    let da = buf_data(a);
    let db = buf_data(b);
    if da == db { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write_u16_be() {
        unsafe {
            let buf = naml_encoding_binary_alloc(8);
            naml_encoding_binary_write_u16_be(buf, 0, 0x1234);
            assert_eq!(naml_encoding_binary_read_u16_be(buf, 0), 0x1234);
        }
    }

    #[test]
    fn test_read_write_u32_le() {
        unsafe {
            let buf = naml_encoding_binary_alloc(8);
            naml_encoding_binary_write_u32_le(buf, 0, 0xDEADBEEF_u32 as i64);
            assert_eq!(
                naml_encoding_binary_read_u32_le(buf, 0),
                0xDEADBEEF_u32 as i64
            );
        }
    }

    #[test]
    fn test_read_write_f64_be() {
        unsafe {
            let buf = naml_encoding_binary_alloc(8);
            naml_encoding_binary_write_f64_be(buf, 0, 3.14159265);
            let val = naml_encoding_binary_read_f64_be(buf, 0);
            assert!((val - 3.14159265).abs() < 1e-10);
        }
    }

    #[test]
    fn test_slice_and_concat() {
        unsafe {
            let buf = naml_encoding_binary_alloc(8);
            for i in 0..8 {
                naml_encoding_binary_write_u8(buf, i, i + 1);
            }
            let s = naml_encoding_binary_slice(buf, 2, 5);
            assert_eq!(naml_encoding_binary_len(s), 3);
            assert_eq!(naml_encoding_binary_read_u8(s, 0), 3);
            assert_eq!(naml_encoding_binary_read_u8(s, 1), 4);
            assert_eq!(naml_encoding_binary_read_u8(s, 2), 5);

            let a = naml_encoding_binary_alloc(2);
            naml_encoding_binary_write_u8(a, 0, 0xAA);
            naml_encoding_binary_write_u8(a, 1, 0xBB);
            let b = naml_encoding_binary_alloc(2);
            naml_encoding_binary_write_u8(b, 0, 0xCC);
            naml_encoding_binary_write_u8(b, 1, 0xDD);
            let c = naml_encoding_binary_concat(a, b);
            assert_eq!(naml_encoding_binary_len(c), 4);
            assert_eq!(naml_encoding_binary_read_u8(c, 0), 0xAA);
            assert_eq!(naml_encoding_binary_read_u8(c, 3), 0xDD);
        }
    }

    #[test]
    fn test_search_operations() {
        unsafe {
            let hay = naml_encoding_binary_alloc(5);
            for i in 0..5 {
                naml_encoding_binary_write_u8(hay, i, (i + 65) as i64);
            }
            let needle = naml_encoding_binary_alloc(2);
            naml_encoding_binary_write_u8(needle, 0, 67);
            naml_encoding_binary_write_u8(needle, 1, 68);

            assert_eq!(naml_encoding_binary_index_of(hay, needle), 2);
            assert_eq!(naml_encoding_binary_contains(hay, needle), 1);

            let prefix = naml_encoding_binary_alloc(2);
            naml_encoding_binary_write_u8(prefix, 0, 65);
            naml_encoding_binary_write_u8(prefix, 1, 66);
            assert_eq!(naml_encoding_binary_starts_with(hay, prefix), 1);

            let suffix = naml_encoding_binary_alloc(2);
            naml_encoding_binary_write_u8(suffix, 0, 68);
            naml_encoding_binary_write_u8(suffix, 1, 69);
            assert_eq!(naml_encoding_binary_ends_with(hay, suffix), 1);
        }
    }

    #[test]
    fn test_from_string() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"hello".as_ptr(), 5);
            let buf = naml_encoding_binary_from_string(s);
            assert_eq!(naml_encoding_binary_len(buf), 5);
            assert_eq!(naml_encoding_binary_read_u8(buf, 0), b'h' as i64);
            assert_eq!(naml_encoding_binary_read_u8(buf, 4), b'o' as i64);
        }
    }
}
