use std::ffi::c_void;

pub const WM_CREATE: u32 = 0x0001;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_SETFOCUS: u32 = 0x0007;
pub const WM_KILLFOCUS: u32 = 0x0008;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_NCCALCSIZE: u32 = 0x0083;
pub const WM_NCHITTEST: u32 = 0x0084;
pub const HTCLIENT: usize = 1;
pub const HTCAPTION: isize = 2;
pub const HTLEFT: isize = 10;
pub const HTRIGHT: isize = 11;
pub const HTTOP: isize = 12;
pub const HTTOPLEFT: isize = 13;
pub const HTTOPRIGHT: isize = 14;
pub const HTBOTTOM: isize = 15;
pub const HTBOTTOMLEFT: isize = 16;
pub const HTBOTTOMRIGHT: isize = 17;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_CHAR: u32 = 0x0102;
pub const WM_INPUT: u32 = 0x00FF;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_SYSKEYUP: u32 = 0x0105;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_RBUTTONDBLCLK: u32 = 0x0206;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_MBUTTONDBLCLK: u32 = 0x0209;
pub const WM_MOUSEWHEEL: u32 = 0x020A;
pub const WM_XBUTTONDOWN: u32 = 0x020B;
pub const WM_XBUTTONUP: u32 = 0x020C;
pub const WM_XBUTTONDBLCLK: u32 = 0x020D;

pub const CS_DBLCLKS: u32 = 0x0008;
pub const WM_MOUSELEAVE: u32 = 0x02A3;
pub const TME_LEAVE: u32 = 0x00000002;
pub const WM_SIZING: u32 = 0x0214;
pub const WM_DROPFILES: u32 = 0x0233;
pub const WM_DPICHANGED: u32 = 0x02E0;

pub const MK_LBUTTON: u32 = 0x0001;
pub const MK_RBUTTON: u32 = 0x0002;
pub const MK_MBUTTON: u32 = 0x0010;
pub const MK_XBUTTON1: u32 = 0x0020;
pub const MK_XBUTTON2: u32 = 0x0040;

pub const RID_INPUT: u32 = 0x10000003;
pub const RIM_TYPEMOUSE: u32 = 0;

pub const GWLP_USERDATA: i32 = -21;

pub const PM_REMOVE: u32 = 0x0001;

pub const WS_CAPTION: u32 = 0x00C00000;
pub const WS_MAXIMIZE: u32 = 0x01000000;
pub const WS_MAXIMIZEBOX: u32 = 0x00010000;
pub const WS_MINIMIZEBOX: u32 = 0x00020000;
pub const WS_POPUP: u32 = 0x80000000;
pub const WS_SYSMENU: u32 = 0x00080000;
pub const WS_THICKFRAME: u32 = 0x00040000;
pub const WS_VISIBLE: u32 = 0x10000000;

pub const CW_USEDEFAULT: i32 = -2147483648i32;

pub const IDC_ARROW: *const u16 = 32512 as *const u16;
pub const IDC_IBEAM: *const u16 = 32513 as *const u16;
pub const IDC_CROSS: *const u16 = 32515 as *const u16;
pub const IDC_SIZEWE: *const u16 = 32644 as *const u16;
pub const IDC_SIZENS: *const u16 = 32645 as *const u16;
pub const IDC_HAND: *const u16 = 32649 as *const u16;

pub const GWL_STYLE: i32 = -16;

pub const SM_CXFRAME: i32 = 32;
pub const SM_CYFRAME: i32 = 33;
pub const SM_CXPADDEDBORDER: i32 = 92;

pub const SW_MAXIMIZE: i32 = 3;
pub const SW_MINIMIZE: i32 = 6;
pub const SW_RESTORE: i32 = 9;

pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_FRAMECHANGED: u32 = 0x0020;

pub const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: *mut c_void = -4isize as *mut c_void;

pub const IDI_APPLICATION: i32 = 32512;
pub const IDI_HAND: i32 = 32513;
pub const IDI_QUESTION: i32 = 32514;
pub const IDI_EXCLAMATION: i32 = 32515;
pub const IDI_ASTERISK: i32 = 32516;
pub const IDI_WINLOGO: i32 = 32517;

pub const SRCCOPY: u32 = 0x00CC0020;
