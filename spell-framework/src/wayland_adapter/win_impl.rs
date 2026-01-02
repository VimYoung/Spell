use crate::wayland_adapter::{SpellWin, way_helper::get_string};
// use owo_colors::OwoColorize;
use slint::{
    SharedString,
    platform::{/*Key,*/ PointerEventButton, WindowEvent},
};
use smithay_client_toolkit::{
    output::OutputState,
    reexports::{
        client::{
            Connection, QueueHandle,
            protocol::{wl_pointer, wl_seat},
        },
        protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability,
        SeatHandler,
        SeatState,
        keyboard::KeyboardHandler,
        pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler},
        // touch::TouchHandler,
    },
    shell::WaylandSurface,
};
use tracing::{info, trace, warn};

// This could be implemented but slint doesn't hve very specific
// APIs for touch support (I think). I am talking with them on what
// can be done so that things like multi-touch support, gestures etc
// can be made possible. For now I am going to place it on standby.
// impl TouchHandler for SpellWin {
//     fn up(
//         &mut self,
//         conn: &Connection,
//         qh: &QueueHandle<Self>,
//         touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
//         serial: u32,
//         time: u32,
//         id: i32,
//     ) {
//     }
//     fn down(
//         &mut self,
//         conn: &Connection,
//         qh: &QueueHandle<Self>,
//         touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
//         serial: u32,
//         time: u32,
//         surface: smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
//         id: i32,
//         position: (f64, f64),
//     ) {
//     }
//
//     fn motion(
//         &mut self,
//         conn: &Connection,
//         qh: &QueueHandle<Self>,
//         touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
//         time: u32,
//         id: i32,
//         position: (f64, f64),
//     ) {
//     }
//
//     fn shape(
//         &mut self,
//         conn: &Connection,
//         qh: &QueueHandle<Self>,
//         touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
//         id: i32,
//         major: f64,
//         minor: f64,
//     ) {
//     }
//     fn orientation(
//         &mut self,
//         conn: &Connection,
//         qh: &QueueHandle<Self>,
//         touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
//         id: i32,
//         orientation: f64,
//     ) {
//     }
//     fn cancel(
//         &mut self,
//         conn: &Connection,
//         qh: &QueueHandle<Self>,
//         touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
//     ) {
//     }
// }

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
        if capability == Capability::Keyboard && self.states.keyboard_state.board.is_none() {
            info!("Setting keyboard capability");
            let keyboard = self
                .states
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.states.keyboard_state.board = Some(keyboard);
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
        if capability == Capability::Keyboard && self.states.keyboard_state.board.is_some() {
            info!("Unsetting keyboard capability");
            self.states.keyboard_state.board.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.states.pointer_state.pointer.is_some() {
            info!("Unsetting pointer capability");
            self.states.pointer_state.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for SpellWin {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            // Ignore events for other surfaces
            if &event.surface != self.layer.as_ref().unwrap().wl_surface() {
                continue;
            }
            match event.kind {
                Enter { .. } => {
                    info!("Pointer entered: {:?}", event.position);

                    // TODO this code is redundent, as it doesn't set the cursor shape.
                    let pointer = &self.states.pointer_state.pointer.as_ref().unwrap();
                    let serial_no: Option<u32> = self
                        .states
                        .pointer_state
                        .pointer_data
                        .as_ref()
                        .unwrap()
                        .latest_enter_serial();
                    if let Some(no) = serial_no {
                        self.states
                            .pointer_state
                            .cursor_shape
                            .get_shape_device(pointer, qh)
                            .set_shape(no, Shape::Pointer);
                        // self.layer.commit();
                    }
                }
                Leave { .. } => {
                    info!("Pointer left: {:?}", event.position);
                    self.adapter
                        .try_dispatch_event(WindowEvent::PointerExited)
                        .unwrap_or_else(|err| {
                            warn!("Pointer exit event failed with error: {:?}", err)
                        });
                }
                Motion { .. } => {
                    // debug!("Pointer entered @{:?}", event.position);
                    self.adapter
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
                    trace!("Press {:x} @ {:?}", button, event.position);
                    self.adapter
                        .try_dispatch_event(WindowEvent::PointerPressed {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            button: PointerEventButton::Left,
                        })
                        .unwrap_or_else(|err| {
                            warn!("Pointer press event failed with error: {:?}", err)
                        });
                }
                Release { button, .. } => {
                    trace!("Release {:x} @ {:?}", button, event.position);
                    self.adapter
                        .try_dispatch_event(WindowEvent::PointerReleased {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            button: PointerEventButton::Left,
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
                    self.adapter
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
                }
            }
        }
    }
}

// TODO FIND What is the use of registery_handlers here?
impl ProvidesRegistryState for SpellWin {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.states.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
