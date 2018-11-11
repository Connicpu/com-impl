use com_impl::{Refcount, VTable};
use winapi::um::unknwnbase::IUnknownVtbl;

#[repr(C)]
#[derive(ComImpl)]
pub struct ComAny<T: Sized> {
    vtbl: VTable<IUnknownVtbl>,
    refcount: Refcount,
    pub data: T,
}

// Expanded:
pub mod generic {
    use com_impl::{Refcount, VTable};
    use winapi::um::unknwnbase::IUnknownVtbl;
    #[repr(C)]
    pub struct ComAny<T: Sized> {
        vtbl: VTable<IUnknownVtbl>,
        refcount: Refcount,
        pub data: T,
    }
    impl<T: Sized> ComAny<T> {
        fn create_raw(data: T) -> *mut Self {
            Box::into_raw(Box::new(ComAny {
                vtbl: <Self as com_impl::BuildVTable<_>>::static_vtable(),
                refcount: Default::default(),
                data: data,
            }))
        }
    }
    unsafe impl<T: Sized> com_impl::BuildVTable<winapi::um::unknwnbase::IUnknownVtbl> for ComAny<T> {
        const VTBL: winapi::um::unknwnbase::IUnknownVtbl = winapi::um::unknwnbase::IUnknownVtbl {
            AddRef: Self::__com_impl__IUnknown__AddRef,
            Release: Self::__com_impl__IUnknown__Release,
            QueryInterface: Self::__com_impl__IUnknown__QueryInterface,
        };
        fn static_vtable() -> com_impl::VTable<winapi::um::unknwnbase::IUnknownVtbl> {
            com_impl::VTable::new(&Self::VTBL)
        }
    }
    #[allow(non_snake_case)]
    impl<T: Sized> ComAny<T> {
        #[inline(never)]
        unsafe extern "system" fn __com_impl__IUnknown__AddRef(
            this: *mut winapi::um::unknwnbase::IUnknown,
        ) -> u32 {
            let this = &*(this as *const Self);
            this.refcount.add_ref()
        }
        #[inline(never)]
        unsafe extern "system" fn __com_impl__IUnknown__Release(
            this: *mut winapi::um::unknwnbase::IUnknown,
        ) -> u32 {
            let ptr = this as *mut Self;
            let count = (*ptr).refcount.release();
            if count == 0 {
                Box::from_raw(ptr);
            }
            count
        }
        #[inline(never)]
        unsafe extern "system" fn __com_impl__IUnknown__QueryInterface(
            this: *mut winapi::um::unknwnbase::IUnknown,
            riid: *const winapi::shared::guiddef::IID,
            ppv: *mut *mut winapi::ctypes::c_void,
        ) -> winapi::shared::winerror::HRESULT {
            if ppv.is_null() {
                return winapi::shared::winerror::E_POINTER;
            }
            if winapi::shared::guiddef::IsEqualIID(
                &*riid,
                &<winapi::um::unknwnbase::IUnknown as winapi::Interface>::uuidof(),
            ) {
                *ppv = this as *mut winapi::ctypes::c_void;
                winapi::shared::winerror::S_OK
            } else {
                *ppv = std::ptr::null_mut();
                winapi::shared::winerror::E_NOINTERFACE
            }
        }
    }
}
