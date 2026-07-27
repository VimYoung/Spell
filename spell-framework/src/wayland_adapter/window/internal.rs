use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    time::Duration,
};

use crate::{
    configure::{HomeHandle, WindowConf},
    wayland_adapter::window::SpellWin,
};
use slint::platform::WindowAdapter;
use smithay_client_toolkit::{
    reexports::{
        calloop::{
            self,
            timer::{TimeoutAction, Timer},
        },
        client::{
            EventQueue, QueueHandle,
            protocol::{wl_output, wl_region::WlRegion},
        },
    },
    shell::{WaylandSurface, wlr_layer::LayerSurface},
};
use tracing::warn;
use tracing_subscriber::EnvFilter;

impl SpellWin {
    pub(super) fn set_config_internal(&self) {
        set_config(
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

    /// Fetches the available monitors from the Wayland registry.
    ///
    /// This function fetches the available monitors from the Wayland registry
    /// and returns a map of the available monitors where the key is the name
    /// of the monitor and the value is [`wl_output::WlOutput`] with its assosiated
    /// logical size(width, height). Dimentions are later used in size determination.
    /// It uses an already registered event queue & spell window.
    ///
    /// # Errors
    ///
    /// Returns `None` if the registry queue could not be initialized.
    pub(super) fn get_available_monitors(
        event_queue: &mut EventQueue<SpellWin>,
        win: &mut SpellWin,
    ) -> Option<HashMap<String, (wl_output::WlOutput, i32, i32)>> {
        // roundtrip to get all available monitors from Wayland
        event_queue.roundtrip(win).ok()?;

        Some(
            win.states
                .output_state
                .outputs()
                .filter_map(|output| {
                    let info = win.states.output_state.info(&output)?;
                    Some((
                        info.name?,
                        (output, info.logical_size?.0, info.logical_size?.1),
                    ))
                })
                .collect(),
        )
    }

    pub(super) fn set_event_sources(
        &self,
        handle: HomeHandle,
        slint_event_receiver: calloop::channel::Channel<Box<dyn FnOnce() + Send>>,
    ) {
        let event_loop = self.event_loop.as_ref().borrow();
        // let backspace_event = event_loop
        //     .handle()
        //     .insert_source(
        //         Timer::from_duration(Duration::from_millis(1500)),
        //         |_, _, data| {
        //             data.adapter
        //                 .try_dispatch_event(slint::platform::WindowEvent::KeyPressed {
        //                     text: Key::Backspace.into(),
        //                 })
        //                 .unwrap();
        //             TimeoutAction::ToDuration(Duration::from_millis(1500))
        //         },
        //     )
        //     .unwrap();
        // event_loop.handle().disable(&backspace_event).unwrap();

        // // Inserting tracing source
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("runtime dir is not set");
        let logging_dir = runtime_dir + "/spell/";
        let socket_cli_dir = logging_dir.clone() + "/spell_cli";

        // This is currently redundent source as it is not working in any way
        event_loop
            .handle()
            .insert_source(
                Timer::from_duration(Duration::from_secs(2)),
                move |_, _, _| {
                    let file = fs::File::open(&socket_cli_dir)
                        .unwrap_or_else(|_| fs::File::create_new(&socket_cli_dir).unwrap());
                    let buf = BufReader::new(file);
                    let file_contents: Vec<String> = buf
                        .lines()
                        .map(|l| l.expect("Could not parse line"))
                        .collect();
                    if !file_contents.is_empty() {
                        match file_contents[0].as_str() {
                            "slint_log" => {
                                handle
                                    .modify(|layer| {
                                        *layer.filter_mut() = EnvFilter::new(
                                            "spell_framework::slint_adapter=info,warn",
                                        );
                                    })
                                    .unwrap_or_else(|error| {
                                        warn!("Error when setting slint_log: {}", error);
                                    });
                            }
                            "debug" => {
                                handle
                                    .modify(|layer| {
                                        *layer.filter_mut() =
                                            EnvFilter::new("spell_framework=info,warn"); //*layer;
                                    })
                                    .unwrap_or_else(|error| {
                                        warn!("Error when setting slint_log: {}", error);
                                    });
                            }
                            "dump" => {
                                handle
                                    .modify(|layer| {
                                        *layer.filter_mut() =
                                            EnvFilter::new("spell_framework=trace,info"); //*layer;
                                    })
                                    .unwrap_or_else(|error| {
                                        warn!("Error when setting slint_log: {}", error);
                                    });
                            }
                            "dev" => {
                                handle
                                    .modify(|layer| {
                                        *layer.filter_mut() =
                                            EnvFilter::new("spell_framework=trace,warn"); //*layer;
                                    })
                                    .unwrap_or_else(|error| {
                                        warn!("Error when setting slint_log: {}", error);
                                    });
                            }
                            val => {
                                warn!("Something else came: {}", val);
                            }
                        }
                    }
                    TimeoutAction::ToDuration(Duration::from_secs(2))
                },
            )
            .unwrap();

        event_loop
            .handle()
            .insert_source(slint_event_receiver, |event, _, data| {
                if let calloop::channel::Event::Msg(callback) = event {
                    callback();
                    data.adapter.as_ref().unwrap().request_redraw();
                }
            })
            .unwrap();
    }
}

fn set_config(
    window_conf: &WindowConf,
    layer: &LayerSurface,
    input_region: Option<&WlRegion>,
    opaque_region: Option<&WlRegion>,
) {
    layer.set_size(window_conf.evaluated_width, window_conf.evaluated_height);
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
    layer.set_layer(window_conf.layer_type);
    set_anchor(window_conf, layer);
}

fn set_anchor(window_conf: &WindowConf, layer: &LayerSurface) {
    let mut anchors = window_conf.anchor.into_iter().flatten();
    if let Some(mut combined) = anchors.next() {
        for a in anchors {
            combined.insert(a);
        }
        layer.set_anchor(combined);
    }
    if let Some(val) = window_conf.exclusive_zone {
        layer.set_exclusive_zone(val);
    }
}
