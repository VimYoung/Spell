use crate::{
    configure::HomeHandle,
    layer_properties::WindowConf,
    wayland_adapter::{SpellWin, mapping},
};
use i_slint_core::items::MouseCursor;
use nonstick::{ConversationAdapter, Result as PamResult};
use slint::platform::WindowAdapter;
use smithay_client_toolkit::{
    reexports::{
        calloop::{
            self, EventLoop,
            timer::{TimeoutAction, Timer},
        },
        client::{
            QueueHandle,
            protocol::{wl_pointer, wl_region::WlRegion},
        },
    },
    seat::pointer::{PointerData, cursor_shape::CursorShapeManager},
    shell::{WaylandSurface, wlr_layer::LayerSurface},
};
use std::{
    fs,
    io::{BufReader, prelude::*},
    time::Duration,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

pub(super) fn set_config(
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

#[derive(Debug)]
pub(crate) struct PointerState {
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_data: Option<PointerData>,
    pub cursor_shape: CursorShapeManager,
    pub current_wayland_cursor: MouseCursor,
    pub last_cursor_enter_serial: Option<u32>,
}

impl PointerState {
    /// Updates the cursor shape
    ///
    /// If the cursor is [MouseCursor::None], the cursor will be hidden
    ///
    /// If the cursor is not [MouseCursor::None], the cursor will be set to the shape corresponding to the cursor
    ///
    /// The cursor is only updated when it doesn't match the current cursor
    pub fn update_cursor(&mut self, mouse_cursor: MouseCursor, queue: &QueueHandle<SpellWin>) {
        if self.last_cursor_enter_serial.is_some()
            && self.pointer.is_some()
            && mouse_cursor != self.current_wayland_cursor
        {
            let pointer = self.pointer.as_ref().unwrap();
            let serial = self.last_cursor_enter_serial.unwrap();

            if mouse_cursor == MouseCursor::None {
                pointer.set_cursor(serial, None, 0, 0);
            } else {
                self.cursor_shape
                    .get_shape_device(pointer, queue)
                    .set_shape(serial, mapping::mouse_cursor_to_shape(mouse_cursor));
            }
            self.current_wayland_cursor = mouse_cursor;
        }
    }
}

pub(crate) fn set_event_sources(
    event_loop: &EventLoop<'static, SpellWin>,
    handle: HomeHandle,
    slint_event_receiver: calloop::channel::Channel<Box<dyn FnOnce() + Send>>,
) {
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
                                    *layer.filter_mut() =
                                        EnvFilter::new("spell_framework::slint_adapter=info,warn");
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

// TODO have to add no auth allowed after 3 consecutive wrong attempts feature.

/// A basic Conversation that assumes that any "regular" prompt is for
/// the username, and that any "masked" prompt is for the password.
///
/// A typical Conversation will provide the user with an interface
/// to interact with PAM, e.g. a dialogue box or a terminal prompt.
pub(crate) struct UsernamePassConvo {
    pub(crate) username: String,
    pub(crate) password: String,
}

// ConversationAdapter is a convenience wrapper for the common case
// of only handling one request at a time.
impl ConversationAdapter for UsernamePassConvo {
    fn prompt(&self, request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        info!("Request: {:?}", request.as_ref());
        Ok(std::ffi::OsString::from(&self.username))
    }

    fn masked_prompt(&self, request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        info!("Masked Request: {:?}", request.as_ref());
        Ok(std::ffi::OsString::from(&self.password))
    }

    fn error_msg(&self, message: impl AsRef<std::ffi::OsStr>) {
        warn!("Ignored Error Message: {:?}", message.as_ref());
    }

    fn info_msg(&self, message: impl AsRef<std::ffi::OsStr>) {
        warn!("Ignored Info Message: {:?}", message.as_ref());
    }
}

pub(crate) struct FingerprintInfo;

impl ConversationAdapter for FingerprintInfo {
    fn prompt(&self, request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        warn!("Ignored Prompt: {:?}", request.as_ref());
        Ok(std::ffi::OsString::from(""))
    }

    fn masked_prompt(&self, request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        warn!("Ignored masked prompt: {:?}", request.as_ref());
        Ok(std::ffi::OsString::from(""))
    }

    fn info_msg(&self, message: impl AsRef<std::ffi::OsStr>) {
        info!("Info Message: {:?}", message.as_ref());
    }

    fn error_msg(&self, message: impl AsRef<std::ffi::OsStr>) {
        info!("Error Message: {:?}", message.as_ref());
    }
}
