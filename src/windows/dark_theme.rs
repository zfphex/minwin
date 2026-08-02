use crate::*;

///Only works on Windows 1809 and above.
pub unsafe fn set_dark_theme(hwnd: isize) {
    const WCA_USEDARKMODECOLORS: u32 = 26;
    const DARK_MODE: i32 = 1;

    #[repr(C)]
    struct WINDOWCOMPOSITIONATTRIBDATA {
        attrib: u32,
        data: *mut std::ffi::c_void,
        size: usize,
    }

    #[repr(C)]
    struct OSVERSIONINFOW {
        pub dw_osversion_info_size: u32,
        pub dw_major_version: u32,
        pub dw_minor_version: u32,
        pub dw_build_number: u32,
        pub dw_platform_id: u32,
        pub sz_csdversion: [u16; 128],
    }

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn RtlGetVersion(version: *mut OSVERSIONINFOW) -> i32;
    }

    let mut v: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
    let status = unsafe { RtlGetVersion(&mut v) };

    //Check if this version of windows supports `SetWindowCompositionAttribute`.
    if v.dw_build_number < 17763 || status < 0 {
        // panic!("Window version must be 1809 or above.");
        return;
    }

    let user32 = unsafe { LoadLibraryA(c"user32.dll".as_ptr() as *const i8) };
    let proc = unsafe {
        GetProcAddress(
            user32,
            c"SetWindowCompositionAttribute".as_ptr() as *const i8,
        )
    };
    let SetWindow: fn(isize, *mut WINDOWCOMPOSITIONATTRIBDATA) -> i32 =
        unsafe { std::mem::transmute(proc) };

    //This must be mutable.
    let mut theme: i32 = DARK_MODE;
    let mut data = WINDOWCOMPOSITIONATTRIBDATA {
        attrib: WCA_USEDARKMODECOLORS,
        data: &mut theme as *mut i32 as _,
        size: size_of::<i32>(),
    };

    assert!(SetWindow(hwnd, &mut data) != 0)
}
