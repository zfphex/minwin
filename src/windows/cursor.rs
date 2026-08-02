//TODO: Maybe load a cursor file instead of programatically creating them...
use crate::*;
use std::sync::OnceLock;

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateIconIndirect(info: *const ICONINFO) -> *mut c_void;
}

#[link(name = "Gdi32")]
unsafe extern "system" {
    fn CreateBitmap(
        width: i32,
        height: i32,
        planes: u32,
        bit_count: u32,
        bits: *const c_void,
    ) -> *mut c_void;
    fn CreateDIBSection(
        hdc: *mut c_void,
        info: *const BITMAPINFO,
        usage: u32,
        bits: *mut *mut c_void,
        section: *mut c_void,
        offset: u32,
    ) -> *mut c_void;
    fn DeleteObject(object: *mut c_void) -> i32;
}

#[repr(C)]
struct ICONINFO {
    icon: BOOL,
    hotspot_x: DWORD,
    hotspot_y: DWORD,
    mask: *mut c_void,
    color: *mut c_void,
}

/// Windows has no autoscroll cursor to load, so the three states are drawn here.
const SIZE: i32 = 32;
const CENTRE: f32 = SIZE as f32 / 2.0;
const OUTLINE: f32 = 1.25;
const DISC_RADIUS: f32 = 10.0;
const DOT_RADIUS: f32 = 1.5;
const MARK_BASE: f32 = 3.2;
const MARK_TIP: f32 = 6.8;
const MARK_HALF_WIDTH: f32 = 2.8;
const MARK_ROUND: f32 = 0.4;
const ARROW_BASE: f32 = -2.0;
const ARROW_TIP: f32 = 1.0;
const ARROW_HALF_WIDTH: f32 = 2.0;
const ARROW_ROUND: f32 = 1.2;
const ANCHOR_OFFSET: f32 = 4.5;
const ANCHOR_RADIUS: f32 = 2.0;

pub fn cursor_handle(icon: CursorIcon) -> *mut c_void {
    static CURSORS: OnceLock<[usize; 3]> = OnceLock::new();

    let index = match icon {
        CursorIcon::AutoScroll => 0,
        CursorIcon::AutoScrollUp => 1,
        CursorIcon::AutoScrollDown => 2,
        _ => {
            let idc = match icon {
                CursorIcon::IBeam => IDC_IBEAM,
                CursorIcon::PointingHand => IDC_HAND,
                CursorIcon::Crosshair => IDC_CROSS,
                CursorIcon::ResizeLeftRight => IDC_SIZEWE,
                CursorIcon::ResizeUpDown => IDC_SIZENS,
                _ => IDC_ARROW,
            };
            return unsafe { LoadCursorW(null_mut(), idc) };
        }
    };

    let cursors =
        CURSORS.get_or_init(|| [0.0, -1.0, 1.0].map(|direction| create(direction) as usize));
    cursors[index] as *mut c_void
}

fn create(direction: f32) -> *mut c_void {
    let mut pixels = [0u32; (SIZE * SIZE) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let point = (x as f32 + 0.5 - CENTRE, y as f32 + 0.5 - CENTRE);
            let (fill, marks) = if direction == 0.0 {
                let marks = mark(point, -1.0)
                    .min(mark(point, 1.0))
                    .min(point.0.hypot(point.1) - DOT_RADIUS);
                (point.0.hypot(point.1) - DISC_RADIUS, marks)
            } else {
                let anchor = point.1 + direction * ANCHOR_OFFSET;
                (
                    arrow(point, direction).min(point.0.hypot(anchor) - ANCHOR_RADIUS),
                    f32::MAX,
                )
            };

            let black = (coverage(fill) - coverage(marks)).max(0.0);
            let alpha = coverage(fill - OUTLINE);
            let shade = if alpha > 0.0 {
                ((alpha - black) / alpha * 255.0) as u32
            } else {
                0
            };
            pixels[(y * SIZE + x) as usize] =
                ((alpha * 255.0) as u32) << 24 | shade << 16 | shade << 8 | shade;
        }
    }

    unsafe {
        let dc = GetDC(0);
        let info = BITMAPINFO::new(SIZE, SIZE);
        let mut bits = null_mut();
        let color = CreateDIBSection(dc, &info, 0, &mut bits, null_mut(), 0);
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u32, pixels.len());

        // The alpha channel does all the masking, so the mask itself is left fully opaque.
        let empty = [0u8; (SIZE * SIZE / 8) as usize];
        let mask = CreateBitmap(SIZE, SIZE, 1, 1, empty.as_ptr() as *const c_void);
        let cursor = CreateIconIndirect(&ICONINFO {
            icon: 0,
            hotspot_x: CENTRE as u32,
            hotspot_y: CENTRE as u32,
            mask,
            color,
        });

        DeleteObject(color);
        DeleteObject(mask);
        cursor
    }
}

fn coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

/// `direction` is 1 pointing down the screen and -1 pointing up it.
fn triangle(
    point: (f32, f32),
    direction: f32,
    base: f32,
    tip: f32,
    half_width: f32,
    round: f32,
) -> f32 {
    let (across, along) = (point.0.abs(), point.1 * direction);
    let height = tip - base;
    let length = height.hypot(half_width);
    let side = ((across - half_width) * height + (along - base) * half_width) / length;
    side.max(base - along) - round
}

fn mark(point: (f32, f32), direction: f32) -> f32 {
    triangle(
        point,
        direction,
        MARK_BASE,
        MARK_TIP,
        MARK_HALF_WIDTH,
        MARK_ROUND,
    )
}

fn arrow(point: (f32, f32), direction: f32) -> f32 {
    triangle(
        point,
        direction,
        ARROW_BASE,
        ARROW_TIP,
        ARROW_HALF_WIDTH,
        ARROW_ROUND,
    )
}
