// Copyright 2022 Ryan Seipp
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! First implementation of session buffer

use std::{
    alloc::{self, Layout},
    borrow::{Borrow, BorrowMut},
    io::Write,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::{copy, copy_nonoverlapping, NonNull},
};

/// A growable, contiguous byte buffer
#[derive(Debug)]
pub struct Buffer {
    ptr: NonNull<u8>,
    cap: usize,
    read_offset: usize,
    write_offset: usize,
    desired_capcity: usize,
    _marker: PhantomData<u8>,
}

impl Buffer {
    /// Creates a new Buffer with a capacity of 0
    pub fn new(desired_capacity: usize) -> Self {
        let mut result = Self {
            ptr: NonNull::dangling(),
            cap: 0, // `grow_to_capacity` will set this
            read_offset: 0,
            write_offset: 0,
            desired_capcity: desired_capacity.next_power_of_two(),
            _marker: PhantomData,
        };

        if desired_capacity > 0 {
            result.desired_capcity = 2;
            result.grow();
        }
        result
    }

    /// Reserves at least `capacity` new space
    pub fn reserve(&mut self, capacity: usize) {
        self.grow_to_capacity(self.cap + capacity);
    }

    /// Determines the capacity of elements available to be read
    pub fn remaining(&self) -> usize {
        self.write_offset - self.read_offset
    }

    /// Determines the capacity available for writing
    pub fn remaining_mut(&self) -> usize {
        self.cap - self.write_offset
    }

    /// The current write position
    pub fn write_pos(&self) -> usize {
        self.write_offset
    }

    /// Gets the current read position as a pointer. Use `remaining` to obtain the length
    pub fn read_ptr(&self) -> *mut u8 {
        // Safety: both `ptr` and the resulting ptr are guaranteed to be within the allocated
        // object due to checks when compacting and mutating offsets. The offset will not overflow
        // `isize::MAX` as we never allocate more than that.
        unsafe { self.ptr.as_ptr().add(self.read_offset) }
    }

    /// Gets the current write position as a pointer. Use `remaining_mut` to obtain the length
    pub fn write_ptr(&self) -> *mut u8 {
        // Safety: both `ptr` and the resulting ptr are guaranteed to be within the allocated
        // object due to checks when compacting and mutating offsets. The offset will not overflow
        // `isize::MAX` as we never allocate more than that.
        unsafe { self.ptr.as_ptr().add(self.write_offset) }
    }

    /// Mark a certain amount of bytes read from the buffer, freeing them for removal. If this is
    /// not called after reading from the buffer, the next read will receive the same data.
    pub fn mark_read(&mut self, amount: usize) {
        self.read_offset = self.write_offset.min(self.read_offset + amount);
        self.compact();
    }

    /// Mark a certain amount of bytes written to the buffer. If this is not called after writing,
    /// the next write will overwrite the previously written data.
    pub fn mark_written(&mut self, amount: usize) {
        self.write_offset = self.cap.min(self.write_offset + amount);
    }

    /// Grows the `Buffer`'s internal capacity.
    ///
    /// On initial allocation, sets capacity to desired_capacity. Afterwards doubles the capacity.
    fn grow(&mut self) {
        let new_cap = if self.cap == 0 {
            self.desired_capcity
        } else {
            2 * self.cap
        };

        self.grow_to_capacity(new_cap);
    }

    /// Grows to a specific capacity.
    ///
    /// It is not guaranteed that `self.cap == capacity` after this method. Capacity will be
    /// expanded to the next power of two that is equal to or greater than `capacity`.
    ///
    /// It is required that `capacity <= isize::MAX`
    ///
    /// Aborts the program if memory allocation fails due to out of memory error.
    fn grow_to_capacity(&mut self, capacity: usize) {
        assert!(capacity <= isize::MAX as usize);

        // limit new_cap to `isize::MAX` as `Layout::array` requires `cap <= isize::MAX`
        // will always land on power of two if the initial capacity is a power of two.
        let new_cap = capacity.next_power_of_two().min(isize::MAX as usize);

        let new_layout = Layout::array::<u8>(new_cap).unwrap();
        let new_ptr = if self.cap == 0 {
            // Safety: allocation failure is handled, layout is not zero-sized
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<u8>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;

            // Safety: allocation failure is handled, layout is not zero-sized
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // If allocation fails, `new_ptr` will be null, in which case we abort
        self.ptr = match NonNull::new(new_ptr as *mut u8) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }

    /// Reset the buffer to a clean initial state and frees excess capacity
    fn clear(&mut self) {
        self.read_offset = 0;
        self.write_offset = 0;

        if self.cap > self.desired_capcity {
            let layout = Layout::array::<u8>(self.cap).unwrap();

            // Safety: allocation failure is handled, layout is not zero-sized
            let new_ptr =
                unsafe { alloc::realloc(self.ptr.as_ptr(), layout, self.desired_capcity) };

            // If allocation fails, `new_ptr` will be null, in which case we abort
            self.ptr = match NonNull::new(new_ptr) {
                Some(p) => p,
                None => alloc::handle_alloc_error(layout),
            };
            self.cap = self.desired_capcity;
        }
    }

    /// Prevent extra allocations and utilize excess space at the beginning of the buffer.
    ///
    /// Only causes an allocation if `self.cap > self.desired_capcity`
    fn compact(&mut self) {
        // buffer is empty, reset to clean state
        if self.remaining() == 0 {
            self.clear();
            return;
        }

        if self.cap == self.desired_capcity {
            return;
        }

        // If read_offset is already over desired capacity, we have a significant amount of unused
        // space, and further writes are likely to cause allocation. Copy read_offset to the
        // beginning of the buffer to clear space for further writes.
        if self.read_offset > self.desired_capcity {
            if self.remaining() < self.read_offset {
                // Safety: `read_ptr()` and `ptr` are valid for `remaining()` and are aligned to
                // u8. Copying to the beginning of the buffer will not overlap with `read_ptr` as
                // the read region is smaller than the offset.
                unsafe { copy_nonoverlapping(self.read_ptr(), self.ptr.as_ptr(), self.remaining()) }
            } else {
                // Safety: `read_ptr()` and `ptr` are valid for `remaining()` and are aligned to u8
                unsafe { copy(self.read_ptr(), self.ptr.as_ptr(), self.remaining()) }
            }

            self.write_offset = self.remaining();
            self.read_offset = 0;
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.cap != 0 {
            // u8 does not require drop, so simply deallocate
            let layout = Layout::array::<u8>(self.cap).unwrap();
            unsafe { alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout) }
        }
    }
}

impl Borrow<[u8]> for Buffer {
    fn borrow(&self) -> &[u8] {
        // Safety: `self.read_ptr` points to a single correctly allocated, contiguous region of
        // memory. It's data is initialized, aligned for `u8`, and cannot be null. The pointer
        // will be valid for the lifetime of this slice as a mutable borrow cannot be taken while
        // this immutable borrow is held. The slice will not be larger than `isize::MAX` as we
        // never allocate more than that much memory.
        unsafe { std::slice::from_raw_parts(self.read_ptr(), self.remaining()) }
    }
}

impl BorrowMut<[u8]> for Buffer {
    fn borrow_mut(&mut self) -> &mut [u8] {
        // Safety: `self.write_ptr` points to a single correctly allocated, contiguous region of
        // memory. It's data is initialized, aligned for `u8`, and cannot be null. The pointer
        // will be valid for the lifetime of this slice as another mutable borrow cannot be taken
        // while this borrow is held. The slice will not be larger than `isize::MAX` as we never
        // allocate more than that much memory.
        unsafe { std::slice::from_raw_parts_mut(self.write_ptr(), self.remaining_mut()) }
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow_mut()
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.remaining_mut() < buf.len() {
            self.reserve(buf.len());
        }
        self.deref_mut()[0..buf.len()].clone_from_slice(buf);
        self.mark_written(buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
