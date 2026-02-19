use std::{env, error::Error};

use slint::{ComponentHandle, SharedString};
use spell_framework::{
    cast_spell,
    layer_properties::{LayerAnchor, WindowConf},
};
slint::include_modules!();
spell_framework::generate_widgets![TopBar];
use chrono::Local;

fn main() -> Result<(), Box<dyn Error>> {
    let window_conf = WindowConf::builder()
        .width(1536_u32)
        .height(40_u32)
        .anchor_1(LayerAnchor::TOP)
        .build()
        .unwrap();

    let ui = TopBarSpell::invoke_spell("counter-widget", window_conf);
    ui.on_set_time({
        let ui_handle = ui.as_weak();
        move || {
            let now = Local::now();
            let time = now.format("%I:%M %p").to_string();
            ui_handle.unwrap().set_time_var(SharedString::from(time));
        }
    });
    cast_spell!(ui)
}
