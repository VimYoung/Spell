use std::{env, error::Error};

use spell_framework::{
    cast_spell,
    layer_properties::{BoardType, LayerAnchor, LayerType, WindowConf},
};
slint::include_modules!();
spell_framework::generate_widgets![CopyPaste];

fn main() -> Result<(), Box<dyn Error>> {
    let window_conf = WindowConf::builder()
        .width(376_u32)
        .height(576_u32)
        .anchor_1(LayerAnchor::TOP)
        .anchor_2(LayerAnchor::LEFT)
        .margins(5, 0, 0, 10)
        .layer_type(LayerType::Top)
        .board_interactivity(BoardType::OnDemand)
        .build()
        .unwrap();

    let ui = CopyPasteSpell::invoke_spell("counter-widget", window_conf);
    // ui.on_request_increase_value({
    //     let ui_handle = ui.as_weak();
    //     move || {
    //         let ui = ui_handle.unwrap();
    //         ui.set_counter(ui.get_counter() + 1);
    //     }
    // });
    cast_spell!(ui)
}
