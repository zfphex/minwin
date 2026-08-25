use crate::*;
pub const DEFAULT_DPI: f32 = 96.0;

pub fn create_window(
    title: &str,
    position: Option<(i32, i32)>,
    width: i32,
    height: i32,
    use_gpu: bool,
    style: WindowStyle,
) -> std::pin::Pin<Box<Window>> {
    unsafe {
        if SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == 0 {
            panic!("Only Windows 10 (1607) or later is supported.")
        };

        //The title doubles as the window class name, which cannot be empty.
        assert!(!title.is_empty(), "window title cannot be empty");

        //Title must be null terminated.
        let title = std::ffi::CString::new(title).unwrap();

        let wnd_class = WNDCLASSA {
            style: CS_DBLCLKS,
            wnd_proc: Some(wnd_proc),
            cls_extra: 0,
            wnd_extra: 0,
            instance: 0,
            icon: 0,
            //Prevent cursor from changing when loading.
            cursor: LoadCursorW(null_mut(), IDC_ARROW) as isize,
            background: 0,
            menu_name: std::mem::zeroed(),
            class_name: title.as_ptr() as *const u8,
        };

        //Adjust the rect to fit exactly what the user requested.
        //Windows has padding and other weird nonsense when trying set the width and height.
        //Not needed anymore?

        // let mut rect = RECT {
        //     left: 0,
        //     top: 0,
        //     right: width as i32,
        //     bottom: height as i32,
        // };
        // AdjustWindowRectEx(&mut rect, style.style, 0, style.exstyle);

        RegisterClassA(&wnd_class);

        let (win_style, win_exstyle) = get_style_flags(style);
        let (x, y) = position.unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

        let hwnd = CreateWindowExA(
            win_exstyle,
            title.as_ptr() as *const u8,
            title.as_ptr() as *const u8,
            win_style,
            x,
            y,
            // CW_USEDEFAULT,
            // CW_USEDEFAULT,
            //These are adjusted later for DPI scaling.
            width,
            height,
            0,
            0,
            0,
            null(),
        );

        DragAcceptFiles(hwnd, 1);

        //Get the display scale factor 1.0, 1.25, 1.5, 1.75, can also be custom.
        let scale = GetDpiForWindow(hwnd) as f32 / DEFAULT_DPI;
        let mut area = get_client_rect(hwnd);

        //Scale the size of the window to match the display scale.
        //AdjustWindowRect used to be needed, but isn't anymore, I'm not sure why?
        if scale != 1.0 {
            SetWindowPos(
                hwnd,
                0,
                area.x,
                area.y,
                (area.width as f32 * scale) as i32,
                (area.height as f32 * scale) as i32,
                SWP_FRAMECHANGED,
            );
            //Update the area since SetWindowPos will change it.
            area = get_client_rect(hwnd);
        }

        assert_ne!(hwnd, 0);
        register_raw_mouse_input(hwnd);
        let dc = GetDC(hwnd);

        // Construct window, initialize WGL, then pin.
        let window = Window {
            //Re-grab the area after calling SetWindowPos.
            area,
            hwnd,
            dc,
            display_scale: scale,
            buffer: if use_gpu {
                Vec::new()
            } else {
                vec![0u32; (area.width.max(0) as usize) * (area.height.max(0) as usize)]
            },
            bitmap: BITMAPINFO::new(area.width, area.height),
            open: true,
            input: InputState::new(),
            hglrc: null_mut(),
            focused: true,
            render_callback: std::ptr::null_mut(),
            render_executor: None,
            native_repaint_requested: false,
            use_gpu,
            cursor: std::cell::Cell::new(CursorIcon::Arrow),
            caption_height: 0,
            caption_exclusions: Vec::new(),
        };

        //Safety: This *should* be pinned.
        let window = Box::pin(window);
        let addr = &*window as *const Window;
        let result = SetWindowLongPtrW(window.hwnd, GWLP_USERDATA, addr as isize);
        assert!(result <= 0);

        window
    }
}

#[derive(Debug)]
pub struct Window {
    pub hwnd: isize,
    pub hglrc: HGLRC,
    pub display_scale: f32,
    pub dc: *mut c_void,
    //Might need to be resized before use.
    buffer: Vec<u32>,
    bitmap: BITMAPINFO,
    area: Rect,
    open: bool,
    input: InputState,
    focused: bool,
    render_callback: *mut std::ffi::c_void,
    render_executor: Option<unsafe fn(*mut std::ffi::c_void, &mut Window)>,
    native_repaint_requested: bool,
    use_gpu: bool,
    cursor: std::cell::Cell<CursorIcon>,
    caption_height: i32,
    caption_exclusions: Vec<Rect>,
}

impl Window {
    /// Safety: Mutiple calls to this is unsafe.
    pub unsafe fn init_wgl_debug(&mut self) {
        pub const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
        pub const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;
        pub const WGL_CONTEXT_FLAGS_ARB: i32 = 0x2094;
        pub const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;

        pub const WGL_CONTEXT_DEBUG_BIT_ARB: i32 = 0x0001;
        pub const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: i32 = 0x00000001;

        unsafe {
            let mut pfd = PIXELFORMATDESCRIPTOR::default();
            pfd.nSize = std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as WORD;
            pfd.nVersion = 1;
            pfd.dwFlags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER;
            pfd.iPixelType = PFD_TYPE_RGBA;
            pfd.cColorBits = 32;
            pfd.cDepthBits = 24;
            pfd.cStencilBits = 8;
            pfd.iLayerType = PFD_MAIN_PLANE;

            let pixel_format = ChoosePixelFormat(self.dc, &pfd);
            assert!(pixel_format > 0);
            assert!(SetPixelFormat(self.dc, pixel_format, &pfd) != 0);

            let dummy_hglrc = wglCreateContext(self.dc);
            assert!(!dummy_hglrc.is_null());
            assert!(wglMakeCurrent(self.dc, dummy_hglrc) != 0,);

            let ptr = wglGetProcAddress(c"wglCreateContextAttribsARB".as_ptr() as *const i8);
            assert!(!ptr.is_null());

            let wgl_create_context_attribs_arb: unsafe extern "system" fn(
                *mut c_void,
                *mut c_void,
                *const i32,
            )
                -> *mut c_void = { std::mem::transmute(ptr) };

            let attribs = [
                WGL_CONTEXT_MAJOR_VERSION_ARB,
                4,
                WGL_CONTEXT_MINOR_VERSION_ARB,
                6,
                WGL_CONTEXT_PROFILE_MASK_ARB,
                WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
                WGL_CONTEXT_FLAGS_ARB,
                WGL_CONTEXT_DEBUG_BIT_ARB,
                0,
            ];

            let hglrc = wgl_create_context_attribs_arb(self.dc, null_mut(), attribs.as_ptr());
            assert!(!hglrc.is_null(), "wglCreateContextAttribsARB failed");

            assert!(wglMakeCurrent(self.dc, hglrc) != 0);
            // Skip for performance
            // assert!(wglDeleteContext(dummy_hglrc) != 0);
            self.hglrc = hglrc;
        }
    }

    pub unsafe fn init_wgl(&mut self) {
        unsafe {
            let mut pfd = PIXELFORMATDESCRIPTOR::default();
            pfd.nSize = std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as WORD;
            pfd.nVersion = 1;
            pfd.dwFlags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER;
            pfd.iPixelType = PFD_TYPE_RGBA;
            pfd.cColorBits = 32;
            pfd.cDepthBits = 24;
            pfd.cStencilBits = 8;
            pfd.iLayerType = PFD_MAIN_PLANE;

            let pixel_format = ChoosePixelFormat(self.dc, &pfd);
            assert!(pixel_format > 0);
            assert!(SetPixelFormat(self.dc, pixel_format, &pfd) != 0);

            let hglrc = wglCreateContext(self.dc);
            assert!(!hglrc.is_null());
            assert!(wglMakeCurrent(self.dc, hglrc) != 0);
            self.hglrc = hglrc;
        }
    }

    pub unsafe fn destroy_wgl(&mut self) {
        if self.hglrc.is_null() {
            return;
        }
        unsafe {
            assert!(wglMakeCurrent(null_mut(), null_mut()) != 0);
            assert!(wglDeleteContext(self.hglrc) != 0);
        }
        self.hglrc = null_mut();
    }

    pub fn get_wgl_proc_address(&self, name: &str) -> *const c_void {
        unsafe { wglGetProcAddress(std::ffi::CString::new(name).unwrap().as_ptr()) }
    }

    pub unsafe fn set_swap_interval(&self, interval: i32) {
        let ptr = unsafe { wglGetProcAddress(c"wglSwapIntervalEXT".as_ptr() as *const _) };
        assert!(!ptr.is_null());
        let func: unsafe extern "system" fn(i32) -> i32 = unsafe { std::mem::transmute(ptr) };
        unsafe { func(interval) };
    }

    fn client_area(&self) -> Rect {
        let mut rect = RECT::default();
        let _ = unsafe { GetClientRect(self.hwnd, &mut rect) };
        Rect::from_windows(rect)
    }

    //  fn reset_style(&mut self) {
    //     unsafe {
    //         SetWindowLongPtrA(
    //             self.hwnd,
    //             GWL_STYLE,
    //             get_style_flags(WindowStyle::Standard).0 as isize,
    //         );

    //         //Update the window area without moving or resizing it.
    //         SetWindowPos(
    //             self.hwnd,
    //             0,
    //             0,
    //             0,
    //             0,
    //             0,
    //             SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE,
    //         );
    //     };
    // }
}

impl PlatformWindow for Window {
    fn draw<F>(&mut self, mut render: F)
    where
        F: FnMut(&mut Self),
    {
        self.input.begin_frame();
        unsafe fn execute_render<F>(closure_ptr: *mut c_void, window: &mut Window)
        where
            F: FnMut(&mut Window),
        {
            unsafe {
                let closure = &mut *(closure_ptr as *mut F);
                closure(window);
            }
        }

        self.render_callback = &mut render as *mut F as *mut c_void;
        self.render_executor = Some(execute_render::<F>);
        self.native_repaint_requested = false;

        unsafe {
            let mut msg = MSG::default();
            while PeekMessageA(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    self.open = false;
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageA(&msg);
                if !self.open {
                    break;
                }
            }
        }

        self.render_callback = std::ptr::null_mut();
        self.render_executor = None;

        if !self.open {
            return;
        }

        if !self.native_repaint_requested {
            render(self);
        }
    }

    fn open(&self) -> bool {
        self.open
    }

    fn close(&mut self) {
        if self.open {
            self.open = false;
            unsafe { DestroyWindow(self.hwnd) };
        }
    }

    fn is_down(&self, key: Key) -> bool {
        self.input.is_down(key)
    }

    fn is_up(&self, key: Key) -> bool {
        self.input.is_up(key)
    }

    fn pressed(&self, key: Key) -> bool {
        self.input.pressed(key)
    }

    fn released(&self, key: Key) -> bool {
        self.input.released(key)
    }

    fn pressed_keys(&self) -> &[Key] {
        self.input.pressed_keys()
    }

    fn mouse_down(&self, button: Mouse) -> bool {
        self.input.mouse_down(button)
    }

    fn mouse_pressed(&self, button: Mouse) -> bool {
        self.input.mouse_pressed(button)
    }

    fn mouse_released(&self, button: Mouse) -> bool {
        self.input.mouse_released(button)
    }

    fn mouse_clicked(&self, button: Mouse, area: Rect) -> bool {
        self.input.mouse_clicked(button, area)
    }

    fn mouse_double_clicked(&self, button: Mouse, area: Rect) -> bool {
        self.input.mouse_double_clicked(button, area)
    }

    fn mouse_pos(&self) -> Option<(f64, f64)> {
        self.input.mouse_pos()
    }

    fn text_input(&self) -> &[char] {
        self.input.text_input()
    }

    fn dropped_files(&self) -> &[std::path::PathBuf] {
        self.input.dropped_files()
    }

    fn scroll_delta(&self) -> (f64, f64) {
        self.input.scroll_delta()
    }

    fn scroll_events(&self) -> &[ScrollEvent] {
        self.input.scroll_events()
    }

    fn raw_mouse_delta(&self) -> (f64, f64) {
        self.input.raw_mouse_delta()
    }

    fn modifiers(&self) -> Modifiers {
        self.input.modifiers()
    }

    fn wait_for_vsync(&self) {
        unsafe { DwmFlush() };
    }

    fn wait_for_event(&self) {
        unsafe { WaitMessage() };
    }

    fn framebuffer(&mut self) -> &mut [u32] {
        let (width, height) = self.scaled_size();

        if self.buffer.len() != width * height {
            self.buffer.resize(width * height, 0);
            self.bitmap = BITMAPINFO::new(width as i32, height as i32);
            self.area = Rect::new(0, 0, width as i32, height as i32);
        }

        &mut self.buffer
    }

    fn size(&self) -> (usize, usize) {
        let rect = self.client_area();
        let scale = self.scale_factor() as f32;
        (
            (rect.width as f32 / scale).ceil().max(0.0) as usize,
            (rect.height as f32 / scale).ceil().max(0.0) as usize,
        )
    }

    fn present(&self) {
        if self.use_gpu {
            unsafe { SwapBuffers(self.dc) };
            return;
        }

        let client_area = self.client_area();

        if client_area.width == 0
            || client_area.height == 0
            || self.area.width == 0
            || self.area.height == 0
            || self.buffer.is_empty()
        {
            return;
        }

        unsafe {
            StretchDIBits(
                self.dc,
                0,
                0,
                client_area.width as i32,
                client_area.height as i32,
                0,
                0,
                self.area.width as i32,
                self.area.height as i32,
                self.buffer.as_ptr() as *const c_void,
                &self.bitmap,
                0,
                SRCCOPY,
            );
        }
    }

    fn present_damage(&self, damage: &[Rect]) {
        if self.use_gpu || damage.is_empty() {
            self.present();
            return;
        }

        let client_area = self.client_area();
        if client_area.width == 0
            || client_area.height == 0
            || self.area.width == 0
            || self.area.height == 0
            || self.buffer.is_empty()
        {
            return;
        }

        if client_area.width != self.area.width || client_area.height != self.area.height {
            self.present();
            return;
        }

        unsafe {
            for rect in damage {
                let rect = rect.intersection(self.area);
                if rect.is_empty() {
                    continue;
                }
                // Describe just the rect's rows as their own top-down bitmap, keeping the
                // full framebuffer width as the stride. Consuming every scanline of that
                // sub-bitmap sidesteps the question of which end GDI measures ySrc from,
                // which is ambiguous for top-down DIBs and differs from the full present.
                let strip = BITMAPINFO::new(self.area.width, rect.height);
                let offset = rect.y as usize * self.area.width as usize;
                StretchDIBits(
                    self.dc,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    rect.x,
                    0,
                    rect.width,
                    rect.height,
                    self.buffer.as_ptr().add(offset) as *const c_void,
                    &strip,
                    0,
                    SRCCOPY,
                );
            }
        }
    }

    fn scale_factor(&self) -> f64 {
        self.display_scale as f64
    }

    fn scaled_size(&self) -> (usize, usize) {
        let rect = self.client_area();
        (rect.width.max(0) as usize, rect.height.max(0) as usize)
    }

    fn set_cursor_visible(&self, visible: bool) {
        unsafe {
            ShowCursor(if visible { 1 } else { 0 });
        }
    }

    fn set_cursor_grab(&self, grab: bool) {
        unsafe {
            if grab {
                let mut rect = RECT::default();
                GetClientRect(self.hwnd, &mut rect);
                let mut pt_top_left = POINT {
                    x: rect.left,
                    y: rect.top,
                };
                let mut pt_bottom_right = POINT {
                    x: rect.right,
                    y: rect.bottom,
                };
                ClientToScreen(self.hwnd, &mut pt_top_left);
                ClientToScreen(self.hwnd, &mut pt_bottom_right);
                rect.left = pt_top_left.x;
                rect.top = pt_top_left.y;
                rect.right = pt_bottom_right.x;
                rect.bottom = pt_bottom_right.y;
                ClipCursor(&rect);
            } else {
                ClipCursor(null());
            }
        }
    }

    //TODO: Allow for users to set custom cursors.
    fn set_cursor_icon(&self, icon: CursorIcon) {
        self.cursor.set(icon);
        unsafe {
            SetCursor(cursor_handle(icon));
        }
    }

    fn get_clipboard_text(&self) -> Option<String> {
        unsafe {
            if OpenClipboard(self.hwnd) == 0 {
                return None;
            }
            let handle = GetClipboardData(CF_TEXT);
            let mut result = None;
            if !handle.is_null() {
                let ptr = GlobalLock(handle) as *const u8;
                if !ptr.is_null() {
                    let mut len = 0;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    if let Ok(s) = std::str::from_utf8(std::slice::from_raw_parts(ptr, len)) {
                        result = Some(s.to_string());
                    }
                    GlobalUnlock(handle);
                }
            }
            CloseClipboard();
            result
        }
    }

    fn set_clipboard_text(&self, text: &str) {
        copy_to_clipboard(text);
    }

    fn set_title(&self, title: &str) {
        if let Ok(c_title) = std::ffi::CString::new(title) {
            unsafe {
                SetWindowTextA(self.hwnd, c_title.as_ptr() as *const u8);
            }
        }
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn fullscreen_mode(&mut self, mode: Fullscreen) {
        if mode == Fullscreen::None {
            return self.window_style(WindowStyle::Standard);
        }
        unsafe {
            let monitor = MonitorFromWindow(self.hwnd, 1);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            GetMonitorInfoA(monitor, &mut info);
            SetWindowLongPtrA(
                self.hwnd,
                GWL_STYLE,
                (get_style_flags(WindowStyle::Borderless).0 | WS_MAXIMIZE) as isize,
            );
            SetWindowPos(
                self.hwnd,
                0,
                info.rcMonitor.left,
                info.rcMonitor.top,
                info.rcMonitor.right - info.rcMonitor.left,
                info.rcMonitor.bottom - info.rcMonitor.top,
                SWP_FRAMECHANGED,
            );
        }
    }

    fn custom_titlebar(&mut self, height: i32, exclusions: &[Rect]) {
        self.caption_exclusions.clear();
        self.caption_exclusions.extend_from_slice(exclusions);

        if self.caption_height == height {
            return;
        }

        self.caption_height = height;
        if self.caption_height == 0 {
            unsafe {
                let margins = MARGINS {
                    left: 0,
                    right: 0,
                    top: 0,
                    bottom: 0,
                };
                DwmExtendFrameIntoClientArea(self.hwnd, &margins);
                SetWindowPos(
                    self.hwnd,
                    0,
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
                );
            }
        }
    }

    fn show(&mut self) {
        unsafe { ShowWindow(self.hwnd, SW_SHOWNORMAL) };
    }

    fn hide(&mut self) {
        unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    fn set_size(&mut self, width: i32, height: i32) {
        unsafe {
            SetWindowPos(self.hwnd, 0, 0, 0, width, height, SWP_NOMOVE | SWP_NOZORDER);
        }
    }

    fn minimize(&mut self) {
        unsafe { ShowWindow(self.hwnd, SW_MINIMIZE) };
    }

    fn toggle_maximize(&mut self) {
        let cmd = if self.maximized() {
            SW_RESTORE
        } else {
            SW_MAXIMIZE
        };
        unsafe { ShowWindow(self.hwnd, cmd) };
    }

    fn maximized(&self) -> bool {
        unsafe { IsZoomed(self.hwnd) != 0 }
    }

    fn window_style(&mut self, style: WindowStyle) {
        unsafe {
            SetWindowLongPtrA(self.hwnd, GWL_STYLE, get_style_flags(style).0 as isize);
            SetWindowPos(
                self.hwnd,
                0,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE,
            );
        }
    }
}

pub fn get_style_flags(style: WindowStyle) -> (u32, u32) {
    match style {
        WindowStyle::Standard => (
            WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_VISIBLE,
            0,
        ),
        WindowStyle::Borderless | WindowStyle::Transparent => (WS_POPUP | WS_VISIBLE, 0),
    }
}

fn invoke_render_callback(window: &mut Window) {
    if window.render_callback.is_null() || window.render_executor.is_none() {
        return;
    }

    let cb_ptr = window.render_callback;
    let executor = window.render_executor;

    window.render_callback = std::ptr::null_mut();
    window.render_executor = None;

    if let Some(exec) = executor {
        unsafe { exec(cb_ptr, window) };
    }

    // Restore the callback pointer
    window.render_callback = cb_ptr;
    window.render_executor = executor;
}

fn current_modifiers() -> Modifiers {
    const KEY_DOWN: i16 = 0x8000u16 as i16;

    unsafe {
        Modifiers {
            shift: (GetKeyState(0x10) & KEY_DOWN) != 0,
            ctrl: (GetKeyState(0x11) & KEY_DOWN) != 0,
            alt: (GetKeyState(0x12) & KEY_DOWN) != 0,
            logo: (GetKeyState(0x5B) & KEY_DOWN) != 0 || (GetKeyState(0x5C) & KEY_DOWN) != 0,
        }
    }
}

fn register_raw_mouse_input(hwnd: HWND) {
    static REGISTER: std::sync::Once = std::sync::Once::new();

    REGISTER.call_once(|| {
        let device = RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x02,
            dwFlags: 0,
            hwndTarget: hwnd,
        };

        let ok = unsafe {
            RegisterRawInputDevices(&device, 1, std::mem::size_of::<RAWINPUTDEVICE>() as UINT)
        };
        debug_assert!(ok != 0, "RegisterRawInputDevices failed");
    });
}

unsafe fn handle_raw_input(lparam: isize, input: &mut InputState) -> bool {
    let mut raw_input = RAWINPUT::default();
    let mut size = std::mem::size_of::<RAWINPUT>() as UINT;
    let bytes_read = unsafe {
        GetRawInputData(
            lparam as HANDLE,
            RID_INPUT,
            &mut raw_input as *mut RAWINPUT as *mut c_void,
            &mut size,
            std::mem::size_of::<RAWINPUTHEADER>() as UINT,
        )
    };

    if bytes_read == UINT::MAX || bytes_read != size || raw_input.header.dwType != RIM_TYPEMOUSE {
        return false;
    }

    let mouse = unsafe { raw_input.data.mouse };
    input.raw_mouse_delta.0 += mouse.lLastX as f64;
    input.raw_mouse_delta.1 -= mouse.lLastY as f64;
    true
}

pub unsafe extern "system" fn wnd_proc(
    hwnd: isize,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    unsafe {
        if msg == WM_CREATE {
            set_dark_theme(hwnd);
            return 0;
        }

        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Window;
        if ptr.is_null() {
            return DefWindowProcA(hwnd, msg, wparam, lparam);
        }

        //I'm not convinced this is the right way to do this.
        let window: &mut Window = &mut *ptr;

        let low = (lparam & 0xffff) as usize;
        let mouse_x = lparam as i16 as f64 / window.display_scale as f64;
        let mouse_y = (lparam >> 16) as i16 as f64 / window.display_scale as f64;

        // println!("{}", wm_code_name(msg));

        match msg {
            //We can choose not to destroy the window, for example with a save prompt.
            WM_CLOSE => {
                window.open = false;
                assert!(DestroyWindow(hwnd) != 0);
                return 0;
            }
            WM_DROPFILES => {
                let hdrop = wparam as HANDLE;
                let count = DragQueryFileW(hdrop, 0xFFFFFFFF, null_mut(), 0);
                let mut files = Vec::new();
                for i in 0..count {
                    let len = DragQueryFileW(hdrop, i, null_mut(), 0);
                    if len > 0 {
                        let mut buf = vec![0u16; (len + 1) as usize];
                        DragQueryFileW(hdrop, i, buf.as_mut_ptr(), len + 1);
                        if let Ok(s) = String::from_utf16(&buf[..len as usize]) {
                            files.push(std::path::PathBuf::from(s));
                        }
                    }
                }
                DragFinish(hdrop);
                window.input.dropped_files.extend(files);
                return 0;
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                window.open = false;
                return 0;
            }
            WM_SIZE => {
                InvalidateRect(hwnd, null(), 0);
                return 0;
            }
            //The client area is stretched over the whole window, hiding the native frame.
            //A maximized window still has to leave the frame alone or it spills off screen.
            WM_NCCALCSIZE if wparam == 1 && window.caption_height > 0 => {
                if IsZoomed(hwnd) != 0 {
                    let dpi = GetDpiForWindow(hwnd);
                    let padding = GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
                    let x = GetSystemMetricsForDpi(SM_CXFRAME, dpi) + padding;
                    let y = GetSystemMetricsForDpi(SM_CYFRAME, dpi) + padding;
                    let params = &mut *(lparam as *mut NCCALCSIZE_PARAMS);
                    params.rgrc[0].left += x;
                    params.rgrc[0].right -= x;
                    params.rgrc[0].top += y;
                    params.rgrc[0].bottom -= y;
                }
                return 0;
            }
            //The frame is gone so the resize borders have to be hit tested by hand.
            WM_NCHITTEST if window.caption_height > 0 => {
                let mut point = POINT {
                    x: lparam as i16 as i32,
                    y: (lparam >> 16) as i16 as i32,
                };
                ScreenToClient(hwnd, &mut point);
                let client = window.client_area();

                if IsZoomed(hwnd) == 0 {
                    let dpi = GetDpiForWindow(hwnd);
                    let border = GetSystemMetricsForDpi(SM_CXFRAME, dpi)
                        + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
                    let left = point.x < border;
                    let right = point.x >= client.width - border;
                    let top = point.y < border;
                    let bottom = point.y >= client.height - border;

                    let edge = match (left, right, top, bottom) {
                        (true, _, true, _) => Some(HTTOPLEFT),
                        (_, true, true, _) => Some(HTTOPRIGHT),
                        (true, _, _, true) => Some(HTBOTTOMLEFT),
                        (_, true, _, true) => Some(HTBOTTOMRIGHT),
                        (true, ..) => Some(HTLEFT),
                        (_, true, ..) => Some(HTRIGHT),
                        (_, _, true, _) => Some(HTTOP),
                        (.., true) => Some(HTBOTTOM),
                        _ => None,
                    };

                    if let Some(edge) = edge {
                        return edge;
                    }
                }

                let x = (point.x as f32 / window.display_scale) as i32;
                let y = (point.y as f32 / window.display_scale) as i32;

                if y >= window.caption_height
                    || window
                        .caption_exclusions
                        .iter()
                        .any(|rect| rect.contains(x, y))
                {
                    return HTCLIENT as isize;
                }

                return HTCAPTION;
            }
            // Without this the class cursor is restored on every move, undoing set_cursor_icon.
            WM_SETCURSOR if low == HTCLIENT => {
                SetCursor(cursor_handle(window.cursor.get()));
                return 1;
            }
            WM_SIZING => {
                InvalidateRect(hwnd, null(), 0);
                return DefWindowProcA(hwnd, msg, wparam, lparam);
            }
            WM_PAINT => {
                window.native_repaint_requested = true;
                invoke_render_callback(window);
                ValidateRect(hwnd, null());
                return 0;
            }
            //https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged
            WM_DPICHANGED => {
                //The new display scale and DPI.
                let dpi = (wparam >> 16) & 0xffff;
                let scale = dpi as f32 / DEFAULT_DPI;

                //This is the recommended x, y, width and height.
                //The width and height is wrong so we ignore it.
                //X and Y seems right.
                let ptr = lparam as *mut RECT;
                assert!(!ptr.is_null());
                let rect = &(*ptr);

                let old = window.client_area();
                let original_width = old.width as f32 / window.display_scale;
                let original_height = old.height as f32 / window.display_scale;

                let width = original_width * scale;
                let height = original_height * scale;
                window.display_scale = scale;

                SetWindowPos(
                    hwnd,
                    0,
                    rect.left,
                    rect.top,
                    width.round() as i32,
                    height.round() as i32,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );

                return 0;
            }
            WM_CHAR => {
                if let Some(c) = char::from_u32(wparam as u32) {
                    if !c.is_control() {
                        window.input.text_input.push(c);
                    }
                }
                return 0;
            }
            WM_INPUT => {
                handle_raw_input(lparam, &mut window.input);
            }
            WM_MOUSEMOVE => {
                window.input.mouse_pos = Some((mouse_x, mouse_y));
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as DWORD,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut tme);
            }
            WM_MOUSELEAVE => {
                window.input.mouse_pos = None;
            }
            WM_MOUSEWHEEL => {
                const WHEEL_DELTA: i16 = 120;
                let value = (wparam >> 16) as i16;
                let delta_y = value as f64 / WHEEL_DELTA as f64;
                window.input.scroll_delta.1 += delta_y;
                // Win32 wheel messages carry no gesture.
                window.input.scroll_events.push(ScrollEvent {
                    delta: (0.0, delta_y),
                    phase: ScrollPhase::None,
                    precise: false,
                    timestamp: GetMessageTime() as f64 / 1000.0,
                });
                return 0;
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let keycode = wparam as u16;
                window.input.set_key_down(Key::from_windows_vk(keycode));
                window.input.modifiers = current_modifiers();
            }
            WM_KEYUP | WM_SYSKEYUP => {
                let keycode = wparam as u16;
                window.input.set_key_up(Key::from_windows_vk(keycode));
                window.input.modifiers = current_modifiers();
            }
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
            | WM_LBUTTONDBLCLK | WM_RBUTTONDBLCLK | WM_MBUTTONDBLCLK | WM_XBUTTONDBLCLK => {
                let is_double = matches!(
                    msg,
                    WM_LBUTTONDBLCLK | WM_RBUTTONDBLCLK | WM_MBUTTONDBLCLK | WM_XBUTTONDBLCLK
                );
                let mouse = match msg {
                    WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => Some(Mouse::Left),
                    WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => Some(Mouse::Right),
                    WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => Some(Mouse::Middle),
                    WM_XBUTTONDOWN | WM_XBUTTONDBLCLK => match ((wparam >> 16) & 0xffff) as u16 {
                        1 => Some(Mouse::Back),
                        2 => Some(Mouse::Forward),
                        _ => None,
                    },
                    _ => None,
                };

                if let Some(mouse) = mouse {
                    SetCapture(hwnd);
                    window.input.mouse_pos = Some((mouse_x, mouse_y));
                    window.input.modifiers = current_modifiers();

                    if is_double {
                        window.input.set_mouse_double_down(mouse);
                    } else {
                        window.input.set_mouse_down(mouse);
                    }
                }
            }
            WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => {
                // Only release capture if no other mouse buttons are currently being held down.
                if wparam as u32
                    & (MK_LBUTTON | MK_RBUTTON | MK_MBUTTON | MK_XBUTTON1 | MK_XBUTTON2)
                    == 0
                {
                    ReleaseCapture();
                }

                window.input.mouse_pos = Some((mouse_x, mouse_y));
                match msg {
                    WM_LBUTTONUP => window.input.set_mouse_up(Mouse::Left),
                    WM_RBUTTONUP => window.input.set_mouse_up(Mouse::Right),
                    WM_MBUTTONUP => window.input.set_mouse_up(Mouse::Middle),
                    WM_XBUTTONUP => {
                        let button = ((wparam >> 16) & 0xffff) as usize;
                        if button == 1 {
                            window.input.set_mouse_up(Mouse::Back);
                        } else if button == 2 {
                            window.input.set_mouse_up(Mouse::Forward);
                        }
                    }
                    _ => {}
                }
                window.input.modifiers = current_modifiers();
            }
            WM_KILLFOCUS => {
                window.focused = false;
                return 0;
            }
            WM_SETFOCUS => {
                window.focused = true;
                return 0;
            }
            WM_TRAYICON if low as u32 == WM_LBUTTONDOWN => {
                return 0;
            }
            _ => {}
        }

        DefWindowProcA(hwnd, msg, wparam, lparam)
    }
}
