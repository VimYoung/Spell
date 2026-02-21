use std::{env, error::Error};

use slint::ComponentHandle;
use spell_framework::{
    cast_spell,
    layer_properties::{BoardType, LayerAnchor, LayerType, WindowConf},
};
slint::include_modules!();
spell_framework::generate_widgets![AppWindow];

fn main() -> Result<(), Box<dyn Error>> {
    let window_conf = WindowConf::new(
        376,
        576,
        (Some(LayerAnchor::TOP), Some(LayerAnchor::LEFT)),
        (5, 0, 0, 10),
        LayerType::Top,
        BoardType::None,
        None,
    );

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
