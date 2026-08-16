#![allow(
    non_snake_case,
    static_mut_refs,
    non_camel_case_types,
    unused_variables
)]

use crate::*;
pub use std::ffi::c_void;
pub use std::ptr::{null, null_mut};

mod clipboard;
mod constants;
mod cursor;
mod dark_theme;
mod gdi;
mod input;
mod window;

pub use clipboard::*;
pub use constants::*;
pub use cursor::*;
pub use dark_theme::*;
pub use gdi::*;
pub use input::*;
pub use window::*;

pub type BYTE = u8;
pub type HDC = *mut c_void;
pub type HANDLE = *mut c_void;
pub type HWND = isize;
pub type HGLRC = *mut c_void;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type HRESULT = i32;
pub type WORD = u16;
pub type DWORD = u32;
pub type BOOL = i32;
pub type UINT = u32;
pub type LONG = i32;
pub type LPCSTR = *const i8;
pub type LPCWSTR = *const u16;
pub type LPWSTR = *mut u16;

#[link(name = "dwmapi")]
unsafe extern "system" {
    pub fn DwmFlush() -> i32;
    pub fn DwmGetColorizationColor(pcrColorization: *mut u32, pfOpaqueBlend: *mut i32) -> i32;
    pub fn DwmExtendFrameIntoClientArea(hWnd: isize, pMarInset: *const MARGINS) -> i32;
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct MARGINS {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct NCCALCSIZE_PARAMS {
    pub rgrc: [RECT; 3],
    pub lppos: *mut c_void,
}

#[link(name = "Opengl32")]
unsafe extern "system" {
    pub fn wglCreateContext(hdc: *mut std::ffi::c_void) -> HGLRC;
    pub fn wglDeleteContext(hglrc: HGLRC) -> i32;
    pub fn wglMakeCurrent(hdc: *mut std::ffi::c_void, hglrc: HGLRC) -> i32;
    pub fn wglGetProcAddress(name: *const i8) -> *const c_void;
}

#[link(name = "shell32")]
unsafe extern "system" {
    pub fn DragAcceptFiles(hWnd: HWND, fAccept: BOOL);
    pub fn DragQueryFileW(hDrop: HANDLE, iFile: UINT, lpszFile: LPWSTR, cch: UINT) -> UINT;
    pub fn DragFinish(hDrop: HANDLE);
}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn GetMessageTime() -> i32;
    pub fn RegisterClassA(lpwndclass: *const WNDCLASSA) -> u16;
    pub fn DispatchMessageA(lpMsg: *const MSG) -> isize;
    pub fn TranslateMessage(lpMsg: *const MSG) -> i32;
    pub fn GetLastError() -> u32;
    pub fn GetProcAddress(hModule: *mut c_void, lpProcName: *const i8) -> *mut c_void;
    pub fn LoadLibraryA(lpFileName: *const i8) -> *mut c_void;
    pub fn GetDC(hwnd: isize) -> *mut c_void;
    pub fn GetPixel(hdc: *mut c_void, x: i32, y: i32) -> u32;
    pub fn GetFocus() -> HWND;
    pub fn WindowFromPoint(point: POINT) -> HWND;
    pub fn GetDeviceCaps(hdc: *mut c_void, index: i32) -> i32;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;
    pub fn LoadCursorW(hInstance: *mut c_void, lpCursorName: *const u16) -> *mut c_void;
    pub fn GetAsyncKeyState(vKey: i32) -> i16;
    pub fn GetKeyState(nVirtKey: i32) -> i16;
    pub fn GetCursorPos(point: *mut POINT) -> i32;
    pub fn GetPhysicalCursorPos(point: *mut POINT) -> i32;
    pub fn DefWindowProcA(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
    pub fn GetWindow(hwnd: isize, uCmd: u32) -> isize;
    pub fn DestroyWindow(hwnd: isize) -> i32;
    pub fn GetForegroundWindow() -> isize;
    pub fn GetWindowLongPtrW(hwnd: isize, nIndex: i32) -> isize;
    pub fn SetWindowLongPtrW(hwnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
    pub fn GetWindowLongPtrA(hwnd: isize, nIndex: i32) -> isize;
    pub fn SetWindowLongPtrA(hwnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
    pub fn GetWindowLongA(hwnd: isize, nIndex: i32) -> LONG;
    pub fn SetWindowLongA(hwnd: isize, nIndex: i32, dwNewLong: LONG) -> LONG;
    pub fn ShowWindow(hwnd: isize, nCmdShow: i32) -> BOOL;
    pub fn IsZoomed(hwnd: isize) -> BOOL;
    pub fn GetWindowInfo(hwnd: isize, pwi: *mut WindowInfo) -> i32;
    pub fn AdjustWindowRectEx(lpRect: *mut RECT, dwStyle: u32, bMenu: i32, dwExStyle: u32) -> i32;
    pub fn GetDesktopWindow() -> isize;
    pub fn GetWindowRect(hwnd: isize, lpRect: *mut RECT) -> i32;
    pub fn GetClientRect(hwnd: isize, lpRect: *mut RECT) -> i32;
    pub fn ClientToScreen(hwnd: isize, lpPoint: *mut POINT) -> i32;
    pub fn ScreenToClient(hwnd: isize, point: *mut POINT) -> i32;
    pub fn ValidateRect(hwnd: isize, lpRect: *const RECT) -> i32;
    pub fn InvalidateRect(hwnd: isize, lpRect: *const RECT, erase: i32) -> i32;
    pub fn SetLayeredWindowAttributes(hwnd: isize, color_key: u32, alpha: u8, flags: u32) -> i32;
    pub fn GetSystemMetricsForDpi(nIndex: i32, dpi: u32) -> i32;
    pub fn GetThreadDpiAwarenessContext() -> *mut c_void;
    pub fn SetThreadDpiAwarenessContext(dpi_context: *mut c_void) -> isize;
    pub fn GetWindowDpiAwarenessContext(hwnd: isize) -> *mut c_void;
    pub fn GetDpiForWindow(hwnd: isize) -> u32;
    pub fn ReleaseCapture() -> i32;
    pub fn SetCursorPos(X: i32, Y: i32) -> i32;
    pub fn ShowCursor(bShow: i32) -> i32;
    pub fn SetCapture(hwnd: isize) -> isize;
    pub fn LoadIconA(hInstance: *mut c_void, lpIconName: *const i8) -> *mut c_void;
    pub fn SetWindowTextA(hwnd: isize, lpString: *const u8) -> i32;
    pub fn MonitorFromWindow(hwnd: isize, dwFlags: u32) -> *mut c_void;
    pub fn ClipCursor(lpRect: *const RECT) -> i32;
    pub fn SetCursor(hCursor: *mut c_void) -> *mut c_void;
    pub fn GetMonitorInfoA(hMonitor: *mut c_void, lpmi: *mut MONITORINFO) -> BOOL;
    pub fn WaitMessage() -> BOOL;
    pub fn TrackMouseEvent(lpEventTrack: *mut TRACKMOUSEEVENT) -> BOOL;
    pub fn CreateWindowExA(
        dwexstyle: u32,
        lpclassname: *const u8,
        lpwindowname: *const u8,
        dwstyle: u32,
        x: i32,
        y: i32,
        nwidth: i32,
        nheight: i32,
        hwndparent: isize,
        hmenu: isize,
        hinstance: isize,
        lpparam: *const std::ffi::c_void,
    ) -> isize;
    pub fn PeekMessageA(
        msg: *mut MSG,
        hwnd: isize,
        msg_filter_min: u32,
        msg_filter_max: u32,
        remove_msg: u32,
    ) -> i32;
    pub fn GetMessageA(msg: *mut MSG, hwnd: isize, msg_filter_min: u32, msg_filter_max: u32)
    -> i32;
    pub fn RegisterRawInputDevices(
        pRawInputDevices: *const RAWINPUTDEVICE,
        uiNumDevices: UINT,
        cbSize: UINT,
    ) -> BOOL;
    pub fn GetRawInputData(
        hRawInput: HANDLE,
        uiCommand: UINT,
        pData: *mut c_void,
        pcbSize: *mut UINT,
        cbSizeHeader: UINT,
    ) -> UINT;
    pub fn SetWindowPos(
        hWnd: isize,
        hWndInsertAfter: isize,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: u32,
    ) -> i32;
    pub fn MoveWindow(
        hWnd: HWND,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        bRepaint: BOOL,
    ) -> BOOL;
    pub fn DwmGetWindowAttribute(
        hWnd: isize,
        dwAttribute: u32,
        pvAttribute: *mut c_void,
        cbAttribute: u32,
    ) -> i32;
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl GUID {
    pub const fn from_u128(uuid: u128) -> Self {
        Self {
            data1: (uuid >> 96) as u32,
            data2: (uuid >> 80 & 0xffff) as u16,
            data3: (uuid >> 64 & 0xffff) as u16,
            data4: (uuid as u64).to_be_bytes(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct TRACKMOUSEEVENT {
    pub cbSize: DWORD,
    pub dwFlags: DWORD,
    pub hwndTrack: HWND,
    pub dwHoverTime: DWORD,
}

impl Rect {
    pub const fn from_windows(rect: RECT) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: rect.right.saturating_sub(rect.left),
            height: rect.bottom.saturating_sub(rect.top),
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct MSG {
    pub hwnd: isize,
    pub message: u32,
    pub w_param: usize,
    pub l_param: isize,
    pub time: u32,
    pub pt: POINT,
}

impl MSG {
    pub const fn new() -> Self {
        Self {
            hwnd: 0,
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            pt: POINT { x: 0, y: 0 },
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct WindowInfo {
    pub size: u32,
    pub window: RECT,
    pub client: RECT,
    pub style: u32,
    pub ex_style: u32,
    pub window_status: u32,
    pub window_borders_x: u32,
    pub window_borders_y: u32,
    pub window_type: u16,
    pub creator_version: u16,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct WNDCLASSA {
    pub style: u32,
    pub wnd_proc: Option<
        unsafe extern "system" fn(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize,
    >,
    pub cls_extra: i32,
    pub wnd_extra: i32,
    pub instance: isize,
    pub icon: isize,
    pub cursor: isize,
    pub background: isize,
    pub menu_name: *const u8,
    pub class_name: *const u8,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct RAWINPUTDEVICE {
    pub usUsagePage: WORD,
    pub usUsage: WORD,
    pub dwFlags: DWORD,
    pub hwndTarget: HWND,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct RAWINPUTHEADER {
    pub dwType: DWORD,
    pub dwSize: DWORD,
    pub hDevice: HANDLE,
    pub wParam: WPARAM,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union RAWMOUSE_BUTTONS {
    pub ulButtons: DWORD,
    pub buttons: RAWMOUSE_BUTTON_DATA,
}

impl Default for RAWMOUSE_BUTTONS {
    fn default() -> Self {
        Self { ulButtons: 0 }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct RAWMOUSE_BUTTON_DATA {
    pub usButtonFlags: WORD,
    pub usButtonData: WORD,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RAWMOUSE {
    pub usFlags: WORD,
    pub buttons: RAWMOUSE_BUTTONS,
    pub ulRawButtons: DWORD,
    pub lLastX: LONG,
    pub lLastY: LONG,
    pub ulExtraInformation: DWORD,
}

impl Default for RAWMOUSE {
    fn default() -> Self {
        Self {
            usFlags: 0,
            buttons: RAWMOUSE_BUTTONS::default(),
            ulRawButtons: 0,
            lLastX: 0,
            lLastY: 0,
            ulExtraInformation: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union RAWINPUT_DATA {
    pub mouse: RAWMOUSE,
}

impl Default for RAWINPUT_DATA {
    fn default() -> Self {
        Self {
            mouse: RAWMOUSE::default(),
        }
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct RAWINPUT {
    pub header: RAWINPUTHEADER,
    pub data: RAWINPUT_DATA,
}

pub fn accent_color() -> u32 {
    let mut color = 0;
    let mut blend = 0;
    unsafe { DwmGetColorizationColor(&mut color, &mut blend) };
    color & 0x00FFFFFF
}

#[inline]
pub fn get_client_rect(hwnd: isize) -> Rect {
    let mut rect = RECT::default();
    let _ = unsafe { GetClientRect(hwnd, &mut rect) };
    Rect::from_windows(rect)
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MONITORINFO {
    pub cbSize: u32,
    pub rcMonitor: RECT,
    pub rcWork: RECT,
    pub dwFlags: u32,
}

impl Default for MONITORINFO {
    fn default() -> Self {
        Self {
            cbSize: size_of::<Self>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        }
    }
}
