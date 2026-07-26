use crate::wayland_adapter::window::{self, SpellWin};
use smithay_client_toolkit::{reexports::client::QueueHandle, shell::WaylandSurface};

impl SpellWin {
    pub(super) fn set_config_internal(&self) {
        window::set_config(
            &self.config,
            self.layer.as_ref().unwrap(),
            Some(self.input_region.wl_region()),
            Some(self.opaque_region.wl_region()),
        );
    }

    pub(super) fn converter(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.adapter.as_ref().unwrap().size.get().width;
        let height: u32 = self.adapter.as_ref().unwrap().size.get().height;
        let window_adapter = self.adapter.clone();

        // Rendering from Skia
        if !self.is_hidden.get() {
            // let skia_now = std::time::Instant::now();
            let redraw_val: bool = window_adapter.unwrap().draw_if_needed();
            // let elasped_time = skia_now.elapsed().as_millis();
            // if elasped_time != 0 {
            //     debug!("Skia Elapsed Time: {}", skia_now.elapsed().as_millis());
            // }

            self.states
                .pointer_state
                .update_cursor(self.adapter.as_ref().unwrap().current_cursor.get(), qh);

            let buffer = &self.buffer;
            if self.first_configure.get() || redraw_val {
                // if self.first_configure {
                self.first_configure.set(false);
                self.layer.as_ref().unwrap().wl_surface().damage_buffer(
                    0,
                    0,
                    width as i32,
                    height as i32,
                );
                // } else {
                //     for (position, size) in self.damaged_part.as_ref().unwrap().iter() {
                //         // println!(
                //         //     "{}, {}, {}, {}",
                //         //     position.x, position.y, size.width as i32, size.height as i32,
                //         // );
                //         // if size.width != width && size.height != height {
                //         self.layer.wl_surface().damage_buffer(
                //             position.x,
                //             position.y,
                //             size.width as i32,
                //             size.height as i32,
                //         );
                //         //}
                //     }
                // }
                // Request our next frame
                self.layer.as_ref().unwrap().wl_surface().attach(
                    Some(buffer.as_ref().unwrap().wl_buffer()),
                    0,
                    0,
                );
            }

            self.layer
                .as_ref()
                .unwrap()
                .wl_surface()
                .frame(qh, self.layer.as_ref().unwrap().wl_surface().clone());
            self.layer.as_ref().unwrap().commit();
        } else {
            self.layer.as_ref().unwrap().commit();
        }
    }
}
