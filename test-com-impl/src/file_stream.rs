use com_impl::{Refcount, VTable};
use winapi::ctypes::c_void;
use winapi::shared::winerror::{ERROR_INVALID_INDEX, HRESULT, HRESULT_FROM_WIN32, S_OK};
use winapi::um::dwrite::{IDWriteFontFileStream, IDWriteFontFileStreamVtbl};
use wio::com::ComPtr;

#[repr(C)]
#[derive(ComImpl)]
pub struct FileStream {
    vtbl: VTable<IDWriteFontFileStreamVtbl>,
    refcount: Refcount,
    write_time: u64,
    file_data: Vec<u8>,
}

impl FileStream {
    // Todo: Use a wrapper type for the ComPtr
    pub fn new(write_time: u64, data: Vec<u8>) -> ComPtr<IDWriteFontFileStream> {
        let ptr = FileStream::create_raw(write_time, data);
        let ptr = ptr as *mut IDWriteFontFileStream;
        unsafe { ComPtr::from_raw(ptr) }
    }
}

#[com_impl]
unsafe impl IDWriteFontFileStream for FileStream {
    unsafe fn get_file_size(&self, size: *mut u64) -> HRESULT {
        *size = self.file_data.len() as u64;
        S_OK
    }

    unsafe fn get_last_write_time(&self, write_time: *mut u64) -> HRESULT {
        *write_time = self.write_time;
        S_OK
    }

    unsafe fn read_file_fragment(
        &self,
        start: *mut *const c_void,
        offset: u64,
        size: u64,
        ctx: *mut *mut c_void,
    ) -> HRESULT {
        if offset > std::isize::MAX as u64 || size > std::isize::MAX as u64 {
            return HRESULT_FROM_WIN32(ERROR_INVALID_INDEX);
        }

        let offset = offset as usize;
        let size = size as usize;

        if offset + size > self.file_data.len() {
            return HRESULT_FROM_WIN32(ERROR_INVALID_INDEX);
        }

        *start = self.file_data.as_ptr().offset(offset as isize) as *const c_void;
        *ctx = std::ptr::null_mut();

        S_OK
    }

    unsafe fn release_file_fragment(&self, _ctx: *mut c_void) {
        // Nothing to do
    }
}
