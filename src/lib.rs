#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

pub trait PlatformWindow {
    fn draw<F>(&mut self, render: F)
    where
        F: FnMut(&mut Self);
    fn open(&self) -> bool;
    fn close(&mut self);
    fn is_down(&self, key: Key) -> bool;
    fn is_up(&self, key: Key) -> bool;
    fn pressed(&self, key: Key) -> bool;
    fn released(&self, key: Key) -> bool;
    fn pressed_keys(&self) -> &[Key];
    fn mouse_down(&self, button: Mouse) -> bool;
    fn mouse_pressed(&self, button: Mouse) -> bool;
    fn mouse_released(&self, button: Mouse) -> bool;
    fn mouse_clicked(&self, button: Mouse, area: Rect) -> bool;
    fn mouse_double_clicked(&self, button: Mouse, area: Rect) -> bool;
    fn mouse_pos(&self) -> Option<(f64, f64)>;
    fn text_input(&self) -> &[char];
    fn dropped_files(&self) -> &[std::path::PathBuf];
    fn scroll_delta(&self) -> (f64, f64);
    fn scroll_events(&self) -> &[ScrollEvent];
    fn raw_mouse_delta(&self) -> (f64, f64);
    fn modifiers(&self) -> Modifiers;
    fn framebuffer_size(&self) -> (usize, usize);
    fn framebuffer(&mut self) -> &mut [u32];
    fn present(&self);
    fn scale_factor(&self) -> f64;
    fn content_size(&self) -> (usize, usize);
    fn wait_for_vsync(&self);
    /// Block the calling thread until the OS queues a new window event.
    fn wait_for_event(&self);
    fn set_cursor_visible(&self, visible: bool);
    fn set_cursor_grab(&self, grab: bool);
    fn set_cursor_icon(&self, icon: CursorIcon);
    fn get_clipboard_text(&self) -> Option<String>;
    fn set_clipboard_text(&self, text: &str);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStyle {
    Standard,
    Borderless,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullscreenMode {
    None,
    Workspace,
    MonitorFit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorIcon {
    Arrow,
    IBeam,
    PointingHand,
    ClosedHand,
    OpenHand,
    Crosshair,
    ResizeLeftRight,
    ResizeUpDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mouse {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

const MOUSE_BUTTON_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Function(u8),
    Enter,
    Space,
    Backspace,
    Escape,
    Control,
    Shift,
    Alt,
    Tab,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Up,
    Down,
    Left,
    Right,
    Logo,
    LeftWindows,
    RightWindows,
    Menu,
    CapsLock,
    ScrollLock,
    PauseBreak,
    Insert,
    Home,
    Delete,
    End,
    PageUp,
    PageDown,
    Unknown(u16),
}

impl Key {
    pub const fn vk_code(&self) -> usize {
        match *self {
            Key::Enter => 0x0D,
            Key::Space => 0x20,
            Key::Backspace => 0x08,
            Key::Escape => 0x1B,
            Key::Control => 0x11,
            Key::Shift => 0x10,
            Key::Alt => 0x12,
            Key::Tab => 0x09,
            Key::ArrowUp | Key::Up => 0x26,
            Key::ArrowDown | Key::Down => 0x28,
            Key::ArrowLeft | Key::Left => 0x25,
            Key::ArrowRight | Key::Right => 0x27,
            Key::Logo | Key::LeftWindows => 0x5B,
            Key::RightWindows => 0x5C,
            Key::Menu => 0x5D,
            Key::CapsLock => 0x14,
            Key::ScrollLock => 0x91,
            Key::PauseBreak => 0x13,
            Key::Insert => 0x2D,
            Key::Home => 0x24,
            Key::Delete => 0x2E,
            Key::End => 0x23,
            Key::PageUp => 0x21,
            Key::PageDown => 0x22,
            Key::Char(c) => {
                let upper = c.to_ascii_uppercase();
                match upper {
                    'A'..='Z' | '0'..='9' => upper as usize,
                    '=' | '+' => 0xBB,
                    '-' | '_' => 0xBD,
                    ';' | ':' => 0xBA,
                    '/' | '?' => 0xBF,
                    '`' | '~' => 0xC0,
                    '[' | '{' => 0xDB,
                    '\\' | '|' => 0xDC,
                    ']' | '}' => 0xDD,
                    '\'' | '"' => 0xDE,
                    ',' | '<' => 0xBC,
                    '.' | '>' => 0xBE,
                    _ => 0,
                }
            }
            Key::Function(n) if n >= 1 && n <= 24 => (0x6F + n) as usize,
            Key::Function(_) => 0,
            Key::Unknown(vk) => vk as usize,
        }
    }

    #[cfg(target_os = "windows")]
    pub const fn from_windows_vk(vk: u16) -> Self {
        match vk {
            0x08 => Key::Backspace,
            0x09 => Key::Tab,
            0x0D => Key::Enter,
            0x10 => Key::Shift,
            0x11 => Key::Control,
            0x12 => Key::Alt,
            0x13 => Key::PauseBreak,
            0x14 => Key::CapsLock,
            0x1B => Key::Escape,
            0x20 => Key::Space,
            0x21 => Key::PageUp,
            0x22 => Key::PageDown,
            0x23 => Key::End,
            0x24 => Key::Home,
            0x25 => Key::ArrowLeft,
            0x26 => Key::ArrowUp,
            0x27 => Key::ArrowRight,
            0x28 => Key::ArrowDown,
            0x2D => Key::Insert,
            0x2E => Key::Delete,
            0x30..=0x39 | 0x41..=0x5A => Key::Char(vk as u8 as char),
            0x5B => Key::LeftWindows,
            0x5C => Key::RightWindows,
            0x5D => Key::Menu,
            0x70..=0x87 => Key::Function((vk - 0x6F) as u8),
            0x91 => Key::ScrollLock,
            _ => Key::Unknown(vk),
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn from_macos_keycode(keycode: u16) -> Self {
        match keycode {
            0 => Key::Char('A'),
            1 => Key::Char('S'),
            2 => Key::Char('D'),
            3 => Key::Char('F'),
            4 => Key::Char('H'),
            5 => Key::Char('G'),
            6 => Key::Char('Z'),
            7 => Key::Char('X'),
            8 => Key::Char('C'),
            9 => Key::Char('V'),
            11 => Key::Char('B'),
            12 => Key::Char('Q'),
            13 => Key::Char('W'),
            14 => Key::Char('E'),
            15 => Key::Char('R'),
            16 => Key::Char('Y'),
            17 => Key::Char('T'),
            18 => Key::Char('1'),
            19 => Key::Char('2'),
            20 => Key::Char('3'),
            21 => Key::Char('4'),
            22 => Key::Char('6'),
            23 => Key::Char('5'),
            25 => Key::Char('9'),
            26 => Key::Char('7'),
            28 => Key::Char('8'),
            29 => Key::Char('0'),
            31 => Key::Char('O'),
            32 => Key::Char('U'),
            34 => Key::Char('I'),
            35 => Key::Char('P'),
            36 | 76 => Key::Enter,
            37 => Key::Char('L'),
            38 => Key::Char('J'),
            40 => Key::Char('K'),
            45 => Key::Char('N'),
            46 => Key::Char('M'),
            48 => Key::Tab,
            49 => Key::Space,
            51 => Key::Backspace,
            53 => Key::Escape,
            56 | 60 => Key::Shift,
            57 => Key::CapsLock,
            58 | 61 => Key::Alt,
            59 | 62 => Key::Control,
            55 | 54 => Key::Logo,
            63 => Key::Function(0),
            114 => Key::Insert,
            115 => Key::Home,
            116 => Key::PageUp,
            117 => Key::Delete,
            119 => Key::End,
            121 => Key::PageDown,
            122 => Key::Function(1),
            120 => Key::Function(2),
            99 => Key::Function(3),
            118 => Key::Function(4),
            96 => Key::Function(5),
            97 => Key::Function(6),
            98 => Key::Function(7),
            100 => Key::Function(8),
            101 => Key::Function(9),
            109 => Key::Function(10),
            103 => Key::Function(11),
            111 => Key::Function(12),
            123 => Key::ArrowLeft,
            124 => Key::ArrowRight,
            125 => Key::ArrowDown,
            126 => Key::ArrowUp,
            _ => Key::Unknown(keycode),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool, // Command/Windows key
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollPhase {
    #[default]
    None,
    Began,
    Changed,
    Ended,
    MomentumBegan,
    MomentumChanged,
    MomentumEnded,
}

impl ScrollPhase {
    pub fn momentum(self) -> bool {
        matches!(
            self,
            ScrollPhase::MomentumBegan | ScrollPhase::MomentumChanged | ScrollPhase::MomentumEnded
        )
    }

    pub fn ended(self) -> bool {
        matches!(self, ScrollPhase::Ended | ScrollPhase::MomentumEnded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollEvent {
    pub delta: (f64, f64),
    pub phase: ScrollPhase,
    /// Pixel-accurate input from a trackpad, as opposed to a mouse wheel's discrete notches.
    pub precise: bool,
    /// When the OS says the event happened, in seconds. 
    pub timestamp: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InputState {
    current_keys: [bool; 256],
    previous_keys: [bool; 256],
    keys_pressed_this_frame: [bool; 256],
    current_mouse: [bool; MOUSE_BUTTON_COUNT],
    previous_mouse: [bool; MOUSE_BUTTON_COUNT],
    mouse_press_positions: [Option<(f64, f64)>; MOUSE_BUTTON_COUNT],
    mouse_release_positions: [Option<(f64, f64)>; MOUSE_BUTTON_COUNT],
    mouse_press_was_double_click: [bool; MOUSE_BUTTON_COUNT],
    mouse_double_clicked_this_frame: [bool; MOUSE_BUTTON_COUNT],
    pressed_keys: Vec<Key>,
    mouse_pos: Option<(f64, f64)>,
    scroll_delta: (f64, f64),
    scroll_events: Vec<ScrollEvent>,
    raw_mouse_delta: (f64, f64),
    modifiers: Modifiers,
    text_input: Vec<char>,
    dropped_files: Vec<std::path::PathBuf>,
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self {
            current_keys: [false; 256],
            previous_keys: [false; 256],
            keys_pressed_this_frame: [false; 256],
            current_mouse: [false; MOUSE_BUTTON_COUNT],
            previous_mouse: [false; MOUSE_BUTTON_COUNT],
            mouse_press_positions: [None; MOUSE_BUTTON_COUNT],
            mouse_release_positions: [None; MOUSE_BUTTON_COUNT],
            mouse_press_was_double_click: [false; MOUSE_BUTTON_COUNT],
            mouse_double_clicked_this_frame: [false; MOUSE_BUTTON_COUNT],
            pressed_keys: Vec::new(),
            mouse_pos: None,
            scroll_delta: (0.0, 0.0),
            scroll_events: Vec::new(),
            raw_mouse_delta: (0.0, 0.0),
            modifiers: Modifiers::default(),
            text_input: Vec::new(),
            dropped_files: Vec::new(),
        }
    }

    pub(crate) fn begin_frame(&mut self) {
        self.previous_keys.copy_from_slice(&self.current_keys);
        self.keys_pressed_this_frame = [false; 256];
        self.previous_mouse.copy_from_slice(&self.current_mouse);
        self.mouse_release_positions = [None; MOUSE_BUTTON_COUNT];
        self.mouse_double_clicked_this_frame = [false; MOUSE_BUTTON_COUNT];
        self.pressed_keys.clear();
        self.text_input.clear();
        self.dropped_files.clear();
        self.scroll_delta = (0.0, 0.0);
        self.scroll_events.clear();
        self.raw_mouse_delta = (0.0, 0.0);
    }

    pub fn is_down(&self, key: Key) -> bool {
        self.key_index(key)
            .map(|index| self.current_keys[index])
            .unwrap_or(false)
    }

    pub fn is_up(&self, key: Key) -> bool {
        !self.is_down(key)
    }

    pub fn pressed(&self, key: Key) -> bool {
        self.key_index(key)
            .map(|index| self.keys_pressed_this_frame[index])
            .unwrap_or(false)
    }

    pub fn released(&self, key: Key) -> bool {
        self.key_index(key)
            .map(|index| !self.current_keys[index] && self.previous_keys[index])
            .unwrap_or(false)
    }

    pub fn pressed_keys(&self) -> &[Key] {
        &self.pressed_keys
    }

    pub fn mouse_down(&self, button: Mouse) -> bool {
        self.mouse_index(button)
            .map(|index| self.current_mouse[index])
            .unwrap_or(false)
    }

    pub fn mouse_pressed(&self, button: Mouse) -> bool {
        self.mouse_index(button)
            .map(|index| self.current_mouse[index] && !self.previous_mouse[index])
            .unwrap_or(false)
    }

    pub fn mouse_released(&self, button: Mouse) -> bool {
        self.mouse_release_position(button).is_some()
            || self
                .mouse_index(button)
                .map(|index| !self.current_mouse[index] && self.previous_mouse[index])
                .unwrap_or(false)
    }

    pub fn mouse_clicked(&self, button: Mouse, area: Rect) -> bool {
        let Some(press_pos) = self.mouse_press_position(button) else {
            return false;
        };
        let Some(release_pos) = self.mouse_release_position(button) else {
            return false;
        };

        point_in_rect(press_pos, area) && point_in_rect(release_pos, area)
    }

    pub fn mouse_double_clicked(&self, button: Mouse, area: Rect) -> bool {
        let is_double = self
            .mouse_index(button)
            .map(|index| self.mouse_double_clicked_this_frame[index])
            .unwrap_or(false);
        is_double && self.mouse_clicked(button, area)
    }

    pub fn mouse_pos(&self) -> Option<(f64, f64)> {
        self.mouse_pos
    }

    pub fn text_input(&self) -> &[char] {
        &self.text_input
    }

    pub fn dropped_files(&self) -> &[std::path::PathBuf] {
        &self.dropped_files
    }

    pub fn scroll_delta(&self) -> (f64, f64) {
        self.scroll_delta
    }

    pub fn scroll_events(&self) -> &[ScrollEvent] {
        &self.scroll_events
    }

    pub fn raw_mouse_delta(&self) -> (f64, f64) {
        self.raw_mouse_delta
    }

    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    pub(crate) fn set_key_down(&mut self, key: Key) {
        let Some(index) = self.key_index(key) else {
            return;
        };

        if !self.current_keys[index] {
            self.pressed_keys.push(key);
        }
        self.keys_pressed_this_frame[index] = true;
        self.current_keys[index] = true;
    }

    pub(crate) fn set_key_up(&mut self, key: Key) {
        if let Some(index) = self.key_index(key) {
            self.current_keys[index] = false;
        }
    }

    pub(crate) fn set_mouse_down(&mut self, button: Mouse) {
        let Some(index) = self.mouse_index(button) else {
            return;
        };

        if !self.current_mouse[index] {
            self.mouse_press_positions[index] = self.mouse_pos;
            self.mouse_press_was_double_click[index] = false;
        }
        self.current_mouse[index] = true;
    }

    pub(crate) fn set_mouse_double_down(&mut self, button: Mouse) {
        let Some(index) = self.mouse_index(button) else {
            return;
        };

        if !self.current_mouse[index] {
            self.mouse_press_positions[index] = self.mouse_pos;
        }
        self.current_mouse[index] = true;
        self.mouse_press_was_double_click[index] = true;
    }

    pub(crate) fn set_mouse_up(&mut self, button: Mouse) {
        let Some(index) = self.mouse_index(button) else {
            return;
        };

        if self.mouse_press_was_double_click[index] {
            self.mouse_double_clicked_this_frame[index] = true;
            self.mouse_press_was_double_click[index] = false;
        }
        self.current_mouse[index] = false;
        self.mouse_release_positions[index] = self.mouse_pos;
    }

    fn key_index(&self, key: Key) -> Option<usize> {
        let index = key.vk_code();
        (index < self.current_keys.len()).then_some(index)
    }

    fn mouse_index(&self, button: Mouse) -> Option<usize> {
        Some(match button {
            Mouse::Left => 0,
            Mouse::Right => 1,
            Mouse::Middle => 2,
            Mouse::Back => 3,
            Mouse::Forward => 4,
        })
    }

    fn mouse_press_position(&self, button: Mouse) -> Option<(f64, f64)> {
        self.mouse_index(button)
            .and_then(|index| self.mouse_press_positions[index])
    }

    fn mouse_release_position(&self, button: Mouse) -> Option<(f64, f64)> {
        self.mouse_index(button)
            .and_then(|index| self.mouse_release_positions[index])
    }
}

fn point_in_rect((x, y): (f64, f64), area: Rect) -> bool {
    x >= area.x as f64 && x < area.right() as f64 && y >= area.y as f64 && y < area.bottom() as f64
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl std::ops::AddAssign for Rect {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.width += rhs.width;
        self.height += rhs.height;
    }
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn from_xyxy(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self {
            x: x0,
            y: y0,
            width: x1.saturating_sub(x0),
            height: y1.saturating_sub(y0),
        }
    }

    pub const fn x(mut self, x: i32) -> Self {
        self.x = x;
        self
    }
    pub const fn y(mut self, y: i32) -> Self {
        self.y = y;
        self
    }
    pub const fn width(mut self, width: i32) -> Self {
        self.width = width;
        self
    }
    pub const fn height(mut self, height: i32) -> Self {
        self.height = height;
        self
    }
    pub const fn right(&self) -> i32 {
        self.x.saturating_add(self.width)
    }
    pub const fn bottom(&self) -> i32 {
        self.y.saturating_add(self.height)
    }
    pub const fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }
    pub fn scale(self, scale: f32) -> Rect {
        let x = (self.x as f32 * scale).round() as i32;
        let y = (self.y as f32 * scale).round() as i32;
        let right = (self.right() as f32 * scale).round() as i32;
        let bottom = (self.bottom() as f32 * scale).round() as i32;
        Rect::from_xyxy(x, y, right, bottom)
    }
    pub const fn intersects(&self, other: Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
    pub const fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
    pub fn intersection(self, other: Rect) -> Rect {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self.right().min(other.right());
        let y1 = self.bottom().min(other.bottom());
        if x1 > x0 && y1 > y0 {
            Rect::from_xyxy(x0, y0, x1, y1)
        } else {
            Rect::default()
        }
    }
    pub fn clamp(self, bounds: Rect) -> Rect {
        self.intersection(bounds)
    }
    pub fn clamp_to_size(self, width: i32, height: i32) -> Rect {
        self.clamp(Rect::new(0, 0, width, height))
    }
    pub const fn inner(&self, w: i32, h: i32) -> Rect {
        Rect {
            x: self.x + w,
            y: self.y + h,
            width: self.width.saturating_sub(2 * w),
            height: self.height.saturating_sub(2 * h),
        }
    }
}
