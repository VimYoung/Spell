use crate::{layer_properties::WindowConf, wayland_adapter::SpellWin};
use slint::{
    SharedString,
    platform::{PointerEventButton, WindowEvent},
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
        Capability, SeatHandler, SeatState,
        keyboard::{KeyboardData, KeyboardHandler},
        pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, LayerSurface},
    },
};

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
        println!("Keyboard focus entered");
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
    ) {
        println!("Keyboard focus left");
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        self.adapter
            .try_dispatch_event(WindowEvent::KeyPressed {
                text: SharedString::from(event.utf8.unwrap()),
            })
            .unwrap();
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        let value = event.keysym.key_char();
        if let Some(val) = value {
            self.adapter
                .try_dispatch_event(WindowEvent::KeyReleased {
                    text: SharedString::from(val),
                })
                .unwrap();
        }
    }

    // TODO needs to be implemented to enable functionalities of ctl, shift, alt etc.
    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        _layout: u32,
    ) {
    }
}

impl SeatHandler for SpellWin {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard_state.board.is_none() {
            println!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            // let keyboard_data = KeyboardData::new(seat);
            self.keyboard_state.board = Some(keyboard);
            // TODO keyboard Data needs to be set later.
            // self.keyboard_state.board_data = Some(keyboard_data::<Self>);
        }

        if capability == Capability::Pointer && self.pointer_state.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            let pointer_data = PointerData::new(seat);
            self.pointer_state.pointer = Some(pointer);
            self.pointer_state.pointer_data = Some(pointer_data);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard_state.board.is_some() {
            println!("Unset keyboard capability");
            self.keyboard_state.board.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer_state.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer_state.pointer.take().unwrap().release();
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
            if &event.surface != self.layer.wl_surface() {
                continue;
            }
            match event.kind {
                Enter { .. } => {
                    // println!("Pointer entered @{:?}", event.position);

                    // TODO this code is redundent, as it doesn't set the cursor shape.
                    let pointer = &self.pointer_state.pointer.as_ref().unwrap();
                    let serial_no: Option<u32> = self
                        .pointer_state
                        .pointer_data
                        .as_ref()
                        .unwrap()
                        .latest_enter_serial();
                    if let Some(no) = serial_no {
                        println!("Cursor Shape set");
                        self.pointer_state
                            .cursor_shape
                            .get_shape_device(pointer, qh)
                            .set_shape(no, Shape::Pointer);
                        // self.layer.commit();
                    }
                }
                Leave { .. } => {
                    println!("Pointer left");
                    self.adapter
                        .try_dispatch_event(WindowEvent::PointerExited)
                        .unwrap();
                }
                Motion { .. } => {
                    // println!("Pointer entered @{:?}", event.position);
                    self.adapter
                        .try_dispatch_event(WindowEvent::PointerMoved {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                        })
                        .unwrap();
                }
                Press { button, .. } => {
                    println!("Press {:x} @ {:?}", button, event.position);
                    self.adapter
                        .try_dispatch_event(WindowEvent::PointerPressed {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            button: PointerEventButton::Left,
                        })
                        .unwrap();
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);
                    self.adapter
                        .try_dispatch_event(WindowEvent::PointerReleased {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            button: PointerEventButton::Left,
                        })
                        .unwrap();
                }
                Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
                    self.adapter
                        .try_dispatch_event(WindowEvent::PointerScrolled {
                            position: slint::LogicalPosition {
                                x: event.position.0 as f32,
                                y: event.position.1 as f32,
                            },
                            delta_x: horizontal.absolute as f32,
                            delta_y: vertical.absolute as f32,
                        })
                        .unwrap();
                }
            }
        }
    }
}

// TODO FIND What is the use of registery_handlers here?
impl ProvidesRegistryState for SpellWin {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

pub(super) fn set_anchor(
    window_conf: &WindowConf,
    layer: &mut LayerSurface,
    width: i32,
    height: i32,
    first_configure: bool,
) {
    match window_conf.anchor.0 {
        Some(mut first_anchor) => match window_conf.anchor.1 {
            Some(sec_anchor) => {
                first_anchor.set(sec_anchor, true);
                layer.set_anchor(first_anchor);
            }
            None => {
                layer.set_anchor(first_anchor);
                if window_conf.exclusive_zone && first_configure {
                    match first_anchor {
                        Anchor::LEFT | Anchor::RIGHT => layer.set_exclusive_zone(width),
                        Anchor::TOP | Anchor::BOTTOM => layer.set_exclusive_zone(35),
                        // Other Scenarios involve Calling the Anchor on 2 sides ( i.e. corners)
                        // in which case no exclusive_zone will be set.
                        _ => {}
                    }
                }
            }
        },
        None => {
            if let Some(sec_anchor) = window_conf.anchor.1 {
                layer.set_anchor(sec_anchor);
                if window_conf.exclusive_zone && first_configure {
                    match sec_anchor {
                        Anchor::LEFT | Anchor::RIGHT => layer.set_exclusive_zone(width),
                        Anchor::TOP | Anchor::BOTTOM => layer.set_exclusive_zone(35),
                        // Other Scenarios involve Calling the Anchor on 2 sides ( i.e. corners)
                        // in which case no exclusive_zone will be set.
                        _ => {}
                    }
                }
            }
        }
    }
}
