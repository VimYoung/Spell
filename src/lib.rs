mod configure;
pub mod slint_adapter;
pub mod wayland_adapter;
pub mod layer_properties {

    pub use crate::configure::WindowConf;
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
}

use configure::Rgba8Pixel;
use slint_adapter::SpellWinAdapter;
use smithay_client_toolkit::reexports::client::EventQueue;
use std::{error::Error, rc::Rc};
use wayland_adapter::SpellWin;

pub fn cast_spell<'a>(
    mut waywindow: SpellWin,
    window_adapter: Rc<SpellWinAdapter>,
    mut event_queue: EventQueue<SpellWin>,
    mut work_buffer: &'a mut [Rgba8Pixel],
    mut currently_displayed_buffer: &'a mut [Rgba8Pixel],
    width: u32,
) -> Result<(), Box<dyn Error>> {
    loop {
        // Following line does the updates to the buffer. Now those updates
        // needs to be picked by the compositer/windowing system and then
        // displayed accordingly.
        // println!("Running the loop");
        let now = std::time::Instant::now();

        if waywindow.render_again.replace(false) {
            slint::platform::update_timers_and_animations();
            window_adapter.draw_if_needed(|renderer| {
                // println!("Rendering");
                let physical_region = renderer.render(work_buffer, width as usize);
                waywindow.set_damaged(physical_region);
                waywindow.set_buffer(work_buffer.to_vec());
            });

            core::mem::swap::<&mut [Rgba8Pixel]>(&mut work_buffer, &mut currently_displayed_buffer);
            let time_gone = now.elapsed().as_millis();
            println!("Render time: {}", time_gone);
            waywindow.render_again.set(false);
        }
        if waywindow.first_configure {
            event_queue.roundtrip(&mut waywindow).unwrap();
        } else {
            event_queue.flush().unwrap();
            event_queue.dispatch_pending(&mut waywindow).unwrap();
            event_queue.blocking_dispatch(&mut waywindow).unwrap();
        }
    }
}

pub fn get_spell_ingredients(width: u32, height: u32) -> (Vec<Rgba8Pixel>, Vec<Rgba8Pixel>) {
    let a: u8 = 0xFF;
    (
        vec![Rgba8Pixel::new(a, 0, 0, 0); width as usize * height as usize],
        vec![Rgba8Pixel::new(a, 0, 0, 0); width as usize * height as usize],
    )
}
