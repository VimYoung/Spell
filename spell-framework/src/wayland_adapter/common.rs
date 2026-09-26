use i_slint_core::{cursor::MouseCursorInner, items::BuiltInMouseCursor};
// This module contains mapping to strings and pointerdata objects common to both
// lock and window.
use slint::{
    SharedString,
    platform::{Key, PointerEventButton},
};
use smithay_client_toolkit::{
    reexports::{
        client::{QueueHandle, protocol::wl_pointer},
        protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    },
    seat::{
        keyboard::{KeyEvent, Keysym},
        pointer::{PointerData, cursor_shape::CursorShapeManager},
    },
};

use crate::wayland_adapter::SpellWin;

#[derive(Debug)]
pub(crate) struct PointerState {
    pub(crate) pointer: Option<wl_pointer::WlPointer>,
    pub(crate) pointer_data: Option<PointerData<()>>,
    pub(crate) cursor_shape: CursorShapeManager,
    pub(crate) current_wayland_cursor: MouseCursorInner,
    pub(crate) last_cursor_enter_serial: Option<u32>,
}

impl PointerState {
    /// Updates the cursor shape
    ///
    /// If the cursor is [BuiltInMouseCursor::None], the cursor will be hidden
    ///
    /// If the cursor is not [BuiltInMouseCursor::None], the cursor will be set to the shape corresponding to the cursor
    ///
    /// The cursor is only updated when it doesn't match the current cursor, currently
    /// there is no support for setting images as cursors via new slint API.
    pub(crate) fn update_cursor(
        &mut self,
        mouse_cursor: &MouseCursorInner,
        queue: &QueueHandle<SpellWin>,
    ) {
        if let Some(serial) = self.last_cursor_enter_serial
            && let Some(pointer) = self.pointer.as_ref()
            && *mouse_cursor != self.current_wayland_cursor
        {
            match mouse_cursor {
                MouseCursorInner::BuiltIn(builtin_type) => {
                    if *builtin_type == BuiltInMouseCursor::None {
                        pointer.set_cursor(serial, None, 0, 0);
                    } else {
                        self.cursor_shape
                            .get_shape_device(pointer, queue)
                            .set_shape(serial, mouse_cursor_to_shape(builtin_type.clone()));
                    }
                    self.current_wayland_cursor = MouseCursorInner::BuiltIn(builtin_type.clone());
                }
                // FIXME: This needs to be handled properly
                MouseCursorInner::CustomMouseCursor { .. } | _ => {}
            }
        }
    }
}

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
pub(super) fn mouse_cursor_to_shape(cursor: BuiltInMouseCursor) -> Shape {
    match cursor {
        BuiltInMouseCursor::Default => Shape::Default,
        BuiltInMouseCursor::Help => Shape::Help,
        BuiltInMouseCursor::Pointer => Shape::Pointer,
        BuiltInMouseCursor::Progress => Shape::Progress,
        BuiltInMouseCursor::Wait => Shape::Wait,
        BuiltInMouseCursor::Crosshair => Shape::Crosshair,
        BuiltInMouseCursor::Text => Shape::Text,
        BuiltInMouseCursor::Alias => Shape::Alias,
        BuiltInMouseCursor::Copy => Shape::Copy,
        BuiltInMouseCursor::Move => Shape::Move,
        BuiltInMouseCursor::NoDrop => Shape::NoDrop,
        BuiltInMouseCursor::NotAllowed => Shape::NotAllowed,
        BuiltInMouseCursor::Grab => Shape::Grab,
        BuiltInMouseCursor::Grabbing => Shape::Grabbing,
        BuiltInMouseCursor::ColResize => Shape::ColResize,
        BuiltInMouseCursor::RowResize => Shape::RowResize,
        BuiltInMouseCursor::NResize => Shape::NResize,
        BuiltInMouseCursor::EResize => Shape::EResize,
        BuiltInMouseCursor::SResize => Shape::SResize,
        BuiltInMouseCursor::WResize => Shape::WResize,
        BuiltInMouseCursor::NeResize => Shape::NeResize,
        BuiltInMouseCursor::NwResize => Shape::NwResize,
        BuiltInMouseCursor::SeResize => Shape::SeResize,
        BuiltInMouseCursor::SwResize => Shape::SwResize,
        BuiltInMouseCursor::EwResize => Shape::EwResize,
        BuiltInMouseCursor::NsResize => Shape::NsResize,
        BuiltInMouseCursor::NeswResize => Shape::NeswResize,
        BuiltInMouseCursor::NwseResize => Shape::NwseResize,
        _ => Shape::Default,
    }
}

// Uses the /// Maps wayland specific keys into slint key events and then parsing the
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
