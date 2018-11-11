#![recursion_limit = "1024"]

//! Implements a COM Object struct with automatic reference counting and implements
//! IUnknown for you. This covers the most common use cases of creating COM objects
//! from Rust. Supports generic parameters!
//! 
//! ```
//! #[macro_use]
//! extern crate derive_com_impl;
//! 
//! extern crate com_impl;
//! extern crate winapi;
//! extern crate wio;
//! 
//! use com_impl::{VTable, Refcount};
//! use winapi::ctypes::c_void;
//! use winapi::shared::winerror::{ERROR_INVALID_INDEX, HRESULT, HRESULT_FROM_WIN32, S_OK};
//! use winapi::um::dwrite::{IDWriteFontFileStream, IDWriteFontFileStreamVtbl};
//! use wio::com::ComPtr;
//! 
//! #[repr(C)]
//! #[derive(ComImpl)]
//! #[interfaces(IDWriteFontFileStream)]
//! pub struct FileStream {
//!     vtbl: VTable<IDWriteFontFileStreamVtbl>,
//!     refcount: Refcount,
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
//! #[com_impl]
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

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;
use syn::DeriveInput;
use syn::{AttributeArgs, Item};

mod derive;
mod com_impl;

#[proc_macro_derive(ComImpl, attributes(interfaces))]
/// `#[derive(ComImpl)]`
/// 
/// Automatically implements reference counting for your COM object, creating a pointer via
/// `Box::into_raw` and deallocating with `Box::from_raw`. A private inherent method named
/// `create_raw` is added to your type that takes all of your struct members except the vtable
/// and refcount as parameters in declaration order.
/// 
/// ### Additional attributes:
/// 
/// `#[interfaces(ISome, IThing)]`
/// 
/// - Specifies the COM interfaces that this type should respond to in QueryInterface. IUnknown
///   is included implicitly. If this attribute is not specified it will be assumed that the only
///   types responded to are IUnknown and the type specified in the VTable.
pub fn derive_com_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    derive::expand_derive_com_impl(&input)
        .unwrap_or_else(compile_error)
        .into()
}

#[proc_macro_attribute]
/// `#[com_impl]`
/// 
/// Generates a VTable for the functions implemented in the `impl` block this attribute is
/// applied to. Method names by default are mapped from snake_case to PascalCase to determine
/// their winapi names. If you would like to override the name, you can specify an attribute
/// on the method (see below).
/// 
/// For the general syntax see the example in the crate root.
/// 
/// ### Additional parameters
/// 
/// `#[com_impl(no_parent)]`
/// 
/// - Specifies that the vtable being implemented here does not have a `parent` member. These
///   are very rare, but include IUnknown.
pub fn com_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let item = parse_macro_input!(item as Item);

    com_impl::expand_com_impl(&args, &item)
        .unwrap_or_else(compile_error)
        .into()
}

fn compile_error(message: String) -> proc_macro2::TokenStream {
    quote! {
        compile_error!(#message);
    }
}
