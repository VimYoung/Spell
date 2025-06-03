mod configure;
pub mod slint_adapter;
pub mod wayland_adapter;
pub mod layer_properties {

    pub use crate::configure::WindowConf;
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
}

use configure::Rgba8Pixel;
use smithay_client_toolkit::reexports::client::EventQueue;
use std::error::Error;
use wayland_adapter::SpellWin;

pub fn cast_spell(
    mut waywindow: SpellWin,
    mut event_queue: EventQueue<SpellWin>,
    work_buffer: Box<[Rgba8Pixel]>,
) -> Result<(), Box<dyn Error>> {
    waywindow.memory_manager.set_ren_buffers(work_buffer);
    loop {
        // Following line does the updates to the buffer. Now those updates
        // needs to be picked by the compositer/windowing system and then
        // displayed accordingly.
        // println!("Running the loop");

        if waywindow.first_configure {
            event_queue.roundtrip(&mut waywindow).unwrap();
        } else {
            // event_queue.flush().unwrap();
            // event_queue.dispatch_pending(&mut waywindow).unwrap();
            event_queue.blocking_dispatch(&mut waywindow).unwrap();
        }
    }
}

pub fn get_spell_ingredients(width: u32, height: u32) -> Box<[Rgba8Pixel]> {
    let a: u8 = 0xFF;
    vec![Rgba8Pixel::new(a, 0, 0, 0); width as usize * height as usize].into_boxed_slice()
}
