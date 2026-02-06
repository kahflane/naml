///
/// Thread-Local Arena Allocator
///
/// Fast bump-pointer allocation with free lists for memory reuse.
/// Designed for high-frequency small allocations like structs.
///
/// Size classes: 32, 48, 64, 80, 96, 128, 192, 256, 512 bytes
/// Larger allocations fall back to system malloc.
///

use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::cell::Cell;

const ARENA_SIZE: usize = 4 * 1024 * 1024;
const MAX_ARENA_ALLOC: usize = 512;
const NUM_SIZE_CLASSES: usize = 9;

#[inline(always)]
fn size_class_index(size: usize) -> usize {
    if size <= 32 { 0 }
    else if size <= 48 { 1 }
    else if size <= 64 { 2 }
    else if size <= 80 { 3 }
    else if size <= 96 { 4 }
    else if size <= 128 { 5 }
    else if size <= 192 { 6 }
    else if size <= 256 { 7 }
    else { 8 }
}

#[inline(always)]
fn size_class_size(index: usize) -> usize {
    match index {
        0 => 32,
        1 => 48,
        2 => 64,
        3 => 80,
        4 => 96,
        5 => 128,
        6 => 192,
        7 => 256,
        _ => 512,
    }
}

#[repr(C)]
struct FreeNode {
    next: *mut FreeNode,
}

#[repr(C)]
struct ArenaState {
    bump_ptr: *mut u8,
    bump_end: *mut u8,
    blocks: *mut ArenaBlock,
    free_lists: [*mut FreeNode; NUM_SIZE_CLASSES],
}

#[repr(C)]
struct ArenaBlock {
    data: *mut u8,
    next: *mut ArenaBlock,
}

impl ArenaState {
    fn new() -> Self {
        let (data, end) = Self::alloc_block();
        let block = unsafe {
            let block_layout = Layout::new::<ArenaBlock>();
            let block = alloc(block_layout) as *mut ArenaBlock;
            (*block).data = data;
            (*block).next = ptr::null_mut();
            block
        };

        Self {
            bump_ptr: data,
            bump_end: end,
            blocks: block,
            free_lists: [ptr::null_mut(); NUM_SIZE_CLASSES],
        }
    }

    fn alloc_block() -> (*mut u8, *mut u8) {
        unsafe {
            let layout = Layout::from_size_align(ARENA_SIZE, 16).unwrap();
            let data = alloc(layout);
            if data.is_null() {
                panic!("Failed to allocate arena block");
            }
            (data, data.add(ARENA_SIZE))
        }
    }

    #[inline(always)]
    unsafe fn alloc(&mut self, size: usize) -> *mut u8 {
        let class_idx = size_class_index(size);
        let class_size = size_class_size(class_idx);

        let free_head = self.free_lists[class_idx];
        if !free_head.is_null() {
            self.free_lists[class_idx] = (*free_head).next;
            return free_head as *mut u8;
        }

        let aligned_size = (class_size + 7) & !7;
        let new_ptr = self.bump_ptr.add(aligned_size);

        if new_ptr <= self.bump_end {
            let result = self.bump_ptr;
            self.bump_ptr = new_ptr;
            return result;
        }

        self.alloc_slow(class_size)
    }

    #[cold]
    #[inline(never)]
    unsafe fn alloc_slow(&mut self, size: usize) -> *mut u8 {
        let (data, end) = Self::alloc_block();

        let block_layout = Layout::new::<ArenaBlock>();
        let new_block = alloc(block_layout) as *mut ArenaBlock;
        (*new_block).data = data;
        (*new_block).next = self.blocks;
        self.blocks = new_block;

        self.bump_ptr = data;
        self.bump_end = end;

        let aligned_size = (size + 7) & !7;
        let result = self.bump_ptr;
        self.bump_ptr = self.bump_ptr.add(aligned_size);
        result
    }

    #[inline(always)]
    unsafe fn free(&mut self, ptr: *mut u8, size: usize) {
        let class_idx = size_class_index(size);
        let node = ptr as *mut FreeNode;
        (*node).next = self.free_lists[class_idx];
        self.free_lists[class_idx] = node;
    }
}

impl Drop for ArenaState {
    fn drop(&mut self) {
        unsafe {
            let mut block = self.blocks;
            while !block.is_null() {
                let next = (*block).next;
                let data_layout = Layout::from_size_align(ARENA_SIZE, 16).unwrap();
                dealloc((*block).data, data_layout);
                let block_layout = Layout::new::<ArenaBlock>();
                dealloc(block as *mut u8, block_layout);
                block = next;
            }
        }
    }
}

thread_local! {
    static ARENA_PTR: Cell<*mut ArenaState> = const { Cell::new(ptr::null_mut()) };
}

#[inline(always)]
fn get_arena() -> *mut ArenaState {
    ARENA_PTR.with(|cell| {
        let ptr = cell.get();
        if ptr.is_null() {
            let arena = Box::into_raw(Box::new(ArenaState::new()));
            cell.set(arena);
            arena
        } else {
            ptr
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_arena_alloc(size: usize) -> *mut u8 {
    arena_alloc(size)
}

#[inline(always)]
pub fn arena_alloc(size: usize) -> *mut u8 {
    if size > MAX_ARENA_ALLOC {
        unsafe {
            let layout = Layout::from_size_align(size, 8).unwrap();
            return alloc(layout);
        }
    }

    unsafe {
        let arena = get_arena();
        (*arena).alloc(size)
    }
}

#[inline(always)]
pub fn arena_free(ptr: *mut u8, size: usize) {
    if ptr.is_null() {
        return;
    }

    if size > MAX_ARENA_ALLOC {
        unsafe {
            let layout = Layout::from_size_align(size, 8).unwrap();
            dealloc(ptr, layout);
        }
        return;
    }

    unsafe {
        let arena = get_arena();
        (*arena).free(ptr, size);
    }
}

pub fn struct_alloc_size(field_count: u32) -> usize {
    std::mem::size_of::<crate::value::NamlStruct>() + (field_count as usize) * std::mem::size_of::<i64>()
}

pub fn string_alloc_size(len: usize) -> usize {
    std::mem::size_of::<crate::value::NamlString>() + len
}

pub const MAX_SMALL_STRING: usize = 224;

#[inline(always)]
pub fn is_small_string(len: usize) -> bool {
    string_alloc_size(len) <= MAX_ARENA_ALLOC
}

#[inline(always)]
pub fn is_small_closure(size: usize) -> bool {
    size <= MAX_ARENA_ALLOC
}

pub const ARRAY_HEADER_SIZE: usize = 40;
