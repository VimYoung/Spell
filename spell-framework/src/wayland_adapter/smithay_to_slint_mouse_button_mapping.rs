use i_slint_core::items::PointerEventButton;

// Uses the official evdev mouse button codes defined in:
// https://github.com/torvalds/linux/blob/8e65320d91cdc3b241d4b94855c88459b91abf66/include/uapi/linux/input-event-codes.h#L357-L361
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
const BTN_SIDE: u32 = 0x113;
const BTN_EXTRA: u32 = 0x114;
const BTN_FORWARD: u32 = 0x115;
const BTN_BACK: u32 = 0x116;

/// Maps evdev mouse button codes to the Slint [PointerEventButton] enum.
pub fn map_mouse_button(button: u32) -> PointerEventButton {
    match button {
        BTN_LEFT => PointerEventButton::Left,
        BTN_RIGHT => PointerEventButton::Right,
        BTN_MIDDLE => PointerEventButton::Middle,
        BTN_SIDE | BTN_BACK => PointerEventButton::Back,
        BTN_EXTRA | BTN_FORWARD => PointerEventButton::Forward,
        _ => PointerEventButton::Other,
    }
}
