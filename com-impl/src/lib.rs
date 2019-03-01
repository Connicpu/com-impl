#![cfg(windows)]

//! Implements a COM Object struct with automatic reference counting and implements
//! IUnknown for you. This covers the most common use cases of creating COM objects
//! from Rust. Supports generic parameters!
//!
//! ```
//! use winapi::ctypes::c_void;
//! use winapi::shared::winerror::{ERROR_INVALID_INDEX, HRESULT, HRESULT_FROM_WIN32, S_OK};
//! use winapi::um::dwrite::{IDWriteFontFileStream, IDWriteFontFileStreamVtbl};
//! use wio::com::ComPtr;
//!
//! #[repr(C)]
//! #[derive(com_impl::ComImpl)]
//! #[interfaces(IDWriteFontFileStream)]
//! pub struct FileStream {
//!     vtbl: com_impl::VTable<IDWriteFontFileStreamVtbl>,
//!     refcount: com_impl::Refcount,
//!     write_time: u64,
//!     file_data: Vec<u8>,
//! }
//!
//! impl FileStream {
//!     pub fn new(write_time: u64, data: Vec<u8>) -> ComPtr<IDWriteFontFileStream> {
//!         let ptr = FileStream::create_raw(write_time, data);
//!         let ptr = ptr as *mut IDWriteFontFileStream;
//!         unsafe { ComPtr::from_raw(ptr) }
//!     }
//! }
//!
//! #[com_impl::com_impl]
//! unsafe impl IDWriteFontFileStream for FileStream {
//!     unsafe fn get_file_size(&self, size: *mut u64) -> HRESULT {
//!         *size = self.file_data.len() as u64;
//!         S_OK
//!     }
//!
//!     unsafe fn get_last_write_time(&self, write_time: *mut u64) -> HRESULT {
//!         *write_time = self.write_time;
//!         S_OK
//!     }
//!
//!     unsafe fn read_file_fragment(
//!         &self,
//!         start: *mut *const c_void,
//!         offset: u64,
//!         size: u64,
//!         ctx: *mut *mut c_void,
//!     ) -> HRESULT {
//!         if offset > std::isize::MAX as u64 || size > std::isize::MAX as u64 {
//!             return HRESULT_FROM_WIN32(ERROR_INVALID_INDEX);
//!         }
//!
//!         let offset = offset as usize;
//!         let size = size as usize;
//!
//!         if offset + size > self.file_data.len() {
//!             return HRESULT_FROM_WIN32(ERROR_INVALID_INDEX);
//!         }
//!
//!         *start = self.file_data.as_ptr().offset(offset as isize) as *const c_void;
//!         *ctx = std::ptr::null_mut();
//!
//!         S_OK
//!     }
//!
//!     unsafe fn release_file_fragment(&self, _ctx: *mut c_void) {
//!         // Nothing to do
//!     }
//! }
//!
//! fn main() {
//!     let ptr = FileStream::new(100, vec![0xDE, 0xAF, 0x00, 0xF0, 0x01]);
//!
//!     // Do things with ptr
//! }
//! ```

extern crate derive_com_impl;
extern crate winapi;

use std::sync::atomic::{AtomicUsize, Ordering};

pub use derive_com_impl::{com_impl, ComImpl};

#[repr(transparent)]
/// Wrapper for the C++ VTable member of a COM object.
///
/// When you're using `#[derive(ComImpl)]`, this should be the first member of your struct.
pub struct VTable<T> {
    pub ptr: *const T,
}

impl<T> VTable<T> {
    pub fn new(ptr: &'static T) -> Self {
        VTable { ptr }
    }
}

impl<T> std::fmt::Debug for VTable<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_tuple("VTable").field(&self.ptr).finish()
    }
}

/// Trait that allows accessing the VTable for all of the COM interfaces your object
/// implements.
pub unsafe trait BuildVTable<T: 'static> {
    const VTBL: T;
    fn static_vtable() -> VTable<T>;
}

#[derive(Debug)]
/// Refcounter object for automatic COM Object implementations. Atomically keeps track of
/// the reference count so that the implementation of IUnknown can properly deallocate
/// the object when all reference counts are gone.
pub struct Refcount {
    count: AtomicUsize,
}

impl Default for Refcount {
    fn default() -> Self {
        Refcount {
            count: AtomicUsize::new(1),
        }
    }
}

impl Refcount {
    #[inline]
    /// `fetch_add(1, Acquire) + 1`
    pub unsafe fn add_ref(&self) -> u32 {
        self.count.fetch_add(1, Ordering::Acquire) as u32 + 1
    }

    #[inline]
    /// `fetch_sub(1, Release) - 1`
    pub unsafe fn release(&self) -> u32 {
        self.count.fetch_sub(1, Ordering::Release) as u32 - 1
    }
}
