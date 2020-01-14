use super::{
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, UCString},
};
use digest::{FixedOutput, Input, Reset};
use libc::{c_char, c_void, size_t};
use qiniu_ng::utils::etag;
use std::{
    mem::{replace, transmute},
    ptr::{copy_nonoverlapping, null_mut},
    slice::from_raw_parts,
};

pub const ETAG_SIZE: size_t = 28;

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_file_path(
    path: *const qiniu_ng_char_t,
    result: *mut c_char,
    error: *mut qiniu_ng_err_t,
) -> bool {
    match etag::from_file(unsafe { UCString::from_ptr(path) }.into_path_buf()) {
        Ok(etag_string) => {
            let etag_bytes = etag_string.as_bytes();
            unsafe { copy_nonoverlapping(etag_bytes.as_ptr(), result.cast(), etag_bytes.len()) };
            true
        }
        Err(ref err) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = err.into();
            }
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_buffer(buffer: *const c_void, buffer_len: size_t, result: *mut c_char) {
    unsafe {
        let e = etag::from_bytes(from_raw_parts(buffer.cast(), buffer_len));
        let e = e.as_bytes();
        copy_nonoverlapping(e.as_ptr(), result.cast(), e.len());
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_etag_t(*mut c_void);

impl Default for qiniu_ng_etag_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_etag_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_etag_t> for Option<Box<etag::Etag>> {
    fn from(etag: qiniu_ng_etag_t) -> Self {
        if etag.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(etag)) })
        }
    }
}

impl From<Box<etag::Etag>> for qiniu_ng_etag_t {
    fn from(etag: Box<etag::Etag>) -> Self {
        unsafe { transmute(Box::into_raw(etag)) }
    }
}

impl From<Option<Box<etag::Etag>>> for qiniu_ng_etag_t {
    fn from(etag: Option<Box<etag::Etag>>) -> Self {
        etag.map(|etag| etag.into()).unwrap_or_default()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_new() -> qiniu_ng_etag_t {
    Box::new(etag::new()).into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_update(etag: qiniu_ng_etag_t, data: *mut c_void, data_len: size_t) {
    let mut etag = Option::<Box<etag::Etag>>::from(etag).unwrap();
    etag.input(unsafe { from_raw_parts(data.cast(), data_len) });
    let _ = qiniu_ng_etag_t::from(etag);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_result(etag: qiniu_ng_etag_t, result_ptr: *mut c_void) {
    let mut etag = Option::<Box<etag::Etag>>::from(etag).unwrap();
    let result = replace(&mut *etag, etag::new()).fixed_result();
    unsafe { copy_nonoverlapping(result.as_ptr(), result_ptr.cast(), ETAG_SIZE) };
    let _ = qiniu_ng_etag_t::from(etag);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_reset(etag: qiniu_ng_etag_t) {
    let mut etag = Option::<Box<etag::Etag>>::from(etag).unwrap();
    etag.reset();
    let _ = qiniu_ng_etag_t::from(etag);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_free(etag: *mut qiniu_ng_etag_t) {
    if let Some(etag) = unsafe { etag.as_mut() } {
        let _ = Option::<Box<etag::Etag>>::from(*etag);
        *etag = qiniu_ng_etag_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_is_freed(etag: qiniu_ng_etag_t) -> bool {
    etag.is_null()
}
