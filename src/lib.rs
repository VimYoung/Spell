pub mod slint_adapter;
pub mod wayland_adapter;
pub mod layer_properties {

    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
}

// use slint_adapter::{SlintLayerShell, SpellWinAdapter};
// use wayland_adapter::SpellWin;
