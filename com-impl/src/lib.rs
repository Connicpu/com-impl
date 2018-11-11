#![cfg(windows)]

extern crate winapi;

use std::sync::atomic::{AtomicUsize, Ordering};

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
