#![cfg(windows)]

extern crate winapi;

use std::sync::atomic::{AtomicUsize, Ordering};

#[repr(transparent)]
pub struct VTable<T> {
    pub ptr: *const T,
}

impl<T> VTable<T> {
    pub fn new(ptr: &'static T) -> Self {
        VTable { ptr }
    }
}

pub unsafe trait BuildVTable<T: 'static> {
    const VTBL: T;
    fn static_vtable() -> VTable<T>;
}

pub struct Refcount {
    count: AtomicUsize,
}

impl Default for Refcount {
    fn default() -> Self {
        Refcount {
            count: AtomicUsize::new(1)
        }
    }
}

impl Refcount {
    #[inline]
    pub unsafe fn add_ref(&self) -> u32 {
        self.count.fetch_add(1, Ordering::Acquire) as u32 + 1
    }

    #[inline]
    pub unsafe fn release(&self) -> u32 {
        self.count.fetch_sub(1, Ordering::Release) as u32 - 1
    }
}
