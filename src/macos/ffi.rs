#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

use std::os::raw::{c_char, c_schar};

// Objective-C Opaque types
pub type id = *mut std::ffi::c_void;
pub type SEL = *mut std::ffi::c_void;
pub type Class = *mut std::ffi::c_void;

// Objective-C BOOL
pub type BOOL = c_schar;
pub const YES: BOOL = 1;
pub const NO: BOOL = 0;

pub const nil: id = std::ptr::null_mut();

// Core Graphics / Quartz Core / Apple Geometry types
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct NSPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct NSSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct NSRect {
    pub origin: NSPoint,
    pub size: NSSize,
}

impl NSRect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: NSPoint { x, y },
            size: NSSize { width, height },
        }
    }
}

// NSWindowStyleMask
pub type NSWindowStyleMask = usize;
pub const NSWindowStyleMaskBorderless: NSWindowStyleMask = 0;
pub const NSWindowStyleMaskTitled: NSWindowStyleMask = 1 << 0;
pub const NSWindowStyleMaskClosable: NSWindowStyleMask = 1 << 1;
pub const NSWindowStyleMaskMiniaturizable: NSWindowStyleMask = 1 << 2;
pub const NSWindowStyleMaskResizable: NSWindowStyleMask = 1 << 3;
pub const NSWindowStyleMaskFullScreen: NSWindowStyleMask = 1 << 14;
pub const NSWindowStyleMaskFullSizeContentView: NSWindowStyleMask = 1 << 15;

// NSBackingStoreType
pub type NSBackingStoreType = usize;
pub const NSBackingStoreBuffered: NSBackingStoreType = 2;

// NSEventType
pub type NSEventType = usize;
pub const NSEventTypeLeftMouseDown: NSEventType = 1;
pub const NSEventTypeLeftMouseUp: NSEventType = 2;
pub const NSEventTypeRightMouseDown: NSEventType = 3;
pub const NSEventTypeRightMouseUp: NSEventType = 4;
pub const NSEventTypeMouseMoved: NSEventType = 5;
pub const NSEventTypeLeftMouseDragged: NSEventType = 6;
pub const NSEventTypeRightMouseDragged: NSEventType = 7;
pub const NSEventTypeKeyDown: NSEventType = 10;
pub const NSEventTypeKeyUp: NSEventType = 11;
pub const NSEventTypeScrollWheel: NSEventType = 22;
pub const NSEventTypeOtherMouseDown: NSEventType = 25;
pub const NSEventTypeOtherMouseUp: NSEventType = 26;
pub const NSEventTypeOtherMouseDragged: NSEventType = 27;

pub const NSEventMaskAny: u64 = std::u64::MAX;

// Activation policy
pub type NSApplicationActivationPolicy = isize;
pub const NSApplicationActivationPolicyRegular: NSApplicationActivationPolicy = 0;

// Linker configuration for zero dependencies
#[link(name = "objc")]
#[link(name = "Foundation", kind = "framework")]
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    pub fn objc_getClass(name: *const c_char) -> Class;
    pub fn sel_registerName(name: *const c_char) -> SEL;

    // The main dispatch function
    pub fn objc_msgSend();

    #[cfg(target_arch = "x86_64")]
    pub fn objc_msgSend_stret();

    // Class creation functions for delegates
    pub fn objc_allocateClassPair(
        superclass: Class,
        name: *const c_char,
        extra_bytes: usize,
    ) -> Class;
    pub fn class_addMethod(
        cls: Class,
        name: SEL,
        imp: extern "C" fn(),
        types: *const c_char,
    ) -> bool;
    pub fn objc_registerClassPair(cls: Class);
}

// Core Graphics Types
pub type CGColorSpaceRef = *mut std::ffi::c_void;
pub type CGDataProviderRef = *mut std::ffi::c_void;
pub type CGImageRef = *mut std::ffi::c_void;
pub type CFTypeRef = *mut std::ffi::c_void;
pub type CFStringRef = *const std::ffi::c_void;

// Core Graphics Constants
pub const kCGImageAlphaNoneSkipFirst: u32 = 6;
pub const kCGBitmapByteOrder32Little: u32 = 2 << 12;

// CoreGraphics & CoreFoundation FFI
#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {
    pub static kCGColorSpaceSRGB: CFStringRef;
    pub fn CGColorSpaceCreateWithName(name: CFStringRef) -> CGColorSpaceRef;
    pub fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;
    pub fn CGDataProviderCreateWithData(
        info: *mut std::ffi::c_void,
        data: *const std::ffi::c_void,
        size: usize,
        releaseData: Option<
            unsafe extern "C" fn(*mut std::ffi::c_void, *const std::ffi::c_void, usize),
        >,
    ) -> CGDataProviderRef;
    pub fn CGImageCreate(
        width: usize,
        height: usize,
        bitsPerComponent: usize,
        bitsPerPixel: usize,
        bytesPerRow: usize,
        space: CGColorSpaceRef,
        bitmapInfo: u32,
        provider: CGDataProviderRef,
        decode: *const f64,
        shouldInterpolate: bool,
        intent: i32,
    ) -> CGImageRef;
    pub fn CFRelease(cf: CFTypeRef);
}

// CoreVideo types and functions
pub type CVDisplayLinkRef = *mut std::ffi::c_void;
pub type CVReturn = i32;

pub type CVDisplayLinkOutputCallback = unsafe extern "C" fn(
    displayLink: CVDisplayLinkRef,
    inNow: *const std::ffi::c_void,
    inOutputTime: *const std::ffi::c_void,
    flagsIn: u64,
    flagsOut: *mut u64,
    displayLinkContext: *mut std::ffi::c_void,
) -> CVReturn;

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    pub fn CVDisplayLinkCreateWithActiveCGDisplays(
        displayLinkOut: *mut CVDisplayLinkRef,
    ) -> CVReturn;
    pub fn CVDisplayLinkSetOutputCallback(
        displayLink: CVDisplayLinkRef,
        callback: CVDisplayLinkOutputCallback,
        userInfo: *mut std::ffi::c_void,
    ) -> CVReturn;
    pub fn CVDisplayLinkStart(displayLink: CVDisplayLinkRef) -> CVReturn;
    pub fn CVDisplayLinkStop(displayLink: CVDisplayLinkRef) -> CVReturn;
    pub fn CVDisplayLinkRelease(displayLink: CVDisplayLinkRef);
}

// Grand Central Dispatch (GCD) Semaphores
pub type dispatch_semaphore_t = *mut std::ffi::c_void;
pub const DISPATCH_TIME_FOREVER: u64 = !0u64;
pub const DISPATCH_TIME_NOW: u64 = 0;

unsafe extern "C" {
    pub fn dispatch_semaphore_create(value: isize) -> dispatch_semaphore_t;
    pub fn dispatch_semaphore_signal(dsema: dispatch_semaphore_t) -> isize;
    pub fn dispatch_semaphore_wait(dsema: dispatch_semaphore_t, timeout: u64) -> isize;
    pub fn dispatch_release(object: *mut std::ffi::c_void);
}

pub type CFRunLoopRef = *mut std::ffi::c_void;
pub type CFRunLoopSourceRef = *mut std::ffi::c_void;

#[repr(C)]
pub struct CFRunLoopSourceContext {
    pub version: isize,
    pub info: *mut std::ffi::c_void,
    pub retain: *const std::ffi::c_void,
    pub release: *const std::ffi::c_void,
    pub copyDescription: *const std::ffi::c_void,
    pub equal: *const std::ffi::c_void,
    pub hash: *const std::ffi::c_void,
    pub schedule: *const std::ffi::c_void,
    pub cancel: *const std::ffi::c_void,
    pub perform: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub static kCFRunLoopDefaultMode: CFStringRef;
    pub static kCFRunLoopCommonModes: CFStringRef;
    pub fn CFRunLoopGetMain() -> CFRunLoopRef;
    pub fn CFRunLoopWakeUp(rl: CFRunLoopRef);
    pub fn CFRunLoopRunInMode(
        mode: CFStringRef,
        seconds: f64,
        return_after_source_handled: bool,
    ) -> i32;
    pub fn CFRunLoopSourceCreate(
        allocator: *const std::ffi::c_void,
        order: isize,
        context: *mut CFRunLoopSourceContext,
    ) -> CFRunLoopSourceRef;
    pub fn CFRunLoopSourceSignal(source: CFRunLoopSourceRef);
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    pub fn CFRunLoopSourceInvalidate(source: CFRunLoopSourceRef);
}

// CoreGraphics cursor association
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    pub fn CGAssociateMouseAndMouseCursorPosition(connected: bool) -> i32;
}

// NSEventPhase
pub const NSEventPhaseNone: usize = 0;
pub const NSEventPhaseBegan: usize = 1 << 0;
pub const NSEventPhaseStationary: usize = 1 << 1;
pub const NSEventPhaseChanged: usize = 1 << 2;
pub const NSEventPhaseEnded: usize = 1 << 3;
pub const NSEventPhaseCancelled: usize = 1 << 4;
pub const NSEventPhaseMayBegin: usize = 1 << 5;

unsafe extern "C" {
    pub fn object_getClass(obj: id) -> Class;
    pub fn class_getMethodImplementation(cls: Class, name: SEL) -> *mut std::ffi::c_void;
    pub fn objc_autoreleasePoolPush() -> *mut std::ffi::c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut std::ffi::c_void);
}
