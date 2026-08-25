#![allow(non_upper_case_globals)]

use crate::ffi::*;
use crate::objc::*;
use crate::vsync::VsyncTracker;
use crate::*;
use std::mem::transmute;
use std::path::PathBuf;

thread_local! {
    pub static REPAINT_CALLBACK: std::cell::Cell<Option<*mut std::ffi::c_void>> = const { std::cell::Cell::new(None) };
    pub static REPAINT_FUNC: std::cell::Cell<Option<fn(*mut std::ffi::c_void, &mut Window)>> = const { std::cell::Cell::new(None) };
    static ACTIVE_WINDOW: std::cell::Cell<*mut Window> = const { std::cell::Cell::new(std::ptr::null_mut()) };
    static TRACKING_REPAINT_TIMER: std::cell::Cell<id> = const { std::cell::Cell::new(nil) };
    static LIVE_RESIZE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub struct Window {
    pub ns_window: id,
    pub ns_view: id,
    pub ns_delegate: id,
    ns_app: id,
    /// The view's backing layer. Stable for the life of a wantsLayer view, so it
    /// is fetched once instead of every present.
    layer: id,
    /// Backing scale and content size, refreshed from the delegate callbacks
    /// rather than queried per frame. Cells so `&self` presents stay cheap.
    scale: std::cell::Cell<f64>,
    view_size: std::cell::Cell<(usize, usize)>,
    /// Implementations for the calls made every frame or every event. Resolved
    /// once against the receiver's class, which skips objc_msgSend's cache probe.
    /// Only valid because each receiver here has a fixed class.
    set_contents: unsafe extern "C" fn(id, SEL, id),
    set_contents_scale: unsafe extern "C" fn(id, SEL, f64),
    ca_begin: unsafe extern "C" fn(id, SEL),
    ca_set_disable_actions: unsafe extern "C" fn(id, SEL, BOOL),
    ca_commit: unsafe extern "C" fn(id, SEL),
    send_event: unsafe extern "C" fn(id, SEL, id),
    buffer: Vec<u32>,
    width: usize,
    height: usize,
    vsync: VsyncTracker,
    input: InputState,
    open: bool,
    use_gpu: bool,
    pub caption_height: i32,
    pub caption_exclusions: Vec<Rect>,
    _marker: std::marker::PhantomData<*mut ()>,
}

static APP_INIT: std::sync::Once = std::sync::Once::new();

#[derive(Debug, Clone, Copy, PartialEq)]
enum WindowPosition {
    Centered,
    TopLeft { x: f64, y: f64 },
}

pub fn create_window(
    title: &str,
    position: Option<(i32, i32)>,
    width: i32,
    height: i32,
    use_gpu: bool,
    style: WindowStyle,
) -> std::pin::Pin<Box<Window>> {
    let position = match position {
        Some((x, y)) => WindowPosition::TopLeft {
            x: x as f64,
            y: y as f64,
        },
        None => WindowPosition::Centered,
    };

    let fullscreen = Fullscreen::None;
    let width = width as f64;
    let height = height as f64;

    #[cfg(debug_assertions)]
    assert_main_thread();

    unsafe {
        APP_INIT.call_once(|| {
            let ns_app = msg_send_id(class!("NSApplication"), sel!("sharedApplication"));

            let set_policy_sel = sel!("setActivationPolicy:");
            let set_policy: unsafe extern "C" fn(id, SEL, NSApplicationActivationPolicy) -> BOOL =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            set_policy(ns_app, set_policy_sel, NSApplicationActivationPolicyRegular);

            // Register delegate and view classes
            register_delegate_class();
            register_view_class();

            // Setup menu bar
            let alloc_sel = sel!("alloc");
            let init_sel = sel!("init");

            let main_menu = msg_send_id(msg_send_id(class!("NSMenu"), alloc_sel), init_sel);

            let app_menu_item = msg_send_id(msg_send_id(class!("NSMenuItem"), alloc_sel), init_sel);

            msg_send_id_id_void(main_menu, sel!("addItem:"), app_menu_item);

            let app_menu = msg_send_id(msg_send_id(class!("NSMenu"), alloc_sel), init_sel);

            msg_send_id_id_void(app_menu_item, sel!("setSubmenu:"), app_menu);

            let quit_title = nsstring("Quit");
            let quit_sel = sel!("terminate:");
            let key = nsstring("q");
            let quit_item_alloc = msg_send_id(class!("NSMenuItem"), alloc_sel);

            let init_quit_func: unsafe extern "C" fn(id, SEL, id, SEL, id) -> id =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            let quit_item = init_quit_func(
                quit_item_alloc,
                sel!("initWithTitle:action:keyEquivalent:"),
                quit_title,
                quit_sel,
                key,
            );

            msg_send_id_id_void(app_menu, sel!("addItem:"), quit_item);

            msg_send_id_id_void(ns_app, sel!("setMainMenu:"), main_menu);

            // Finish launching
            msg_send_id(ns_app, sel!("finishLaunching"));
        });

        let alloc_sel = sel!("alloc");

        let mut final_width = width;
        let mut final_height = height;

        let mut style_mask = match style {
            WindowStyle::Standard => {
                NSWindowStyleMaskTitled
                    | NSWindowStyleMaskClosable
                    | NSWindowStyleMaskMiniaturizable
                    | NSWindowStyleMaskResizable
            }
            WindowStyle::Borderless | WindowStyle::Transparent => NSWindowStyleMaskBorderless,
        };

        if fullscreen == Fullscreen::Workspace {
            style_mask |= NSWindowStyleMaskFullScreen;
        }

        if fullscreen == Fullscreen::Monitor {
            let main_screen = msg_send_id(class!("NSScreen"), sel!("mainScreen"));
            let frame_sel = sel!("frame");
            let screen_rect = msg_send_rect(main_screen, frame_sel);
            final_width = screen_rect.size.width;
            final_height = screen_rect.size.height;
            style_mask = NSWindowStyleMaskBorderless;
        }

        let rect = match (fullscreen, position) {
            (Fullscreen::None, WindowPosition::TopLeft { x, y }) => {
                let main_screen = msg_send_id(class!("NSScreen"), sel!("mainScreen"));
                let visible_frame_sel = sel!("visibleFrame");
                let screen_rect = msg_send_rect(main_screen, visible_frame_sel);
                let origin_y = screen_rect.origin.y + screen_rect.size.height - y - final_height;

                NSRect::new(
                    screen_rect.origin.x + x,
                    origin_y,
                    final_width,
                    final_height,
                )
            }
            _ => NSRect::new(0.0, 0.0, final_width, final_height),
        };
        let window_class = class!("NSWindow");
        let window_alloc = msg_send_id(window_class, alloc_sel);

        let init_window_func: unsafe extern "C" fn(
            id,
            SEL,
            NSRect,
            NSWindowStyleMask,
            NSBackingStoreType,
            BOOL,
        ) -> id = std::mem::transmute(objc_msgSend as *const std::ffi::c_void);

        let ns_window = init_window_func(
            window_alloc,
            sel!("initWithContentRect:styleMask:backing:defer:"),
            rect,
            style_mask,
            NSBackingStoreBuffered,
            NO,
        );

        if fullscreen == Fullscreen::None && position == WindowPosition::Centered {
            msg_send_void(ns_window, sel!("center"));
        }

        if style == WindowStyle::Transparent {
            msg_send_id_bool_void(ns_window, sel!("setOpaque:"), NO);
            let color_class = class!("NSColor");
            let clear_color = msg_send_id(color_class, sel!("clearColor"));
            msg_send_id_id_void(ns_window, sel!("setBackgroundColor:"), clear_color);
        }

        let title_ns = nsstring(title);
        msg_send_id_id_void(ns_window, sel!("setTitle:"), title_ns);

        // Instantiate our custom RustView class
        let view_class = register_view_class();
        let view_alloc = msg_send_id(view_class, alloc_sel);
        let init_view_func: unsafe extern "C" fn(id, SEL, NSRect) -> id =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        let ns_view = init_view_func(view_alloc, sel!("initWithFrame:"), rect);

        msg_send_id_bool_void(ns_view, sel!("setWantsLayer:"), YES);

        msg_send_id_usize_void(ns_view, sel!("setLayerContentsRedrawPolicy:"), 0);
        msg_send_id_usize_void(ns_view, sel!("setLayerContentsPlacement:"), 11);

        msg_send_id_id_void(ns_window, sel!("setContentView:"), ns_view);

        if fullscreen == Fullscreen::Workspace {
            // NSWindowCollectionBehaviorFullScreenPrimary = 1 << 7
            msg_send_id_usize_void(ns_window, sel!("setCollectionBehavior:"), 1 << 7);
        }

        // Register content view for Drag & Drop file drops
        let pb_type = nsstring("public.file-url");
        let array_class = class!("NSArray");
        let array_sel = sel!("arrayWithObject:");
        let array_func: unsafe extern "C" fn(id, SEL, id) -> id =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        let types_array = array_func(array_class, array_sel, pb_type);

        let register_sel = sel!("registerForDraggedTypes:");
        let register_func: unsafe extern "C" fn(id, SEL, id) =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        register_func(ns_view, register_sel, types_array);

        // Create and set delegate
        let delegate_class = register_delegate_class();
        let delegate_alloc = msg_send_id(delegate_class, sel!("alloc"));
        let ns_delegate = msg_send_id(delegate_alloc, sel!("init"));

        msg_send_id_id_void(ns_window, sel!("setDelegate:"), ns_delegate);

        msg_send_id_id_void(ns_window, sel!("makeKeyAndOrderFront:"), nil);

        // Required for windows launched from a terminal/Cargo to become key and
        // actually appear in front of other apps on macOS.
        let ns_app = msg_send_id(class!("NSApplication"), sel!("sharedApplication"));
        let activate_sel = sel!("activateIgnoringOtherApps:");
        let activate: unsafe extern "C" fn(id, SEL, BOOL) =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        activate(ns_app, activate_sel, YES);

        msg_send_id_id_void(ns_window, sel!("makeFirstResponder:"), ns_view);

        let vsync = VsyncTracker::new();

        let layer = msg_send_id(ns_view, sel!("layer"));
        let ca_transaction = class!("CATransaction");

        let window = Window {
            ns_window,
            ns_view,
            ns_delegate,
            ns_app,
            layer,
            scale: std::cell::Cell::new(1.0),
            view_size: std::cell::Cell::new((0, 0)),
            set_contents: transmute(imp_of(layer, sel!("setContents:"))),
            set_contents_scale: transmute(imp_of(layer, sel!("setContentsScale:"))),
            ca_begin: transmute(imp_of(ca_transaction, sel!("begin"))),
            ca_set_disable_actions: transmute(imp_of(ca_transaction, sel!("setDisableActions:"))),
            ca_commit: transmute(imp_of(ca_transaction, sel!("commit"))),
            send_event: transmute(imp_of(ns_app, sel!("sendEvent:"))),
            vsync,
            input: InputState::new(),
            open: true,
            buffer: Vec::new(),
            width: 0,
            height: 0,
            use_gpu,
            caption_height: 0,
            caption_exclusions: Vec::new(),
            _marker: std::marker::PhantomData,
        };

        refresh_metrics(&raw const window as *mut Window);
        Box::pin(window)
    }
}

/// Re-read the values `present` and `framebuffer` would otherwise ask AppKit for
/// every frame. Called at creation and from the resize/backing delegate hooks.
unsafe fn refresh_metrics(window: *mut Window) {
    unsafe {
        (*window).scale.set(msg_send_f64(
            (*window).ns_window,
            sel!("backingScaleFactor"),
        ));
        let frame = msg_send_rect((*window).ns_view, sel!("frame"));
        (*window).view_size.set((
            frame.size.width.round().max(0.0) as usize,
            frame.size.height.round().max(0.0) as usize,
        ));
    }
}

impl PlatformWindow for Window {
    fn framebuffer(&mut self) -> &mut [u32] {
        let (w, h) = self.scaled_size();
        let expected_size = w * h;

        // Dynamically resize the internal buffer if the window size changes
        if self.buffer.len() != expected_size {
            self.buffer.resize(expected_size, 0);
            self.width = w;
            self.height = h;
        }

        &mut self.buffer
    }

    fn present(&self) {
        if self.use_gpu {
            return;
        }

        let (w, h) = (self.width, self.height);
        if w == 0 || h == 0 || self.buffer.is_empty() {
            return;
        }

        unsafe {
            let size = self.buffer.len() * 4;
            let pixels = Box::new(self.buffer.clone());
            let data_ptr = pixels.as_ptr() as *const std::ffi::c_void;
            let provider = CGDataProviderCreateWithData(
                Box::into_raw(pixels) as *mut std::ffi::c_void,
                data_ptr,
                size,
                Some(release_provider_data),
            );

            let screen = msg_send_id(self.ns_window, sel!("screen"));
            let color_space = if screen.is_null() {
                static FALLBACK: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
                *FALLBACK.get_or_init(|| CGColorSpaceCreateDeviceRGB() as usize) as CGColorSpaceRef
            } else {
                let ns_cs = msg_send_id(screen, sel!("colorSpace"));
                msg_send_id(ns_cs, sel!("CGColorSpace")) as CGColorSpaceRef
            };
            let bitmap_info = kCGImageAlphaNoneSkipFirst | kCGBitmapByteOrder32Little;

            let cg_image = CGImageCreate(
                w,
                h,
                8,
                32,
                w * 4,
                color_space,
                bitmap_info,
                provider,
                std::ptr::null(),
                false,
                0,
            );

            let transaction = class!("CATransaction");
            (self.ca_begin)(transaction, sel!("begin"));
            (self.ca_set_disable_actions)(transaction, sel!("setDisableActions:"), YES);
            (self.set_contents_scale)(self.layer, sel!("setContentsScale:"), self.scale.get());
            (self.set_contents)(self.layer, sel!("setContents:"), cg_image as id);
            (self.ca_commit)(transaction, sel!("commit"));

            CFRelease(cg_image as CFTypeRef);
            CFRelease(provider as CFTypeRef);
        }
    }

    fn present_damage(&self, _damage: &[Rect]) {
        self.present();
    }

    fn scale_factor(&self) -> f64 {
        self.scale.get()
    }

    fn size(&self) -> (usize, usize) {
        self.view_size.get()
    }

    fn scaled_size(&self) -> (usize, usize) {
        let (width, height) = self.size();
        let scale = self.scale_factor();
        (
            (width as f64 * scale).round() as usize,
            (height as f64 * scale).round() as usize,
        )
    }

    fn wait_for_vsync(&self) {
        self.vsync.wait_for_vsync();
    }

    fn set_cursor_visible(&self, visible: bool) {
        unsafe {
            let ns_cursor = class!("NSCursor");
            let sel = if visible {
                sel!("unhide")
            } else {
                sel!("hide")
            };
            msg_send_id(ns_cursor, sel);
        }
    }

    fn set_cursor_grab(&self, grab: bool) {
        unsafe {
            CGAssociateMouseAndMouseCursorPosition(!grab);
        }
    }

    fn set_cursor_icon(&self, icon: CursorIcon) {
        unsafe {
            let ns_cursor = class!("NSCursor");
            let selector = match icon {
                CursorIcon::Arrow => c"arrowCursor".as_ptr(),
                CursorIcon::IBeam => c"IBeamCursor".as_ptr(),
                CursorIcon::PointingHand => c"pointingHandCursor".as_ptr(),
                CursorIcon::ClosedHand => c"closedHandCursor".as_ptr(),
                CursorIcon::OpenHand => c"openHandCursor".as_ptr(),
                CursorIcon::Crosshair => c"crosshairCursor".as_ptr(),
                CursorIcon::ResizeLeftRight => c"resizeLeftRightCursor".as_ptr(),
                CursorIcon::ResizeUpDown => c"resizeUpDownCursor".as_ptr(),
                CursorIcon::AutoScroll => c"resizeUpDownCursor".as_ptr(),
                CursorIcon::AutoScrollUp => c"resizeUpCursor".as_ptr(),
                CursorIcon::AutoScrollDown => c"resizeDownCursor".as_ptr(),
            };
            let cursor_sel = sel_registerName(selector as *const _);
            let cursor = msg_send_id(ns_cursor, cursor_sel);
            if !cursor.is_null() {
                msg_send_id(cursor, sel!("set"));
            }
        }
    }

    fn get_clipboard_text(&self) -> Option<String> {
        unsafe {
            let pb_class = class!("NSPasteboard");
            let pb = msg_send_id(pb_class, sel!("generalPasteboard"));
            if pb.is_null() {
                return None;
            }

            let type_ns = nsstring("public.utf8-plain-text");
            let string_sel = sel!("stringForType:");
            let string_func: unsafe extern "C" fn(id, SEL, id) -> id =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            let ns_string = string_func(pb, string_sel, type_ns);

            if ns_string.is_null() {
                return None;
            }

            let utf8_func: unsafe extern "C" fn(id, SEL) -> *const std::os::raw::c_char =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            let utf8_ptr = utf8_func(ns_string, sel!("UTF8String"));
            if utf8_ptr.is_null() {
                return None;
            }

            let c_str = std::ffi::CStr::from_ptr(utf8_ptr);
            c_str.to_str().ok().map(|s| s.to_string())
        }
    }

    fn set_clipboard_text(&self, text: &str) {
        unsafe {
            let pb_class = class!("NSPasteboard");
            let pb = msg_send_id(pb_class, sel!("generalPasteboard"));
            if pb.is_null() {
                return;
            }

            msg_send_id(pb, sel!("clearContents"));

            let type_ns = nsstring("public.utf8-plain-text");
            let text_ns = nsstring(text);

            let set_sel = sel!("setString:forType:");
            let set_func: unsafe extern "C" fn(id, SEL, id, id) -> BOOL =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            set_func(pb, set_sel, text_ns, type_ns);
        }
    }

    fn set_title(&self, title: &str) {
        unsafe {
            let title_ns = nsstring(title);
            msg_send_id_id_void(self.ns_window, sel!("setTitle:"), title_ns);
        }
    }

    fn focused(&self) -> bool {
        unsafe {
            let func: unsafe extern "C" fn(id, SEL) -> BOOL =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            func(self.ns_window, sel!("isKeyWindow")) != 0
        }
    }

    fn window_style(&mut self, style: WindowStyle) {
        let mask = match style {
            WindowStyle::Standard => {
                NSWindowStyleMaskTitled
                    | NSWindowStyleMaskClosable
                    | NSWindowStyleMaskMiniaturizable
                    | NSWindowStyleMaskResizable
            }
            WindowStyle::Borderless | WindowStyle::Transparent => NSWindowStyleMaskBorderless,
        };
        unsafe {
            msg_send_id_usize_void(self.ns_window, sel!("setStyleMask:"), mask);
            refresh_metrics(self as *mut Window);
        };
    }

    fn fullscreen_mode(&mut self, mode: Fullscreen) {
        match mode {
            Fullscreen::None => self.window_style(WindowStyle::Standard),
            Fullscreen::Workspace => unsafe {
                msg_send_id_id_void(
                    self.ns_window,
                    sel!("toggleFullScreen:"),
                    std::ptr::null_mut(),
                );
                refresh_metrics(self as *mut Window);
            },
            Fullscreen::Monitor => {
                self.window_style(WindowStyle::Borderless);
                unsafe {
                    let screen = msg_send_id(class!("NSScreen"), sel!("mainScreen"));
                    let frame = msg_send_rect(screen, sel!("frame"));
                    let set_frame: unsafe extern "C" fn(id, SEL, NSRect, BOOL) =
                        std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                    set_frame(self.ns_window, sel!("setFrame:display:"), frame, YES);
                    refresh_metrics(self as *mut Window);
                }
            }
        }
    }

    fn draw<F>(&mut self, mut render: F)
    where
        F: FnMut(&mut Self),
    {
        #[cfg(debug_assertions)]
        assert_main_thread();

        // Refresh once here so size(), scale_factor() and framebuffer() all agree
        // for the whole frame.
        unsafe { refresh_metrics(self as *mut Window) };

        self.input.begin_frame();

        // AppKit only reports the cursor while it is over the window or a button is held, so poll
        // it every frame. Hover falls off on its own because the position leaves the content view.
        unsafe {
            self.input.mouse_pos = Some(content_point(self.ns_window, cursor_position()));
        }

        // Store the closure in thread-local storage
        let repaint_ptr = &mut render as *mut F as *mut std::ffi::c_void;
        let repaint_func = |ptr: *mut std::ffi::c_void, window: &mut Window| unsafe {
            window.input.begin_frame();
            window.input.mouse_pos = Some(content_point(window.ns_window, cursor_position()));
            if !LIVE_RESIZE.with(|r| r.get()) {
                window.input.sync_mouse_buttons(pressed_mouse_buttons());
            }
            let f = &mut *(ptr as *mut F);
            f(window);
        };

        REPAINT_CALLBACK.with(|c| c.set(Some(repaint_ptr)));
        REPAINT_FUNC.with(|f| f.set(Some(repaint_func)));
        ACTIVE_WINDOW.with(|w| w.set(self as *mut Window));

        unsafe {
            let ns_app = self.ns_app;

            // Scope an autorelease pool to this tick. The C entry points do the same
            // work as NSAutoreleasePool alloc/init/drain without the object or the
            // three message sends.
            let pool = objc_autoreleasePoolPush();

            let distant_past = msg_send_id(class!("NSDate"), sel!("distantPast"));

            let next_event_sel = sel!("nextEventMatchingMask:untilDate:inMode:dequeue:");

            let next_event_func: unsafe extern "C" fn(id, SEL, u64, id, id, BOOL) -> id =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);

            // NSEvent type and window vary by NSEvent subclass, so these stay on
            // objc_msgSend. The selectors are still resolved once by sel!.
            let type_sel = sel!("type");
            let window_sel = sel!("window");
            let event_type_func: unsafe extern "C" fn(id, SEL) -> NSEventType =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);

            loop {
                let event = next_event_func(
                    ns_app,
                    next_event_sel,
                    NSEventMaskAny,
                    distant_past,
                    kCFRunLoopDefaultMode as id,
                    YES,
                );

                if event.is_null() {
                    break;
                }

                translate_event(event, &mut self.input);

                let event_type = event_type_func(event, type_sel);
                let starts_mouse_tracking = matches!(
                    event_type,
                    NSEventTypeLeftMouseDown | NSEventTypeRightMouseDown
                );

                if starts_mouse_tracking {
                    let event_window = msg_send_id(event, window_sel);
                    if event_window == self.ns_window {
                        start_tracking_repaint_timer(self.ns_window, self.ns_delegate);
                    }
                }

                // Dispatch event to targets. AppKit can block here inside border/titlebar tracking.
                (self.send_event)(ns_app, sel!("sendEvent:"), event);

                if starts_mouse_tracking {
                    stop_tracking_repaint_timer();
                }
            }

            objc_autoreleasePoolPop(pool);
        }

        REPAINT_CALLBACK.with(|c| c.set(None));
        REPAINT_FUNC.with(|f| f.set(None));

        // ACTIVE_WINDOW stays set across `render` so that AppKit calls made from
        // user code (zoom:, toggleFullScreen:, ...) still reach the delegate
        // hooks that refresh the cached size and scale.
        render(self);

        ACTIVE_WINDOW.with(|w| w.set(std::ptr::null_mut()));
    }

    fn open(&self) -> bool {
        self.open
    }

    fn close(&mut self) {
        if self.open {
            self.open = false;
            unsafe {
                msg_send_id(self.ns_window, sel!("close"));
            }
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

    fn wait_for_event(&self) {
        // Block the current thread until the OS delivers any window event.
        // We peek with `dequeue: NO` so the event stays in the queue for the
        // draw loop to process on the very next frame.
        unsafe {
            let ns_app = self.ns_app;

            let distant_future = msg_send_id(class!("NSDate"), sel!("distantFuture"));

            let next_event_func: unsafe extern "C" fn(id, SEL, u64, id, id, BOOL) -> id =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);

            // Block until an event arrives. dequeue: NO means we don't consume
            // the event here — the draw loop will pick it up on the next tick.
            next_event_func(
                ns_app,
                sel!("nextEventMatchingMask:untilDate:inMode:dequeue:"),
                NSEventMaskAny,
                distant_future,
                kCFRunLoopDefaultMode as id,
                NO,
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

        let custom = height > 0;
        unsafe {
            let mask_func: unsafe extern "C" fn(id, SEL) -> NSWindowStyleMask =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            let mut mask = mask_func(self.ns_window, sel!("styleMask"));
            if custom {
                mask |= NSWindowStyleMaskTitled
                    | NSWindowStyleMaskClosable
                    | NSWindowStyleMaskMiniaturizable
                    | NSWindowStyleMaskResizable
                    | NSWindowStyleMaskFullSizeContentView;
            } else {
                mask &= !NSWindowStyleMaskFullSizeContentView;
            }
            msg_send_id_usize_void(self.ns_window, sel!("setStyleMask:"), mask);

            msg_send_id_bool_void(
                self.ns_window,
                sel!("setTitlebarAppearsTransparent:"),
                if custom { YES } else { NO },
            );
            msg_send_id_usize_void(
                self.ns_window,
                sel!("setTitleVisibility:"),
                if custom { 1 } else { 0 },
            );

            let button_func: unsafe extern "C" fn(id, SEL, usize) -> id =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            let button_sel = sel!("standardWindowButton:");
            for kind in 0..3 {
                let button = button_func(self.ns_window, button_sel, kind);
                if !button.is_null() {
                    msg_send_id_bool_void(
                        button,
                        sel!("setHidden:"),
                        if custom { YES } else { NO },
                    );
                }
            }

            msg_send_id_id_void(self.ns_window, sel!("makeFirstResponder:"), self.ns_view);

            refresh_metrics(self as *mut Window);
        }
    }

    fn show(&mut self) {
        unsafe {
            msg_send_id_id_void(self.ns_window, sel!("makeKeyAndOrderFront:"), nil);
            refresh_metrics(self as *mut Window);
        }
    }

    fn hide(&mut self) {
        unsafe { msg_send_id_id_void(self.ns_window, sel!("orderOut:"), nil) };
    }

    fn set_size(&mut self, width: i32, height: i32) {
        unsafe {
            let size = NSSize {
                width: width as f64,
                height: height as f64,
            };
            let set_content_size: unsafe extern "C" fn(id, SEL, NSSize) =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            set_content_size(self.ns_window, sel!("setContentSize:"), size);
            refresh_metrics(self as *mut Window);
        }
    }

    fn minimize(&mut self) {
        unsafe { msg_send_id_id_void(self.ns_window, sel!("miniaturize:"), nil) };
    }

    fn toggle_maximize(&mut self) {
        unsafe { msg_send_id_id_void(self.ns_window, sel!("zoom:"), nil) };
    }

    fn maximized(&self) -> bool {
        unsafe {
            let func: unsafe extern "C" fn(id, SEL) -> BOOL =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            func(self.ns_window, sel!("isZoomed")) != NO
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        assert_main_thread();
        unsafe {
            msg_send_id_id_void(self.ns_window, sel!("setDelegate:"), nil);
            msg_send_id_bool_void(self.ns_window, sel!("setReleasedWhenClosed:"), YES);
            msg_send_id(self.ns_window, sel!("close"));
            msg_send_id(self.ns_delegate, sel!("release"));
        }
    }
}

unsafe extern "C" fn release_provider_data(
    info: *mut std::ffi::c_void,
    _data: *const std::ffi::c_void,
    _size: usize,
) {
    unsafe { drop(Box::from_raw(info as *mut Vec<u32>)) }
}

fn parse_modifiers(flags: usize) -> Modifiers {
    Modifiers {
        shift: (flags & (1 << 17)) != 0,
        ctrl: (flags & (1 << 18)) != 0,
        alt: (flags & (1 << 19)) != 0,
        logo: (flags & (1 << 20)) != 0,
    }
}

unsafe fn content_point(ns_window: id, screen_point: NSPoint) -> (f64, f64) {
    unsafe {
        let point = msg_send_point_point(ns_window, sel!("convertPointFromScreen:"), screen_point);
        let content_view = msg_send_id(ns_window, sel!("contentView"));
        let frame = msg_send_rect(content_view, sel!("frame"));
        (point.x, frame.size.height - point.y)
    }
}

unsafe fn pressed_mouse_buttons() -> usize {
    unsafe {
        let func: unsafe extern "C" fn(id, SEL) -> usize =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        func(class!("NSEvent"), sel!("pressedMouseButtons"))
    }
}

unsafe fn cursor_position() -> NSPoint {
    unsafe { msg_send_point(class!("NSEvent"), sel!("mouseLocation")) }
}

unsafe fn mouse_location(ns_event: id) -> Option<(f64, f64)> {
    unsafe {
        let active = ACTIVE_WINDOW.with(|w| w.get());
        if active.is_null() {
            return None;
        }

        let mut point = msg_send_point(ns_event, sel!("locationInWindow"));
        let event_window = msg_send_id(ns_event, sel!("window"));

        // Events raised outside the window carry no window and are already in screen space.
        if !event_window.is_null() {
            point = msg_send_point_point(event_window, sel!("convertPointToScreen:"), point);
        }

        Some(content_point((*active).ns_window, point))
    }
}

unsafe fn mouse_from_macos_event(ns_event: id, event_type: NSEventType) -> Option<Mouse> {
    unsafe {
        match event_type {
            NSEventTypeLeftMouseDown | NSEventTypeLeftMouseUp | NSEventTypeLeftMouseDragged => {
                Some(Mouse::Left)
            }
            NSEventTypeRightMouseDown | NSEventTypeRightMouseUp | NSEventTypeRightMouseDragged => {
                Some(Mouse::Right)
            }
            NSEventTypeOtherMouseDown | NSEventTypeOtherMouseUp | NSEventTypeOtherMouseDragged => {
                let button_number_sel = sel!("buttonNumber");
                let button_number_func: unsafe extern "C" fn(id, SEL) -> isize =
                    std::mem::transmute(objc_msgSend as *const std::ffi::c_void);

                match button_number_func(ns_event, button_number_sel) {
                    2 => Some(Mouse::Middle),
                    3 => Some(Mouse::Back),
                    4 => Some(Mouse::Forward),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

unsafe fn msg_send_f64(receiver: id, selector: SEL) -> f64 {
    let func: unsafe extern "C" fn(id, SEL) -> f64 =
        unsafe { std::mem::transmute(objc_msgSend as *const std::ffi::c_void) };
    unsafe { func(receiver, selector) }
}

unsafe fn translate_event(ns_event: id, input: &mut InputState) {
    unsafe {
        let event_type_sel = sel!("type");
        let event_type_func: unsafe extern "C" fn(id, SEL) -> NSEventType =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        let event_type = event_type_func(ns_event, event_type_sel);

        let modifier_flags_sel = sel!("modifierFlags");
        let modifier_flags_func: unsafe extern "C" fn(id, SEL) -> usize =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        let flags = modifier_flags_func(ns_event, modifier_flags_sel);
        let modifiers = parse_modifiers(flags);
        input.modifiers = modifiers;

        match event_type {
            NSEventTypeKeyDown | NSEventTypeKeyUp => {
                let key_code_sel = sel!("keyCode");
                let key_code_func: unsafe extern "C" fn(id, SEL) -> u16 =
                    std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                let key = Key::from_macos_keycode(key_code_func(ns_event, key_code_sel));

                if event_type == NSEventTypeKeyDown {
                    input.set_key_down(key);

                    // Extract text input characters
                    let chars_ns = msg_send_id(ns_event, sel!("characters"));
                    if !chars_ns.is_null() {
                        let len_func: unsafe extern "C" fn(id, SEL) -> usize =
                            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                        let len = len_func(chars_ns, sel!("length"));
                        if len > 0 {
                            let utf8_func: unsafe extern "C" fn(
                                id,
                                SEL,
                            )
                                -> *const std::os::raw::c_char =
                                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                            let utf8_ptr = utf8_func(chars_ns, sel!("UTF8String"));
                            if !utf8_ptr.is_null() {
                                let c_str = std::ffi::CStr::from_ptr(utf8_ptr);
                                if let Ok(s) = c_str.to_str() {
                                    for c in s.chars() {
                                        if !c.is_control() {
                                            input.text_input.push(c);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    input.set_key_up(key);
                }
            }
            NSEventTypeLeftMouseDown
            | NSEventTypeLeftMouseUp
            | NSEventTypeRightMouseDown
            | NSEventTypeRightMouseUp
            | NSEventTypeOtherMouseDown
            | NSEventTypeOtherMouseUp => {
                input.mouse_pos = mouse_location(ns_event);
                if let Some(button) = mouse_from_macos_event(ns_event, event_type) {
                    if event_type == NSEventTypeLeftMouseDown
                        || event_type == NSEventTypeRightMouseDown
                        || event_type == NSEventTypeOtherMouseDown
                    {
                        // AppKit reports multi-click count using system double-click interval.
                        let click_count_sel = sel!("clickCount");
                        let click_count_func: unsafe extern "C" fn(id, SEL) -> isize =
                            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                        let click_count = click_count_func(ns_event, click_count_sel);
                        if click_count >= 2 {
                            input.set_mouse_double_down(button);
                        } else {
                            input.set_mouse_down(button);
                        }
                    } else {
                        input.set_mouse_up(button);
                    }
                }
            }
            NSEventTypeMouseMoved => {
                let delta_x = msg_send_f64(ns_event, sel!("deltaX"));
                let delta_y = msg_send_f64(ns_event, sel!("deltaY"));

                input.mouse_pos = mouse_location(ns_event);
                input.raw_mouse_delta.0 += delta_x;
                input.raw_mouse_delta.1 += -delta_y;
            }
            NSEventTypeLeftMouseDragged
            | NSEventTypeRightMouseDragged
            | NSEventTypeOtherMouseDragged => {
                let delta_x = msg_send_f64(ns_event, sel!("deltaX"));
                let delta_y = msg_send_f64(ns_event, sel!("deltaY"));

                input.mouse_pos = mouse_location(ns_event);
                input.raw_mouse_delta.0 += delta_x;
                input.raw_mouse_delta.1 += -delta_y;
                if let Some(button) = mouse_from_macos_event(ns_event, event_type) {
                    input.set_mouse_down(button);
                }
            }
            NSEventTypeScrollWheel => {
                let dx_sel = sel!("scrollingDeltaX");
                let dy_sel = sel!("scrollingDeltaY");
                let double_func: unsafe extern "C" fn(id, SEL) -> f64 =
                    std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                let delta_x = double_func(ns_event, dx_sel);
                let delta_y = double_func(ns_event, dy_sel);
                input.scroll_delta.0 += delta_x;
                input.scroll_delta.1 += delta_y;

                // A scroll event carries the cursor position, and on a fresh window it is the only
                // event that arrives before the mouse has moved. Without this there is no position
                // to hit test against, so every scroll is dropped until the mouse moves or clicks.
                input.mouse_pos = mouse_location(ns_event);

                let usize_func: unsafe extern "C" fn(id, SEL) -> usize =
                    std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                let bool_func: unsafe extern "C" fn(id, SEL) -> bool =
                    std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                let phase = usize_func(ns_event, sel!("phase"));
                let momentum = usize_func(ns_event, sel!("momentumPhase"));
                let precise = bool_func(ns_event, sel!("hasPreciseScrollingDeltas"));

                // A gesture reports `phase` and leaves `momentumPhase` at None, then the two swap
                // once the fingers lift and the OS takes over the fling.
                let phase = match (phase, momentum) {
                    (_, NSEventPhaseBegan) => ScrollPhase::MomentumBegan,
                    (_, NSEventPhaseChanged) => ScrollPhase::MomentumChanged,
                    (_, NSEventPhaseEnded) | (_, NSEventPhaseCancelled) => {
                        ScrollPhase::MomentumEnded
                    }
                    (NSEventPhaseBegan, _) | (NSEventPhaseMayBegin, _) => ScrollPhase::Began,
                    (NSEventPhaseChanged, _) | (NSEventPhaseStationary, _) => ScrollPhase::Changed,
                    (NSEventPhaseEnded, _) | (NSEventPhaseCancelled, _) => ScrollPhase::Ended,
                    _ => ScrollPhase::None,
                };

                input.scroll_events.push(ScrollEvent {
                    delta: (delta_x, delta_y),
                    phase,
                    precise,
                    timestamp: double_func(ns_event, sel!("timestamp")),
                });
            }
            _ => {}
        }
    }
}

static REGISTER_DELEGATE: std::sync::Once = std::sync::Once::new();

pub fn register_delegate_class() -> Class {
    let mut cls = std::ptr::null_mut();
    REGISTER_DELEGATE.call_once(|| unsafe {
        let superclass = class!("NSObject");
        cls = objc_allocateClassPair(superclass, c"RustWindowDelegate".as_ptr() as *const _, 0);

        class_addMethod(
            cls,
            sel!("windowShouldClose:"),
            std::mem::transmute(window_should_close as *const std::ffi::c_void),
            c"c@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("windowDidResize:"),
            std::mem::transmute(window_did_resize as *const std::ffi::c_void),
            c"v@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("windowWillStartLiveResize:"),
            std::mem::transmute(window_will_start_live_resize as *const std::ffi::c_void),
            c"v@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("windowDidEndLiveResize:"),
            std::mem::transmute(window_did_end_live_resize as *const std::ffi::c_void),
            c"v@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("liveResizeTick:"),
            std::mem::transmute(live_resize_tick as *const std::ffi::c_void),
            c"v@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("windowDidBecomeKey:"),
            std::mem::transmute(window_did_become_key as *const std::ffi::c_void),
            c"v@:@".as_ptr() as *const _,
        );

        objc_registerClassPair(cls);
    });
    if cls.is_null() {
        class!("RustWindowDelegate")
    } else {
        cls
    }
}

extern "C" fn window_did_become_key(_this: id, _cmd: SEL, notification: id) {
    unsafe {
        let window: id = msg_send_id(notification, sel!("object"));
        let content_view = msg_send_id(window, sel!("contentView"));
        if !content_view.is_null() {
            msg_send_id_id_void(window, sel!("makeFirstResponder:"), content_view);
        }
    }
}

extern "C" fn window_should_close(_this: id, _cmd: SEL, _sender: id) -> BOOL {
    ACTIVE_WINDOW.with(|w| {
        let window = w.get();
        if !window.is_null() {
            unsafe {
                (*window).open = false;
            }
        }
    });
    YES
}

unsafe fn repaint_window(window: id) {
    unsafe {
        let callback_opt = REPAINT_CALLBACK.with(|c| c.get());
        let func_opt = REPAINT_FUNC.with(|f| f.get());
        let active_window = ACTIVE_WINDOW.with(|w| w.get());

        if let (Some(ptr), Some(func)) = (callback_opt, func_opt) {
            if !active_window.is_null() && (*active_window).ns_window == window {
                refresh_metrics(active_window);
                func(ptr, &mut *active_window);
            }
        }
    }
}

unsafe fn start_tracking_repaint_timer(window: id, target: id) {
    unsafe {
        if REPAINT_CALLBACK.with(|c| c.get()).is_none() {
            return;
        }

        TRACKING_REPAINT_TIMER.with(|cell| {
            let old_timer = cell.get();
            if !old_timer.is_null() {
                msg_send_id(old_timer, sel!("invalidate"));
            }

            let timer_class = class!("NSTimer");
            let timer_sel = sel!("timerWithTimeInterval:target:selector:userInfo:repeats:");
            let make_timer: unsafe extern "C" fn(id, SEL, f64, id, SEL, id, BOOL) -> id =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            let timer = make_timer(
                timer_class,
                timer_sel,
                repaint_interval_for_window(window),
                target,
                sel!("liveResizeTick:"),
                window,
                YES,
            );

            let run_loop = msg_send_id(class!("NSRunLoop"), sel!("currentRunLoop"));
            let add_timer_sel = sel!("addTimer:forMode:");
            let add_timer: unsafe extern "C" fn(id, SEL, id, id) =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            add_timer(
                run_loop,
                add_timer_sel,
                timer,
                nsstring("NSEventTrackingRunLoopMode"),
            );
            add_timer(
                run_loop,
                add_timer_sel,
                timer,
                nsstring("NSRunLoopCommonModes"),
            );

            cell.set(timer);
        });
    }
}

unsafe fn repaint_interval_for_window(window: id) -> f64 {
    unsafe {
        let mut screen = msg_send_id(window, sel!("screen"));
        if screen.is_null() {
            screen = msg_send_id(class!("NSScreen"), sel!("mainScreen"));
        }

        let fps_sel = sel!("maximumFramesPerSecond");
        let responds_to_sel = sel!("respondsToSelector:");
        let responds_to: unsafe extern "C" fn(id, SEL, SEL) -> BOOL =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);

        if !screen.is_null() && responds_to(screen, responds_to_sel, fps_sel) == YES {
            let fps_func: unsafe extern "C" fn(id, SEL) -> isize =
                std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
            let fps = fps_func(screen, fps_sel);
            if fps > 0 {
                return 1.0 / fps as f64;
            }
        }

        1.0 / 60.0
    }
}

fn stop_tracking_repaint_timer() {
    TRACKING_REPAINT_TIMER.with(|cell| {
        let timer = cell.replace(nil);
        if !timer.is_null() {
            unsafe {
                msg_send_id(timer, sel!("invalidate"));
            }
        }
    });
}

extern "C" fn window_will_start_live_resize(_this: id, _cmd: SEL, notification: id) {
    unsafe {
        let window: id = msg_send_id(notification, sel!("object"));

        LIVE_RESIZE.with(|r| r.set(true));

        let active = ACTIVE_WINDOW.with(|w| w.get());
        if !active.is_null() {
            (*active).input.cancel_mouse();
        }

        start_tracking_repaint_timer(window, _this);
    }
}

extern "C" fn window_did_end_live_resize(_this: id, _cmd: SEL, _notification: id) {
    LIVE_RESIZE.with(|r| r.set(false));
    stop_tracking_repaint_timer();
}

extern "C" fn live_resize_tick(_this: id, _cmd: SEL, timer: id) {
    unsafe {
        let window = msg_send_id(timer, sel!("userInfo"));
        if !window.is_null() {
            repaint_window(window);
        }
    }
}

extern "C" fn window_did_resize(_this: id, _cmd: SEL, notification: id) {
    unsafe {
        let window: id = msg_send_id(notification, sel!("object"));
        // Repaint while AppKit is inside its live-resize tracking loop.
        repaint_window(window);
    }
}

static REGISTER_VIEW: std::sync::Once = std::sync::Once::new();

pub fn register_view_class() -> Class {
    let mut cls = std::ptr::null_mut();
    REGISTER_VIEW.call_once(|| unsafe {
        let superclass = class!("NSView");
        cls = objc_allocateClassPair(superclass, c"RustView".as_ptr() as *const _, 0);

        // Bind drag-and-drop destination methods directly to our custom view class
        class_addMethod(
            cls,
            sel!("draggingEntered:"),
            std::mem::transmute(dragging_entered as *const std::ffi::c_void),
            c"Q@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("performDragOperation:"),
            std::mem::transmute(perform_drag_operation as *const std::ffi::c_void),
            c"c@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("acceptsFirstResponder"),
            std::mem::transmute(view_accepts_first_responder as *const std::ffi::c_void),
            c"c@:".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("keyDown:"),
            std::mem::transmute(view_key_down as *const std::ffi::c_void),
            c"v@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("keyUp:"),
            std::mem::transmute(view_key_up as *const std::ffi::c_void),
            c"v@:@".as_ptr() as *const _,
        );

        class_addMethod(
            cls,
            sel!("mouseDown:"),
            std::mem::transmute(view_mouse_down as *const std::ffi::c_void),
            c"v@:@".as_ptr() as *const _,
        );

        objc_registerClassPair(cls);
    });

    if cls.is_null() {
        class!("RustView")
    } else {
        cls
    }
}

extern "C" fn view_accepts_first_responder(_this: id, _cmd: SEL) -> BOOL {
    YES
}

extern "C" fn view_key_down(_this: id, _cmd: SEL, _event: id) {
    // Consume keyboard events so AppKit does not NSBeep() for unhandled keys.
}

extern "C" fn view_key_up(_this: id, _cmd: SEL, _event: id) {
    // Consume keyboard events so AppKit does not NSBeep() for unhandled keys.
}

extern "C" fn view_mouse_down(this: id, _cmd: SEL, event: id) {
    unsafe {
        let window = msg_send_id(this, sel!("window"));
        if window.is_null() {
            return;
        }
        msg_send_id_id_void(window, sel!("makeFirstResponder:"), this);

        // A click in the caption strip is handed back to AppKit so the window drags, snaps and
        // zooms on double click exactly like a native title bar would.
        let active = ACTIVE_WINDOW.with(|w| w.get());
        if active.is_null() || (*active).ns_window != window || (*active).caption_height == 0 {
            return;
        }
        let Some((x, y)) = mouse_location(event) else {
            return;
        };
        let (x, y) = (x as i32, y as i32);
        if y >= (*active).caption_height
            || (*active)
                .caption_exclusions
                .iter()
                .any(|rect| rect.contains(x, y))
        {
            return;
        }

        msg_send_id_id_void(window, sel!("performWindowDragWithEvent:"), event);
    }
}

extern "C" fn dragging_entered(_this: id, _cmd: SEL, _sender: id) -> usize {
    1 // NSDragOperationGeneric
}

extern "C" fn perform_drag_operation(_this: id, _cmd: SEL, sender: id) -> BOOL {
    unsafe {
        let pb_sel = sel!("draggingPasteboard");
        let pb = msg_send_id(sender, pb_sel);
        if pb.is_null() {
            return NO;
        }

        // Read file URLs from the pasteboard
        let url_class = class!("NSURL");
        let class_array_class = class!("NSArray");
        let array_sel = sel!("arrayWithObject:");
        let array_func: unsafe extern "C" fn(id, SEL, id) -> id =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        let classes = array_func(class_array_class, array_sel, url_class);

        let read_sel = sel!("readObjectsForClasses:options:");
        let read_func: unsafe extern "C" fn(id, SEL, id, id) -> id =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        let urls = read_func(pb, read_sel, classes, std::ptr::null_mut());

        if urls.is_null() {
            return NO;
        }

        let count_sel = sel!("count");
        let count_func: unsafe extern "C" fn(id, SEL) -> usize =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        let count = count_func(urls, count_sel);

        let mut file_paths = Vec::new();
        let object_at_index_sel = sel!("objectAtIndex:");
        let object_func: unsafe extern "C" fn(id, SEL, usize) -> id =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);

        for i in 0..count {
            let url = object_func(urls, object_at_index_sel, i);
            if !url.is_null() {
                let path_sel = sel!("path");
                let path_ns = msg_send_id(url, path_sel);
                if !path_ns.is_null() {
                    let utf8_func: unsafe extern "C" fn(id, SEL) -> *const std::os::raw::c_char =
                        std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
                    let utf8_ptr = utf8_func(path_ns, sel!("UTF8String"));
                    if !utf8_ptr.is_null() {
                        let c_str = std::ffi::CStr::from_ptr(utf8_ptr);
                        if let Ok(s) = c_str.to_str() {
                            file_paths.push(PathBuf::from(s));
                        }
                    }
                }
            }
        }

        if file_paths.is_empty() {
            return NO;
        }

        ACTIVE_WINDOW.with(|w| {
            let window = w.get();
            if !window.is_null() {
                (*window).input.dropped_files.extend(file_paths);
            }
        });
        YES
    }
}

pub fn assert_main_thread() {
    unsafe {
        let thread_class = class!("NSThread");
        let is_main_sel = sel!("isMainThread");
        let func: unsafe extern "C" fn(id, SEL) -> BOOL =
            std::mem::transmute(objc_msgSend as *const std::ffi::c_void);
        let is_main = func(thread_class, is_main_sel);
        if is_main == NO {
            panic!("AppKit functions must be called from the main thread!");
        }
    }
}
