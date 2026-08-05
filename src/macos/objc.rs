use crate::ffi::*;
use std::mem::transmute;
use std::os::raw::c_void;

// Basic msgSend wrappers
pub unsafe fn msg_send_id(obj: id, sel: SEL) -> id {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL) -> id = transmute(objc_msgSend as *const c_void);
        func(obj, sel)
    }
}

pub unsafe fn msg_send_point(obj: id, sel: SEL) -> NSPoint {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL) -> NSPoint =
            transmute(objc_msgSend as *const c_void);
        func(obj, sel)
    }
}

pub unsafe fn msg_send_point_point(obj: id, sel: SEL, arg1: NSPoint) -> NSPoint {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL, NSPoint) -> NSPoint =
            transmute(objc_msgSend as *const c_void);
        func(obj, sel, arg1)
    }
}

pub unsafe fn msg_send_id_id(obj: id, sel: SEL, arg1: id) -> id {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL, id) -> id =
            transmute(objc_msgSend as *const c_void);
        func(obj, sel, arg1)
    }
}

pub unsafe fn msg_send_id_id_void(obj: id, sel: SEL, arg1: id) {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL, id) = transmute(objc_msgSend as *const c_void);
        func(obj, sel, arg1)
    }
}

pub unsafe fn msg_send_id_bool_void(obj: id, sel: SEL, arg1: BOOL) {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL, BOOL) = transmute(objc_msgSend as *const c_void);
        func(obj, sel, arg1)
    }
}

pub unsafe fn msg_send_id_usize_void(obj: id, sel: SEL, arg1: usize) {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL, usize) = transmute(objc_msgSend as *const c_void);
        func(obj, sel, arg1)
    }
}

pub unsafe fn msg_send_void(obj: id, sel: SEL) {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL) = transmute(objc_msgSend as *const c_void);
        func(obj, sel)
    }
}

// Convert a Rust string to a Cocoa NSString (retaining pointer)
pub unsafe fn nsstring(s: &str) -> id {
    unsafe {
        let nsstring_class = objc_getClass(c"NSString".as_ptr() as *const _);
        let alloc_sel = sel_registerName(c"alloc".as_ptr() as *const _);
        let init_sel = sel_registerName(c"initWithBytes:length:encoding:".as_ptr() as *const _);

        let allocated = msg_send_id(nsstring_class, alloc_sel);
        let init_func: unsafe extern "C" fn(id, SEL, *const c_void, usize, usize) -> id =
            transmute(objc_msgSend as *const c_void);
        init_func(allocated, init_sel, s.as_ptr() as *const c_void, s.len(), 4) // 4 is NSUTF8StringEncoding
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn msg_send_rect(obj: id, sel: SEL) -> NSRect {
    unsafe {
        let mut rect = std::mem::zeroed();
        let func: unsafe extern "C" fn(*mut NSRect, id, SEL) =
            transmute(objc_msgSend_stret as *const c_void);
        func(&mut rect, obj, sel);
        rect
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn msg_send_rect(obj: id, sel: SEL) -> NSRect {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL) -> NSRect =
            transmute(objc_msgSend as *const c_void);
        func(obj, sel)
    }
}
