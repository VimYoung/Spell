use crate::{layer_properties::WindowConf, wayland_adapter::SpellWin};
use smithay_client_toolkit::{
    reexports::client::protocol::{wl_keyboard, wl_pointer, wl_region::WlRegion},
    seat::{
        keyboard::KeyboardData,
        pointer::{PointerData, cursor_shape::CursorShapeManager},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, LayerSurface},
    },
};

pub(super) fn set_config(
    window_conf: &WindowConf,
    layer: &LayerSurface,
    first_configure: bool,
    input_region: Option<&WlRegion>,
    opaque_region: Option<&WlRegion>,
) {
    layer.set_size(window_conf.width, window_conf.height);
    layer.set_margin(
        window_conf.margin.0,
        window_conf.margin.1,
        window_conf.margin.2,
        window_conf.margin.3,
    );
    layer.set_keyboard_interactivity(window_conf.board_interactivity.get());
    if let Some(in_region) = input_region {
        layer.set_input_region(Some(in_region));
    }
    if let Some(op_region) = opaque_region {
        layer.set_opaque_region(Some(op_region));
    }
    set_anchor(window_conf, layer, first_configure);
}

fn set_anchor(window_conf: &WindowConf, layer: &LayerSurface, first_configure: bool) {
    match window_conf.anchor.0 {
        Some(mut first_anchor) => match window_conf.anchor.1 {
            Some(sec_anchor) => {
                first_anchor.set(sec_anchor, true);
                layer.set_anchor(first_anchor);
            }
            None => {
                layer.set_anchor(first_anchor);
                if window_conf.exclusive_zone && first_configure {
                    match first_anchor {
                        Anchor::LEFT | Anchor::RIGHT => {
                            layer.set_exclusive_zone(window_conf.width as i32)
                        }
                        Anchor::TOP | Anchor::BOTTOM => layer.set_exclusive_zone(35),
                        // Other Scenarios involve Calling the Anchor on 2 sides ( i.e. corners)
                        // in which case no exclusive_zone will be set.
                        _ => {}
                    }
                }
            }
        },
        None => {
            if let Some(sec_anchor) = window_conf.anchor.1 {
                layer.set_anchor(sec_anchor);
                if window_conf.exclusive_zone && first_configure {
                    match sec_anchor {
                        Anchor::LEFT | Anchor::RIGHT => {
                            layer.set_exclusive_zone(window_conf.width as i32)
                        }
                        Anchor::TOP | Anchor::BOTTOM => layer.set_exclusive_zone(35),
                        // Other Scenarios involve Calling the Anchor on 2 sides ( i.e. corners)
                        // in which case no exclusive_zone will be set.
                        _ => {}
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct PointerState {
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_data: Option<PointerData>,
    pub cursor_shape: CursorShapeManager,
}

#[derive(Debug)]
pub(crate) struct KeyboardState {
    pub board: Option<wl_keyboard::WlKeyboard>,
    pub board_data: Option<KeyboardData<SpellWin>>,
}
