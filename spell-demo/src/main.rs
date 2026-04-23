use std::{env, error::Error};

use slint::ComponentHandle;
use spell_framework::{
    cast_spell,
    layer_properties::{LayerAnchor, LayerType, WindowConf},
};
slint::include_modules!();
spell_framework::generate_widgets![AppWindow];

fn main() -> Result<(), Box<dyn Error>> {
    let window_conf = WindowConf::builder()
        .width(376u32)
        .height(576u32)
        .anchor_1(LayerAnchor::TOP)
        .anchor_2(LayerAnchor::LEFT)
        .margins(50, 0, 0, 0)
        .layer_type(LayerType::Top)
        .build()
        .unwrap();

    let ui = AppWindowSpell::invoke_spell("counter-widget", window_conf);
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });
    cast_spell!(ui)
}
