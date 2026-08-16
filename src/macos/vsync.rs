use crate::ffi::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// How the display-link callback wakes `wait_for_vsync`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VsyncMode {
    /// Legacy: signal on every display tick. Idle periods bank permits on the
    /// counting semaphore, so the next stretch of waits returns in a burst.
    AlwaysSignal,
    /// Current: signal only when a waiter has armed. Idle ticks are ignored.
    ArmGated,
}

struct CallbackContext {
    semaphore: dispatch_semaphore_t,
    waiting: AtomicBool,
    mode: VsyncMode,
    run_loop: CFRunLoopRef,
    source: CFRunLoopSourceRef,
}

unsafe impl Send for CallbackContext {}
unsafe impl Sync for CallbackContext {}

impl Drop for CallbackContext {
    fn drop(&mut self) {
        unsafe {
            CFRunLoopSourceInvalidate(self.source);
            CFRelease(self.source);
            dispatch_release(self.semaphore);
        }
    }
}

unsafe extern "C" fn wake_perform(_info: *mut std::ffi::c_void) {}

fn notify_vsync_waiter(context: &CallbackContext) {
    // The main thread blocks inside CFRunLoopRunInMode, not on the semaphore alone, so that
    // mach sources (accessibility requests from window managers) keep being serviced between
    // frames. Signalling the source is what lets that wait return immediately on a display tick.
    let wake = || unsafe {
        CFRunLoopSourceSignal(context.source);
        CFRunLoopWakeUp(context.run_loop);
    };

    match context.mode {
        VsyncMode::AlwaysSignal => unsafe {
            dispatch_semaphore_signal(context.semaphore);
            wake();
        },
        VsyncMode::ArmGated => {
            // Signal only an active waiter. Signalling every display tick turns
            // the semaphore into a backlog while the app is idle or rendering
            // slowly, causing later frames to run in bursts.
            //
            // Single-waiter only: concurrent wait_for_vsync callers are not supported.
            if context.waiting.swap(false, Ordering::SeqCst) {
                unsafe {
                    dispatch_semaphore_signal(context.semaphore);
                }
                wake();
            }
        }
    }
}

pub struct VsyncTracker {
    display_link: CVDisplayLinkRef,
    callback_context: Box<CallbackContext>,
}

unsafe extern "C" fn display_link_callback(
    _display_link: CVDisplayLinkRef,
    _in_now: *const std::ffi::c_void,
    _in_output_time: *const std::ffi::c_void,
    _flags_in: u64,
    _flags_out: *mut u64,
    context: *mut std::ffi::c_void,
) -> CVReturn {
    unsafe {
        notify_vsync_waiter(&*(context as *const CallbackContext));
    }
    0 // kCVReturnSuccess
}

impl VsyncTracker {
    pub fn new() -> Self {
        Self::with_mode(VsyncMode::ArmGated)
    }

    pub fn with_mode(mode: VsyncMode) -> Self {
        unsafe {
            let mut display_link = std::ptr::null_mut();
            let res = CVDisplayLinkCreateWithActiveCGDisplays(&mut display_link);
            if res != 0 || display_link.is_null() {
                panic!("Failed to create CVDisplayLink (result: {})", res);
            }

            let semaphore = dispatch_semaphore_create(0);
            if semaphore.is_null() {
                CVDisplayLinkRelease(display_link);
                panic!("Failed to create GCD semaphore");
            }

            let mut context = CFRunLoopSourceContext {
                version: 0,
                info: std::ptr::null_mut(),
                retain: std::ptr::null(),
                release: std::ptr::null(),
                copyDescription: std::ptr::null(),
                equal: std::ptr::null(),
                hash: std::ptr::null(),
                schedule: std::ptr::null(),
                cancel: std::ptr::null(),
                perform: Some(wake_perform),
            };
            let source = CFRunLoopSourceCreate(std::ptr::null(), 0, &mut context);
            let run_loop = CFRunLoopGetMain();
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);

            let mut callback_context = Box::new(CallbackContext {
                semaphore,
                waiting: AtomicBool::new(false),
                mode,
                run_loop,
                source,
            });

            let callback_res = CVDisplayLinkSetOutputCallback(
                display_link,
                display_link_callback,
                (&mut *callback_context as *mut CallbackContext).cast(),
            );
            if callback_res != 0 {
                CVDisplayLinkRelease(display_link);
                drop(callback_context);
                panic!(
                    "Failed to set CVDisplayLink output callback (result: {})",
                    callback_res
                );
            }

            let start_res = CVDisplayLinkStart(display_link);
            if start_res != 0 {
                CVDisplayLinkRelease(display_link);
                drop(callback_context);
                panic!("Failed to start CVDisplayLink (result: {})", start_res);
            }

            Self {
                display_link,
                callback_context,
            }
        }
    }

    pub fn mode(&self) -> VsyncMode {
        self.callback_context.mode
    }

    pub fn wait_for_vsync(&self) {
        unsafe {
            if self.callback_context.mode == VsyncMode::ArmGated {
                // Arm, then wait. A counting semaphore retains a signal that
                // arrives between store and wait, so there is no lost wakeup.
                // Ticks while idle leave waiting false and do not signal.
                self.callback_context.waiting.store(true, Ordering::SeqCst);
            }
            // Poll the semaphore, but idle inside the run loop so the main thread stays
            // responsive to mach messages. The timeout is only a fallback for a stalled
            // display link; the display-link callback signals our source to return early.
            while dispatch_semaphore_wait(self.callback_context.semaphore, DISPATCH_TIME_NOW) != 0 {
                CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.05, true);
            }
        }
    }
}

impl Drop for VsyncTracker {
    fn drop(&mut self) {
        unsafe {
            // Stop the link before CallbackContext (and its semaphore) is freed.
            CVDisplayLinkStop(self.display_link);
            CVDisplayLinkRelease(self.display_link);
        }
    }
}
