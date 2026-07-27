use slint::platform::{Key, WindowAdapter};
use smithay_client_toolkit::reexports::{
    calloop::{
        self,
        channel::{self, Channel},
        timer::{TimeoutAction, Timer},
    },
    client::QueueHandle,
};
use tracing::warn;

use crate::wayland_adapter::SpellLock;

impl SpellLock {
    pub(super) fn converter_lock(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.slint_part.as_ref().unwrap().size[0].width;
        let height: u32 = self.slint_part.as_ref().unwrap().size[0].height;
        let window_adapter = self.slint_part.as_ref().unwrap().adapters[0].clone();
        let _redraw_val: bool = window_adapter.draw_if_needed();

        let buffer = &self.slint_part.as_ref().unwrap().wayland_buffer[0];
        self.lock_surfaces[0]
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        self.lock_surfaces[0]
            .wl_surface()
            .frame(qh, self.lock_surfaces[0].wl_surface().clone());
        self.lock_surfaces[0]
            .wl_surface()
            .attach(Some(buffer.wl_buffer()), 0, 0);

        self.lock_surfaces[0].wl_surface().commit();
    }

    pub(super) fn set_event_sources(
        &mut self,
        slint_event_receiver: Channel<Box<dyn FnOnce() + Send>>,
        rx: Channel<bool>,
    ) {
        let loop_handle = self.loop_handle.clone();
        loop_handle
            .insert_source(slint_event_receiver, |event, _, data| {
                if let calloop::channel::Event::Msg(callback) = event {
                    callback();

                    if let Some(slint_part) = &data.slint_part {
                        for adapter in &slint_part.adapters {
                            adapter.request_redraw();
                        }
                    }
                }
            })
            .unwrap();

        self.backspace = Some(
            loop_handle
                .insert_source(
                    Timer::from_duration(std::time::Duration::from_millis(1500)),
                    |_, _, data| {
                        data.slint_part.as_ref().unwrap().adapters[0]
                            .try_dispatch_event(slint::platform::WindowEvent::KeyPressed {
                                text: Key::Backspace.into(),
                            })
                            .unwrap();
                        TimeoutAction::ToDuration(std::time::Duration::from_millis(1500))
                    },
                )
                .unwrap(),
        );

        let _ = loop_handle
            .clone()
            .insert_source(rx, move |event, _, data| match event {
                channel::Event::Msg(msg) => {
                    if msg {
                        if let Some(locked_val) = data.session_lock.take() {
                            locked_val.unlock();
                        } else {
                            warn!("Authentication verified but couldn't unlock");
                        }
                        data.is_locked = false;
                        data.conn.roundtrip().unwrap();
                    }
                }
                channel::Event::Closed => {
                    warn!("Unlock channel to open thread is closed.");
                }
            });

        loop_handle.disable(&self.backspace.unwrap()).unwrap();
    }
}
