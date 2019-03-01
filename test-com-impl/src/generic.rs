use com_impl::{Refcount, VTable};
use winapi::um::unknwnbase::IUnknownVtbl;

#[repr(C)]
#[derive(com_impl::ComImpl)]
pub struct ComAny<T: Sized> {
    vtbl: VTable<IUnknownVtbl>,
    refcount: Refcount,
    pub data: T,
}

