//! BURROW_SHOT=<png>: the window photographs itself a few seconds after it opens and saves the picture, then the app
//! quits. For looking at the app without anyone's eyes (an app may capture its own windows without the screen
//! recording permission). Debug only.

use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[allow(non_upper_case_globals)]
const kCGWindowListOptionIncludingWindow: u32 = 1 << 3;
#[allow(non_upper_case_globals)]
const kCGWindowImageBoundsIgnoreFraming: u32 = 1 << 0;
#[allow(non_upper_case_globals)]
const kCGWindowImageBestResolution: u32 = 1 << 3;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

unsafe extern "C" {
    fn CGImageGetWidth(img: *const std::ffi::c_void) -> usize;
    fn CGImageRelease(img: *const std::ffi::c_void);
    fn CFURLCreateFromFileSystemRepresentation(a: *const std::ffi::c_void, buf: *const u8, len: isize, dir: bool) -> *const std::ffi::c_void;
    fn CGImageDestinationCreateWithURL(url: *const std::ffi::c_void, ty: *const std::ffi::c_void, n: usize, opts: *const std::ffi::c_void) -> *const std::ffi::c_void;
    fn CGImageDestinationAddImage(d: *const std::ffi::c_void, img: *const std::ffi::c_void, props: *const std::ffi::c_void);
    fn CGImageDestinationFinalize(d: *const std::ffi::c_void) -> bool;
    fn CFStringCreateWithCString(a: *const std::ffi::c_void, s: *const i8, enc: u32) -> *const std::ffi::c_void;
    fn CFRelease(o: *const std::ffi::c_void);
    fn dlsym(h: *mut std::ffi::c_void, s: *const i8) -> *mut std::ffi::c_void;
}

type CreateImage = unsafe extern "C" fn(CGRect, u32, u32, u32) -> *const std::ffi::c_void;

/// the window's number (NSWindow windowNumber)
fn window_number(window: &Window) -> Option<u32> {
    let h = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(a) = h.as_raw() else { return None };
    unsafe {
        use objc::{msg_send, sel, sel_impl};
        let view = a.ns_view.as_ptr() as *mut objc::runtime::Object;
        let win: *mut objc::runtime::Object = msg_send![view, window];
        if win.is_null() {
            return None;
        }
        let n: isize = msg_send![win, windowNumber];
        Some(n as u32)
    }
}

/// the window as it is on screen, written as a PNG; false if it could not be had
pub fn shoot(window: &Window, path: &str) -> bool {
    let Some(wid) = window_number(window) else { return false };
    unsafe {
        let f = dlsym(-2isize as *mut _, c"CGWindowListCreateImage".as_ptr()); // RTLD_DEFAULT
        if f.is_null() {
            return false;
        }
        let create: CreateImage = std::mem::transmute(f);
        let null = CGRect { x: f64::INFINITY, y: f64::INFINITY, w: 0.0, h: 0.0 }; // CGRectNull: the window's own bounds
        let img = create(null, kCGWindowListOptionIncludingWindow, wid, kCGWindowImageBoundsIgnoreFraming | kCGWindowImageBestResolution);
        if img.is_null() || CGImageGetWidth(img) == 0 {
            return false;
        }
        let url = CFURLCreateFromFileSystemRepresentation(std::ptr::null(), path.as_ptr(), path.len() as isize, false);
        let ty = CFStringCreateWithCString(std::ptr::null(), c"public.png".as_ptr(), 0x0800_0100);
        let dest = CGImageDestinationCreateWithURL(url, ty, 1, std::ptr::null());
        let ok = !dest.is_null() && {
            CGImageDestinationAddImage(dest, img, std::ptr::null());
            CGImageDestinationFinalize(dest)
        };
        for o in [dest, ty, url] {
            if !o.is_null() {
                CFRelease(o);
            }
        }
        CGImageRelease(img);
        ok
    }
}
