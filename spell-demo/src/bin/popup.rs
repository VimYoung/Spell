use std::{env, error::Error};

use slint::ComponentHandle;
use spell_framework::{
    cast_spell,
    layer_properties::{
        internal::{Popup, QueueHandle, WlSurface},
        popup::{PopupAnchor, PopupConf, PopupGravity, PopupSettings},
        LayerAnchor, LayerType, WindowConf,
    },
    wayland_adapter::SpellXDGPopup,
    PopupSlint,
};
slint::include_modules!();
spell_framework::generate_widgets![AppWindow];

struct TestPopupSpell {
    frontend: TestPopup,
    backend: SpellXDGPopup,
}

impl PopupSlint for TestPopupSpell {
    fn create_new(settings: PopupSettings) -> Self
    where
        Self: Sized,
    {
        let popup = SpellXDGPopup::new(settings);
        TestPopupSpell {
            frontend: TestPopup::new().unwrap(),
            backend: popup,
        }
    }

    fn inner(&self) -> &Popup {
        self.backend.popup()
    }

    fn converter_popup<'a>(&self, wl_surface: &'a WlSurface, qh: &'a QueueHandle<SpellWin>) {
        self.backend.converter_popup(wl_surface, qh)
    }

    fn first_configure(&self) -> bool {
        self.backend.first_configure()
    }

    fn adapter(&self) -> &std::rc::Rc<spell_framework::slint_adapter::SpellSkiaWinAdapter> {
        self.backend.adapter()
    }
}

impl std::ops::Deref for TestPopupSpell {
    type Target = TestPopup;
    fn deref(&self) -> &Self::Target {
        &self.frontend
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let window_conf = WindowConf::builder()
        .width(376u32)
        .height(576u32)
        .anchor_1(LayerAnchor::TOP)
        .anchor_2(LayerAnchor::RIGHT)
        // .anchor_3(LayerAnchor::LEFT)
        .margins(50, 0, 0, 0)
        .layer_type(LayerType::Top)
        .build()
        .unwrap();

    let mut ui = AppWindowSpell::invoke_spell("counter-widget", window_conf);
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });
    if let Err(_) = ui.open_popup::<TestPopupSpell>(PopupConf {
        width: 200,
        height: 200,
        anchor: PopupAnchor::Left,
        gravity: PopupGravity::TopRight,
        anchor_rect: (10, 100, 10, 10),
    }) {
        println!("Error encountered when creating popup");
    };
    cast_spell!(ui)
}
