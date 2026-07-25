// This module contains mapping from slint to wayland and vice versa for pointer,
// cursor and strings.
use i_slint_core::items::{MouseCursor, PointerEventButton};
use slint::{SharedString, platform::Key};
use smithay_client_toolkit::{
    reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    seat::keyboard::{KeyEvent, Keysym},
};

// Uses the official evdev pointer button codes defined in:
// https://github.com/torvalds/linux/blob/8e65320d91cdc3b241d4b94855c88459b91abf66/include/uapi/linux/input-event-codes.h#L357-L361
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
const BTN_SIDE: u32 = 0x113;
const BTN_EXTRA: u32 = 0x114;
const BTN_FORWARD: u32 = 0x115;
const BTN_BACK: u32 = 0x116;

/// Maps evdev pointer button codes to the Slint [PointerEventButton] enum.
pub(super) fn map_pointer_button(button: u32) -> PointerEventButton {
    match button {
        BTN_LEFT => PointerEventButton::Left,
        BTN_RIGHT => PointerEventButton::Right,
        BTN_MIDDLE => PointerEventButton::Middle,
        BTN_SIDE | BTN_BACK => PointerEventButton::Back,
        BTN_EXTRA | BTN_FORWARD => PointerEventButton::Forward,
        _ => PointerEventButton::Other,
    }
}

/// Maps the slint cursor enum to the wayland cursor shape enum
///
/// [MouseCursor::None] is handled internally by the program because there
/// is no wayland cursor shape for it
pub(super) fn mouse_cursor_to_shape(cursor: MouseCursor) -> Shape {
    match cursor {
        MouseCursor::Default => Shape::Default,
        MouseCursor::Help => Shape::Help,
        MouseCursor::Pointer => Shape::Pointer,
        MouseCursor::Progress => Shape::Progress,
        MouseCursor::Wait => Shape::Wait,
        MouseCursor::Crosshair => Shape::Crosshair,
        MouseCursor::Text => Shape::Text,
        MouseCursor::Alias => Shape::Alias,
        MouseCursor::Copy => Shape::Copy,
        MouseCursor::Move => Shape::Move,
        MouseCursor::NoDrop => Shape::NoDrop,
        MouseCursor::NotAllowed => Shape::NotAllowed,
        MouseCursor::Grab => Shape::Grab,
        MouseCursor::Grabbing => Shape::Grabbing,
        MouseCursor::ColResize => Shape::ColResize,
        MouseCursor::RowResize => Shape::RowResize,
        MouseCursor::NResize => Shape::NResize,
        MouseCursor::EResize => Shape::EResize,
        MouseCursor::SResize => Shape::SResize,
        MouseCursor::WResize => Shape::WResize,
        MouseCursor::NeResize => Shape::NeResize,
        MouseCursor::NwResize => Shape::NwResize,
        MouseCursor::SeResize => Shape::SeResize,
        MouseCursor::SwResize => Shape::SwResize,
        MouseCursor::EwResize => Shape::EwResize,
        MouseCursor::NsResize => Shape::NsResize,
        MouseCursor::NeswResize => Shape::NeswResize,
        MouseCursor::NwseResize => Shape::NwseResize,
        _ => Shape::Default,
    }
}

/// Maps wayland specific keys into slint key events and then parsing the
/// information as a SharedString.
/// In case the matching is not present sharedstring is created from utf8
/// representation of event.
pub(super) fn get_string(event: KeyEvent) -> SharedString {
    let mut key: Option<Key> = None;
    match event.keysym {
        Keysym::BackSpace => key = Some(Key::Backspace),
        Keysym::Tab => key = Some(Key::Tab),
        Keysym::Return => key = Some(Key::Return),
        Keysym::Escape => key = Some(Key::Escape),
        Keysym::BackTab => key = Some(Key::Backtab),
        Keysym::Delete => key = Some(Key::Delete),
        Keysym::Shift_L => key = Some(Key::Shift),
        Keysym::Shift_R => key = Some(Key::ShiftR),
        Keysym::Control_L => key = Some(Key::Control),
        Keysym::Control_R => key = Some(Key::ControlR),
        Keysym::Alt_L => key = Some(Key::Alt),
        Keysym::Alt_R => key = Some(Key::AltGr),
        Keysym::Caps_Lock => key = Some(Key::CapsLock),
        Keysym::Meta_L => key = Some(Key::Meta),
        Keysym::Meta_R => key = Some(Key::MetaR),
        Keysym::space => key = Some(Key::Space),
        Keysym::Up | Keysym::uparrow => key = Some(Key::UpArrow),
        Keysym::Down | Keysym::downarrow => key = Some(Key::DownArrow),
        Keysym::Left | Keysym::leftarrow => key = Some(Key::LeftArrow),
        Keysym::Right | Keysym::rightarrow => key = Some(Key::RightArrow),
        Keysym::F1 => key = Some(Key::F1),
        Keysym::F2 => key = Some(Key::F2),
        Keysym::F3 => key = Some(Key::F3),
        Keysym::F4 => key = Some(Key::F4),
        Keysym::F5 => key = Some(Key::F5),
        Keysym::F6 => key = Some(Key::F6),
        Keysym::F7 => key = Some(Key::F7),
        Keysym::F8 => key = Some(Key::F8),
        Keysym::F9 => key = Some(Key::F9),
        Keysym::F10 => key = Some(Key::F10),
        Keysym::F11 => key = Some(Key::F11),
        Keysym::F12 => key = Some(Key::F12),
        Keysym::F13 => key = Some(Key::F13),
        Keysym::F14 => key = Some(Key::F14),
        Keysym::F15 => key = Some(Key::F15),
        Keysym::F16 => key = Some(Key::F16),
        Keysym::F17 => key = Some(Key::F17),
        Keysym::F18 => key = Some(Key::F18),
        Keysym::F19 => key = Some(Key::F19),
        Keysym::F20 => key = Some(Key::F20),
        Keysym::F21 => key = Some(Key::F21),
        Keysym::F22 => key = Some(Key::F22),
        Keysym::F23 => key = Some(Key::F23),
        Keysym::F24 => key = Some(Key::F24),
        Keysym::Insert => key = Some(Key::Insert),
        Keysym::Home => key = Some(Key::Home),
        Keysym::End => key = Some(Key::End),
        Keysym::Page_Up => key = Some(Key::PageUp),
        Keysym::Page_Down => key = Some(Key::PageDown),
        Keysym::Scroll_Lock => key = Some(Key::ScrollLock),
        Keysym::Pause => key = Some(Key::Pause),
        Keysym::Sys_Req => key = Some(Key::SysReq),
        Keysym::XF86_Stop => key = Some(Key::Stop),
        Keysym::Menu => key = Some(Key::Menu),
        _ => {}
    }

    if let Some(key) = key {
        key.into()
    } else {
        SharedString::from(event.utf8.unwrap_or_default())
    }
}
