use crate::{configure::HomeHandle, layer_properties::WindowConf, wayland_adapter::SpellWin};
use nonstick::{ConversationAdapter, Result as PamResult};
use slint::{SharedString, platform::Key};
use smithay_client_toolkit::{
    reexports::{
        calloop::{
            EventLoop,
            timer::{TimeoutAction, Timer},
        },
        client::protocol::{wl_keyboard, wl_pointer, wl_region::WlRegion},
    },
    seat::{
        keyboard::{KeyEvent, Keysym},
        pointer::{PointerData, cursor_shape::CursorShapeManager},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{/*Anchor,*/ LayerSurface},
    },
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
    layer.set_size(window_conf.width, window_conf.height);
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
    set_anchor(window_conf, layer);
}

fn set_anchor(window_conf: &WindowConf, layer: &LayerSurface) {
    match window_conf.anchor.0 {
        Some(mut first_anchor) => match window_conf.anchor.1 {
            Some(sec_anchor) => {
                first_anchor.set(sec_anchor, true);
                layer.set_anchor(first_anchor);
            }
            None => {
                layer.set_anchor(first_anchor);
                // if window_conf.exclusive_zone && first_configure {
                //     match first_anchor {
                //         Anchor::LEFT | Anchor::RIGHT => {
                //             layer.set_exclusive_zone(window_conf.width as i32)
                //         }
                //         Anchor::TOP | Anchor::BOTTOM => {
                //             layer.set_exclusive_zone(window_conf.height as i32)
                //         }
                //         // Other Scenarios involve Calling the Anchor on 2 sides ( i.e. corners)
                //         // in which case no exclusive_zone will be set.
                //         _ => {}
                //     }
                // }
            }
        },
        None => {
            if let Some(sec_anchor) = window_conf.anchor.1 {
                layer.set_anchor(sec_anchor);
                // if window_conf.exclusive_zone && first_configure {
                //     match sec_anchor {
                //         Anchor::LEFT | Anchor::RIGHT => {
                //             layer.set_exclusive_zone(window_conf.width as i32)
                //         }
                //         Anchor::TOP | Anchor::BOTTOM => {
                //             layer.set_exclusive_zone(window_conf.height as i32)
                //         }
                //         // Other Scenarios involve Calling the Anchor on 2 sides ( i.e. corners)
                //         // in which case no exclusive_zone will be set.
                //         _ => {}
                //     }
                // }
            }
        }
    }
    if let Some(val) = window_conf.exclusive_zone {
        layer.set_exclusive_zone(val);
    }
}

pub(super) fn get_string(event: KeyEvent) -> SharedString {
    let mut key: Option<Key> = None;
    match event.keysym {
        Keysym::BackSpace => key = Some(Key::Backspace),
        Keysym::Tab => key = Some(Key::Tab),
        Keysym::Return => key = Some(Key::Return),
        Keysym::Escape => key = Some(Key::Escape),
        Keysym::BackTab => key = Some(Key::Backtab),
        Keysym::Delete => key = Some(Key::Delete),
        Keysym::Shift_L => key = Some(Key::Shift),
        Keysym::Shift_R => key = Some(Key::ShiftR),
        Keysym::Control_L => key = Some(Key::Control),
        Keysym::Control_R => key = Some(Key::ControlR),
        Keysym::Alt_L => key = Some(Key::Alt),
        Keysym::Alt_R => key = Some(Key::AltGr),
        Keysym::Caps_Lock => key = Some(Key::CapsLock),
        Keysym::Meta_L => key = Some(Key::Meta),
        Keysym::Meta_R => key = Some(Key::MetaR),
        Keysym::space => key = Some(Key::Space),
        Keysym::Up | Keysym::uparrow => key = Some(Key::UpArrow),
        Keysym::Down | Keysym::downarrow => key = Some(Key::DownArrow),
        Keysym::Left | Keysym::leftarrow => key = Some(Key::LeftArrow),
        Keysym::Right | Keysym::rightarrow => key = Some(Key::RightArrow),
        Keysym::F1 => key = Some(Key::F1),
        Keysym::F2 => key = Some(Key::F2),
        Keysym::F3 => key = Some(Key::F3),
        Keysym::F4 => key = Some(Key::F4),
        Keysym::F5 => key = Some(Key::F5),
        Keysym::F6 => key = Some(Key::F6),
        Keysym::F7 => key = Some(Key::F7),
        Keysym::F8 => key = Some(Key::F8),
        Keysym::F9 => key = Some(Key::F9),
        Keysym::F10 => key = Some(Key::F10),
        Keysym::F11 => key = Some(Key::F11),
        Keysym::F12 => key = Some(Key::F12),
        Keysym::F13 => key = Some(Key::F13),
        Keysym::F14 => key = Some(Key::F14),
        Keysym::F15 => key = Some(Key::F15),
        Keysym::F16 => key = Some(Key::F16),
        Keysym::F17 => key = Some(Key::F17),
        Keysym::F18 => key = Some(Key::F18),
        Keysym::F19 => key = Some(Key::F19),
        Keysym::F20 => key = Some(Key::F20),
        Keysym::F21 => key = Some(Key::F21),
        Keysym::F22 => key = Some(Key::F22),
        Keysym::F23 => key = Some(Key::F23),
        Keysym::F24 => key = Some(Key::F24),
        Keysym::Insert => key = Some(Key::Insert),
        Keysym::Home => key = Some(Key::Home),
        Keysym::End => key = Some(Key::End),
        Keysym::Page_Up => key = Some(Key::PageUp),
        Keysym::Page_Down => key = Some(Key::PageDown),
        Keysym::Scroll_Lock => key = Some(Key::ScrollLock),
        Keysym::Pause => key = Some(Key::Pause),
        Keysym::Sys_Req => key = Some(Key::SysReq),
        Keysym::XF86_Stop => key = Some(Key::Stop),
        Keysym::Menu => key = Some(Key::Menu),
        _ => {}
    }

    if let Some(key) = key {
        key.into()
    } else {
        SharedString::from(event.utf8.unwrap_or_default())
    }
}

#[derive(Debug)]
pub(crate) struct PointerState {
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_data: Option<PointerData>,
    pub cursor_shape: CursorShapeManager,
}

#[derive(Debug)]
pub(crate) struct KeyboardState {
    pub board: Option<wl_keyboard::WlKeyboard>,
}

pub(crate) fn set_event_sources(event_loop: &EventLoop<'static, SpellWin>, handle: HomeHandle) {
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

    // let _ = fs::create_dir(Path::new(&logging_dir));
    // let _ = fs::remove_file(&socket_cli_dir);

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
        .insert_source(
            Timer::from_duration(Duration::from_millis(1000)),
            |_, _, data| {
                let slint_event_proxy = data.adapter.slint_event_proxy.clone();
                if let Ok(mut list_of_events) = slint_event_proxy.try_lock()
                    && !(*list_of_events).is_empty()
                {
                    let original_len = (*list_of_events).len();
                    let mut x = 0;
                    while x < original_len {
                        let event = (*list_of_events).pop().unwrap();
                        event();
                        x += 1;
                    }
                }
                TimeoutAction::ToDuration(Duration::from_millis(1000))
            },
        )
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
