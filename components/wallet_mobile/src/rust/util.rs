use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[inline(always)]
pub fn c_char_to_string(cchar: *const c_char) -> String {
    let c_str = unsafe { CStr::from_ptr(cchar) };
    c_str.to_str().unwrap_or("").to_string()
}

#[inline(always)]
pub fn string_to_c_char(r_string: String) -> *mut c_char {
    let c_str = CString::new(r_string).expect("CString::new failed");
    c_str.into_raw()
}
