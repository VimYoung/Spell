use smithay_client_toolkit::{
    reexports::client::protocol::wl_pointer,
    seat::pointer::{PointerData, cursor_shape::CursorShapeManager},
};
pub struct PointerState {
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_data: Option<PointerData>,
    pub cursor_shape: CursorShapeManager,
}
