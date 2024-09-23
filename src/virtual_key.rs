use std::fmt::Display;
use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, KBDLLHOOKSTRUCT, MOUSEHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_SYSKEYDOWN,
};

#[derive(strum_macros::AsRefStr, PartialEq, Eq, Hash, Clone, Copy)]
pub enum KeyCode {
    Unknown(u32),
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Backquote,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Num0,
    Minus,
    Equal,
    Backspace,
    Tab,
    KeyQ,
    KeyW,
    KeyE,
    KeyR,
    KeyT,
    KeyY,
    KeyU,
    KeyI,
    KeyO,
    KeyP,
    LeftBracket,
    RightBracket,
    Backslash,
    Capslock,
    KeyA,
    KeyS,
    KeyD,
    KeyF,
    KeyG,
    KeyH,
    KeyJ,
    KeyK,
    KeyL,
    Semicolon,
    Quote,
    Enter,
    ShiftLeft,
    KeyZ,
    KeyX,
    KeyC,
    KeyV,
    KeyB,
    KeyN,
    KeyM,
    Comma,
    Dot,
    Slash,
    ShiftRight,
    ControlLeft,
    /// ALSO KNOWN AS "WINDOWS", "SUPER", AND "COMMAND"
    MetaLeft,
    AltLeft,
    Space,
    AltRight,
    /// ALSO KNOWN AS "WINDOWS", "SUPER", AND "COMMAND"
    MetaRight,
    ControlRight,
    Printscreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    Delete,
    End,
    PageDown,
    UpArrow,
    DownArrow,
    LeftArrow,
    RightArrow,
    MouseLeft,
    MouseRight,
    MouseMiddle,
}

macro_rules! create_converter {
    ($from_func_name:ident, $to_func_name:ident, $($key:ident, $id:literal),+) => {
        pub fn $from_func_name(id: u32) -> KeyCode {
            match id {
                $($id => KeyCode::$key,)+
                _ => KeyCode::Unknown(id)
            }
        }

        pub fn $to_func_name(&self) -> u32 {
            match self {
                $(KeyCode::$key => $id,)+
                KeyCode::Unknown(v) => *v,
            }
        }

    };
}

impl KeyCode {
    /// ref https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
    create_converter! {from_windows_id, to_windows_id,
        Escape, 0x1B,
        F1, 0x70,
        F2, 0x71,
        F3, 0x72,
        F4, 0x73,
        F5, 0x74,
        F6, 0x75,
        F7, 0x76,
        F8, 0x77,
        F9, 0x78,
        F10, 0x79,
        F11, 0x7A,
        F12, 0x7B,
        Backquote, 0xC0,
        Num1, 0x31,
        Num2, 0x32,
        Num3, 0x33,
        Num4, 0x34,
        Num5, 0x35,
        Num6, 0x36,
        Num7, 0x37,
        Num8, 0x38,
        Num9, 0x39,
        Num0, 0x30,
        Minus, 0xBD,
        Equal, 0xBB,
        Backspace, 0x08,
        Tab, 0x09,
        KeyQ, 0x51,
        KeyW, 0x57,
        KeyE, 0x45,
        KeyR, 0x52,
        KeyT, 0x54,
        KeyY, 0x59,
        KeyU, 0x55,
        KeyI, 0x49,
        KeyO, 0x4F,
        KeyP, 0x50,
        LeftBracket, 0xDB,
        RightBracket, 0xDD,
        Backslash, 0xDC,
        Capslock, 0x14,
        KeyA, 0x41,
        KeyS, 0x53,
        KeyD, 0x44,
        KeyF, 0x46,
        KeyG, 0x47,
        KeyH, 0x48,
        KeyJ, 0x4A,
        KeyK, 0x4B,
        KeyL, 0x4C,
        Semicolon, 	0xBA,
        Quote, 0xDE,
        Enter, 0x0D,
        ShiftLeft, 0xA0,
        KeyZ, 0x5A,
        KeyX, 0x58,
        KeyC, 0x43,
        KeyV, 0x56,
        KeyB, 0x42,
        KeyN, 0x4E,
        KeyM, 0x4D,
        Comma, 0xBC,
        Dot, 0xBE,
        Slash, 0xBF,
        ShiftRight, 0xA1,
        ControlLeft, 0xA2,
        MetaLeft, 0x5B,
        AltLeft, 0xA4,
        Space, 0x20,
        AltRight, 0xA5,
        MetaRight, 0x5C,
        ControlRight, 0xA3,
        Printscreen, 0x2C,
        ScrollLock, 0x91,
        Pause, 0x13,
        Insert, 0x2D,
        Home, 0x24,
        PageUp, 0x21,
        Delete, 0x2E,
        End, 0x23,
        PageDown, 0x22,
        UpArrow, 0x26,
        DownArrow, 0x28,
        LeftArrow, 0x25,
        RightArrow, 0x27,
        MouseLeft, 0x01,
        MouseRight, 0x02,
        MouseMiddle, 0x04
    }

    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl Display for KeyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} windows_id:{}", self.to_str(), self.to_windows_id())
    }
}

#[derive(strum_macros::AsRefStr, PartialEq, Clone, Copy)]
pub enum KeyOpt {
    Unknown,
    Up,
    Down,
    Move,
    DoubleClick,
}

pub struct InputKey {
    pub key: KeyCode,
    pub opt: KeyOpt,
}

impl InputKey {
    pub fn from(wparam: WPARAM, l_param: LPARAM) -> Option<Self> {
        match wparam.0 as u32 {
            WM_KEYUP | WM_KEYDOWN | WM_SYSKEYDOWN => Some(Self::from_keyboard(wparam, l_param)),
            WM_LBUTTONDBLCLK | WM_LBUTTONUP | WM_LBUTTONDOWN | WM_RBUTTONDBLCLK | WM_RBUTTONUP
            | WM_RBUTTONDOWN => Some(Self::from_mouse(wparam, l_param)),
            _ => None,
        }
    }

    fn from_keyboard(wparam: WPARAM, l_param: LPARAM) -> Self {
        let kb_struct: &KBDLLHOOKSTRUCT = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
        let mut key_event = Self::default();
        key_event.key = KeyCode::from_windows_id(kb_struct.vkCode);
        let id = wparam.0 as u32;
        if id == WM_KEYDOWN || id == WM_SYSKEYDOWN {
            key_event.opt = KeyOpt::Down;
        } else {
            key_event.opt = KeyOpt::Up;
        }
        key_event
    }

    fn from_mouse(wparam: WPARAM, l_param: LPARAM) -> Self {
        // let mouse_struct: &MOUSEHOOKSTRUCT = unsafe { &*(l_param.0 as *const MOUSEHOOKSTRUCT) };
        let mut mouse_event = Self::default();
        match wparam.0 as u32 {
            WM_LBUTTONDBLCLK => {
                mouse_event.opt = KeyOpt::DoubleClick;
                mouse_event.key = KeyCode::MouseLeft;
            }
            WM_LBUTTONUP => {
                mouse_event.opt = KeyOpt::Up;
                mouse_event.key = KeyCode::MouseLeft;
            }
            WM_LBUTTONDOWN => {
                mouse_event.opt = KeyOpt::Down;
                mouse_event.key = KeyCode::MouseLeft;
            }
            WM_RBUTTONDBLCLK => {
                mouse_event.opt = KeyOpt::DoubleClick;
                mouse_event.key = KeyCode::MouseRight;
            }
            WM_RBUTTONUP => {
                mouse_event.opt = KeyOpt::Up;
                mouse_event.key = KeyCode::MouseRight;
            }
            WM_RBUTTONDOWN => {
                mouse_event.opt = KeyOpt::Down;
                mouse_event.key = KeyCode::MouseRight;
            }
            WM_MOUSEMOVE => {
                mouse_event.opt = KeyOpt::Move;
            }
            _ => (),
        }
        mouse_event
    }
}

impl Default for InputKey {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

impl Display for InputKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InputKey:{{opt:{}, key:{}}}",
            self.opt.as_ref(),
            self.key,
        )
    }
}

#[derive(PartialEq, Clone, Copy)]
pub struct CursorPos {
    pub x: i32,
    pub y: i32,
}

impl CursorPos {
    pub fn get_cursor_pos() -> Self {
        let mut point = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut point);
        }
        CursorPos::from(point)
    }
}

impl From<POINT> for CursorPos {
    fn from(value: POINT) -> Self {
        CursorPos {
            x: value.x,
            y: value.y,
        }
    }
}

impl Display for CursorPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ x:{},y:{} }}", self.x, self.y)
    }
}
