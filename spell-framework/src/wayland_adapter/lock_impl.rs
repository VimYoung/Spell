use slint::{
    PhysicalSize, SharedString,
    platform::{PointerEventButton, /*Key,*/ WindowEvent},
};
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            Connection, QueueHandle,
            protocol::{wl_output, wl_pointer, wl_seat, wl_surface},
        },
        protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyboardHandler /*Keysym*/},
        pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler},
    },
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockSurface, SessionLockSurfaceConfigure,
    },
    shm::{Shm, ShmHandler, slot::Buffer},
};
use tracing::{info /*warn*/, trace};

use crate::{
    slint_adapter::SpellSkiaWinAdapter,
    wayland_adapter::{SpellLock, way_helper::get_string},
};

impl ProvidesRegistryState for SpellLock {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

impl ShmHandler for SpellLock {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl OutputHandler for SpellLock {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        info!("New output source added");
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        info!("Updated output source");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        info!("Output is destroyed");
    }
}

impl CompositorHandler for SpellLock {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        info!("Scale factor changed");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        info!("Compositor transformation changed");
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.converter_lock(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        info!("Surface entered");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        info!("Surface left");
    }
}

impl SessionLockHandler for SpellLock {
    fn locked(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
        info!("Session is locked");
    }

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        info!("Session could not be locked");
        self.is_locked = true;
    }
    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: SessionLockSurface,
        _configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        self.converter_lock(qh);
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
        info!("Key pressed with value : {:?}", string_val);
        // if string_val == <slint::platform::Key as Into<SharedString>>::into(Key::Backspace) {
        //     self.loop_handle.enable(&self.backspace.unwrap()).unwrap();
        //     self.slint_part.as_ref().unwrap().adapters[0]
        //         .try_dispatch_event(WindowEvent::KeyPressed { text: string_val })
        //         .unwrap();
        // } else {
        self.slint_part.as_ref().unwrap().adapters[0]
            .try_dispatch_event(WindowEvent::KeyPressed { text: string_val })
            .unwrap();
        //}
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        /*mut*/ event: smithay_client_toolkit::seat::keyboard::KeyEvent,
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
            .unwrap();
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

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        info!("Repeated key entered");
    }
    // TODO This method needs to be implemented after the looping mecha is changed to calloop.
    fn update_repeat_info(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _info: smithay_client_toolkit::seat::keyboard::RepeatInfo,
    ) {
    }
}

impl SeatHandler for SpellLock {
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
            info!("Setting keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.keyboard_state.board = Some(keyboard);
        }
        if capability == Capability::Pointer && self.pointer_state.pointer.is_none() {
            info!("Setting pointer capability");
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
            info!("Unsettting keyboard capability");
            self.keyboard_state.board.take().unwrap().release();
        }
        if capability == Capability::Pointer && self.pointer_state.pointer.is_some() {
            info!("Unsetting pointer capability");
            self.pointer_state.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
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
                        // self.layer.commit();
                    }
                }
                Leave { .. } => {
                    info!("Pointer left: {:?}", event.position);
                    self.slint_part.as_ref().unwrap().adapters[0]
                        .try_dispatch_event(WindowEvent::PointerExited)
                        .unwrap();
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
                        .unwrap();
                }
                Press { button, .. } => {
                    trace!("Press {:x} @ {:?}", button, event.position);
                    self.slint_part.as_ref().unwrap().adapters[0]
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
                    trace!("Release {:x} @ {:?}", button, event.position);
                    self.slint_part.as_ref().unwrap().adapters[0]
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
                        .unwrap();
                }
            }
        }
    }
}
/// It is an internal struct used by [`SpellLock`] internally.
pub struct SpellSlintLock {
    pub(crate) adapters: Vec<std::rc::Rc<SpellSkiaWinAdapter>>,
    pub(crate) size: Vec<PhysicalSize>,
    pub(crate) wayland_buffer: Vec<Buffer>,
}
