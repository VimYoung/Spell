use crate::{
    slint_adapter::SpellSkiaWinAdapter,
    wayland_adapter::{SpellWin, pointer_button::map_pointer_button, way_helper::get_string},
};
use slint::{SharedString, platform::WindowEvent};
use smithay_client_toolkit::{
    output::OutputState,
    reexports::{
        client::{
            Connection, Dispatch, QueueHandle,
            protocol::{wl_pointer, wl_seat},
        },
        protocols::xdg::shell::client::xdg_surface::XdgSurface,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::KeyboardHandler,
        pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler},
        touch::TouchHandler,
    },
    shell::{
        WaylandSurface,
        xdg::{popup::PopupHandler, window::WindowHandler},
    },
};
use tracing::{info, trace, warn};

impl WindowHandler for SpellWin {
    fn request_close(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &smithay_client_toolkit::shell::xdg::window::Window,
    ) {
        todo!()
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &smithay_client_toolkit::shell::xdg::window::Window,
        _: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        _: u32,
    ) {
        todo!()
    }
}

// Slint doesn't hve very specific
// APIs for touch support (I think). I am talking with them on what
// can be done so that things like multi-touch support, gestures etc
// can be made possible. For now I am going to place empty value in here.
impl TouchHandler for SpellWin {
    fn up(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        _id: i32,
    ) {
        info!("Up event from touch");
    }
    fn down(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        _surface: smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _id: i32,
        position: (f64, f64),
    ) {
        info!("Down event produced with posaition: {position:?}");
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _time: u32,
        _id: i32,
        position: (f64, f64),
    ) {
        self.adapter
            .as_ref()
            .unwrap()
            .try_dispatch_event(WindowEvent::PointerMoved {
                position: slint::LogicalPosition {
                    x: position.0 as f32,
                    y: position.1 as f32,
                },
            })
            .unwrap_or_else(|err| warn!("Touch move event failed with error: {:?}", err));
    }

    fn shape(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _id: i32,
        major: f64,
        minor: f64,
    ) {
        info!("Shape data released. Major: {major}, Minor: {minor}");
    }
    fn orientation(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _id: i32,
        orientation: f64,
    ) {
        info!("Orientation data released: {orientation}.")
    }
    fn cancel(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
    ) {
        info!("Active touch sequence cancelled");
    }
}

impl KeyboardHandler for SpellWin {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
        info!("Keyboard focus entered");
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
    ) {
        info!("Keyboard focus left");
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        trace!("Key pressed");
        let string_val: SharedString = get_string(event);
        // if string_val == <slint::platform::Key as Into<SharedString>>::into(Key::Backspace) {
        //     self.loop_handle.enable(&self.backspace).unwrap();
        //     self.adapter
        //         .try_dispatch_event(WindowEvent::KeyPressed { text: string_val })
        //         .unwrap();
        // } else {
        self.adapter
            .as_ref()
            .unwrap()
            .try_dispatch_event(WindowEvent::KeyPressed { text: string_val })
            .unwrap_or_else(|err| warn!("Key press event failed with error: {:?}", err));
        // }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        /*mut*/ event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        trace!("Key released");
        // if let Err(err) = self.loop_handle.disable(&self.backspace) {
        //     warn!("{}", err);
        // }
        // let key_sym = Keysym::new(event.raw_code);
        // event.keysym = key_sym;
        let string_val: SharedString = get_string(event);
        self.adapter
            .as_ref()
            .unwrap()
            .try_dispatch_event(WindowEvent::KeyReleased { text: string_val })
            .unwrap_or_else(|err| warn!("Key release event failed with error: {:?}", err));
    }

    // TODO needs to be implemented to enable functionalities of ctl, shift, alt etc.
    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        _raw_modifiers: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _layout: u32,
    ) {
    }
    // TODO This method needs to be implemented after the looping mecha is changed to calloop.
    fn update_repeat_info(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _info: smithay_client_toolkit::seat::keyboard::RepeatInfo,
    ) {
        trace!("Key repeat info updated");
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        trace!("Repeat key called");
    }
}

impl SeatHandler for SpellWin {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.states.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.states.keyboard_state.is_none() {
            info!("Setting keyboard capability");
            let keyboard = self
                .states
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.states.keyboard_state = Some(keyboard);
        }
        if capability == Capability::Touch && self.states.touch_state.is_none() {
            info!("Setting touch Capability");
            let touch = self
                .states
                .seat_state
                .get_touch(qh, &seat)
                .expect("Failed to create touch");
            self.states.touch_state = Some(touch);
        }
        if capability == Capability::Pointer && self.states.pointer_state.pointer.is_none() {
            info!("Setting pointer capability");
            let pointer = self
                .states
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            let pointer_data = PointerData::new(seat);
            self.states.pointer_state.pointer = Some(pointer);
            self.states.pointer_state.pointer_data = Some(pointer_data);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.states.keyboard_state.is_some() {
            info!("Unsetting keyboard capability");
            self.states.keyboard_state.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.states.pointer_state.pointer.is_some() {
            info!("Unsetting pointer capability");
            self.states.pointer_state.pointer.take().unwrap().release();
        }
        if capability == Capability::Touch && self.states.touch_state.is_some() {
            info!("Unsetting pointer capability");
            self.states.touch_state.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for SpellWin {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            let adapter: &std::rc::Rc<SpellSkiaWinAdapter> =
                if let Some(popup) = self.popup_manager.return_adapter(&event.surface) {
                    popup
                } else if &event.surface == self.layer.as_ref().unwrap().wl_surface() {
                    self.adapter.as_ref().unwrap()
                } else {
                    continue;
                };

            match event.kind {
                Enter { serial } => {
                    trace!(
                        "Pointer entered with serial {:?} at: {:?}",
                        serial, event.position
                    );

                    adapter
                        .try_dispatch_event(WindowEvent::PointerMoved {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                        })
                        .unwrap_or_else(|err| {
                            warn!(
                                "Pointer move event after entry failed with error: {:?}",
                                err
                            )
                        });
                    self.states.pointer_state.last_cursor_enter_serial = Some(serial);
                }
                Leave { .. } => {
                    trace!("Pointer left: {:?}", event.position);

                    adapter
                        .try_dispatch_event(WindowEvent::PointerExited)
                        .unwrap_or_else(|err| {
                            warn!("Pointer exit event failed with error: {:?}", err)
                        });
                }
                Motion { .. } => {
                    trace!("Pointer entered @{:?}", event.position);

                    adapter
                        .try_dispatch_event(WindowEvent::PointerMoved {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                        })
                        .unwrap_or_else(|err| {
                            warn!("Pointer move event failed with error: {:?}", err)
                        });
                }
                Press { button, .. } => {
                    trace!("Press {:?} @ {:?}", button, event.position);

                    adapter
                        .try_dispatch_event(WindowEvent::PointerPressed {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            button: map_pointer_button(button),
                        })
                        .unwrap_or_else(|err| {
                            warn!("Pointer press event failed with error: {:?}", err)
                        });
                }
                Release { button, .. } => {
                    trace!("Release {:?} @ {:?}", button, event.position);

                    adapter
                        .try_dispatch_event(WindowEvent::PointerReleased {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            button: map_pointer_button(button),
                        })
                        .unwrap_or_else(|err| {
                            warn!("Pointer release event failed with error: {:?}", err)
                        });
                }
                Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    trace!("Scroll H:{horizontal:?}, V:{vertical:?}");

                    if !self.natural_scroll {
                        adapter
                            .try_dispatch_event(WindowEvent::PointerScrolled {
                                position: slint::LogicalPosition {
                                    x: event.position.0 as f32,
                                    y: event.position.1 as f32,
                                },
                                delta_x: horizontal.absolute as f32,
                                delta_y: vertical.absolute as f32,
                            })
                            .unwrap_or_else(|err| {
                                warn!("Pointer scroll event failed with error: {:?}", err)
                            });
                    } else {
                        adapter
                            .try_dispatch_event(WindowEvent::PointerScrolled {
                                position: slint::LogicalPosition {
                                    x: event.position.0 as f32,
                                    y: event.position.1 as f32,
                                },
                                delta_x: -horizontal.absolute as f32,
                                delta_y: -vertical.absolute as f32,
                            })
                            .unwrap_or_else(|err| {
                                warn!("Pointer scroll event failed with error: {:?}", err)
                            });
                    }
                }
            }
        }
    }
}

impl PopupHandler for SpellWin {
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        popup: &smithay_client_toolkit::shell::xdg::popup::Popup,
        _: smithay_client_toolkit::shell::xdg::popup::PopupConfigure,
    ) {
        let x = self.popup_manager.return_popup(popup);
        if let Some(current_popup) = x {
            // FIXME: Is this commit required?
            current_popup.inner().wl_surface().commit();
            if current_popup.first_configure() {
                current_popup.converter_popup(current_popup.inner().wl_surface(), &self.queue);
            }
        } else {
            warn!("Popup configured but not pushed to the manager");
        }
    }

    fn done(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &smithay_client_toolkit::shell::xdg::popup::Popup,
    ) {
        todo!()
    }
}

// TODO: FIND What is the use of registery_handlers here?
impl ProvidesRegistryState for SpellWin {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.states.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

impl Dispatch<XdgSurface, ()> for SpellWin {
    fn event(
        _: &mut Self,
        _: &XdgSurface,
        _: <XdgSurface as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        todo!()
    }
}
