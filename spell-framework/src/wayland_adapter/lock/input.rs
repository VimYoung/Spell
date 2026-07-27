use crate::wayland_adapter::{
    SpellLock,
    common::{self, get_string},
};
use slint::{SharedString, platform::WindowEvent};
use smithay_client_toolkit::{
    reexports::{
        client::{Connection, QueueHandle, protocol::wl_pointer},
        protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    },
    seat::{
        keyboard::KeyboardHandler,
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        touch::TouchHandler,
    },
};
use tracing::{info, trace, warn};

impl TouchHandler for SpellLock {
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
        self.slint_part.as_ref().unwrap().adapters[0]
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

impl PointerHandler for SpellLock {
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
            for surface in self.lock_surfaces.iter() {
                if event.surface != *surface.wl_surface() {
                    continue;
                }
            }
            match event.kind {
                Enter { .. } => {
                    info!("Pointer entered: {:?}", event.position);

                    // TODO this code is redundent, as it doesn't set the cursor shape.
                    let pointer = &self.pointer_state.pointer.as_ref().unwrap();
                    let serial_no: Option<u32> = self
                        .pointer_state
                        .pointer_data
                        .as_ref()
                        .unwrap()
                        .latest_enter_serial();
                    if let Some(no) = serial_no {
                        self.pointer_state
                            .cursor_shape
                            .get_shape_device(pointer, qh)
                            .set_shape(no, Shape::Pointer);
                    }
                }
                Leave { .. } => {
                    info!("Pointer left: {:?}", event.position);
                    self.slint_part.as_ref().unwrap().adapters[0]
                        .try_dispatch_event(WindowEvent::PointerExited)
                        .unwrap_or_else(|err| {
                            warn!("Pointer left event failed with error: {:?}", err)
                        });
                }
                Motion { .. } => {
                    // debug!("Pointer entered @{:?}", event.position);
                    self.slint_part.as_ref().unwrap().adapters[0]
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

                    self.slint_part.as_ref().unwrap().adapters[0]
                        .try_dispatch_event(WindowEvent::PointerPressed {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            button: common::map_pointer_button(button),
                        })
                        .unwrap_or_else(|err| {
                            warn!("Pointer press event failed with error: {:?}", err)
                        });
                }
                Release { button, .. } => {
                    trace!("Release {:x} @ {:?}", button, event.position);

                    self.slint_part.as_ref().unwrap().adapters[0]
                        .try_dispatch_event(WindowEvent::PointerReleased {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            button: common::map_pointer_button(button),
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
                    self.slint_part.as_ref().unwrap().adapters[0]
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

impl KeyboardHandler for SpellLock {
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
        let string_val: SharedString = get_string(event);
        // if string_val == <slint::platform::Key as Into<SharedString>>::into(Key::Backspace) {
        //     self.loop_handle.enable(&self.backspace.unwrap()).unwrap();
        //     self.slint_part.as_ref().unwrap().adapters[0]
        //         .try_dispatch_event(WindowEvent::KeyPressed { text: string_val })
        //         .unwrap();
        // } else {
        info!("Key pressed with value : {:?}", string_val);
        self.slint_part.as_ref().unwrap().adapters[0]
            .try_dispatch_event(WindowEvent::KeyPressed { text: string_val })
            .unwrap_or_else(|err| warn!("Key press event failed with error: {:?}", err));
        //}
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        info!("Key is released");
        // if let Err(err) = self.loop_handle.disable(&self.backspace.unwrap()) {
        //     warn!("{}", err);
        // }
        // let key_sym = Keysym::new(event.raw_code);
        // event.keysym = key_sym;
        let string_val: SharedString = get_string(event);
        self.slint_part.as_ref().unwrap().adapters[0]
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
        trace!("Updated modifiers");
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        trace!("Repeated key entered");
    }
    // TODO This method needs to be implemented after the looping mecha is changed to calloop.
    fn update_repeat_info(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _info: smithay_client_toolkit::seat::keyboard::RepeatInfo,
    ) {
        trace!("Repeat info updation called");
    }
}
