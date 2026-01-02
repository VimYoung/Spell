use i_slint_core::items::MouseCursor;
use smithay_client_toolkit::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;

pub fn mouse_cursor_to_shape(cursor: MouseCursor) -> Shape {
    match cursor {
        MouseCursor::Default => Shape::Default,
        MouseCursor::None => Shape::Default, // TODO: there is a way to hide the cursor via wayland
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
