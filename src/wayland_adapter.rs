use std::{cell::RefCell, rc::Rc};

use slint::{
    PhysicalSize,
    platform::{PlatformError, PointerEventButton, WindowEvent},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            Connection, EventQueue, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
        },
        protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{
            PointerData, PointerEvent, PointerEventKind, PointerHandler,
            cursor_shape::CursorShapeManager,
        },
    },
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};

use crate::{
    configure::WindowConf,
    shared_context::{MemoryManager, SharedCore},
};

pub mod window_state;
use self::window_state::PointerState;

// This trait helps in defining specifc functions that would be required to run
// inside the SpellWin. Benefit of this abstraction is that I am sure that every function
// I am defining works even if inside `Rc`, i.e. only using non interior mutability
// functions.
pub trait EventAdapter {
    fn draw_if_needed(&self) -> bool;
    fn try_dispatch_event(&self, event: WindowEvent) -> Result<(), PlatformError>;
}

pub struct SpellWin {
    pub adapter: Rc<dyn EventAdapter>,
    pub core: Rc<RefCell<SharedCore>>,
    pub size: PhysicalSize,
    pub memory_manager: MemoryManager,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub pointer_state: PointerState,
    pub layer: LayerSurface,
    pub keyboard_focus: bool,
    pub first_configure: bool,
}

impl SpellWin {
    pub fn invoke_spell(name: &str, window_conf: WindowConf) -> (Self, EventQueue<SpellWin>) {
        // Initialisation of wayland components.
        let conn = Connection::connect_to_env().unwrap();
        let (globals, event_queue) = registry_queue_init(&conn).unwrap();
        let qh: QueueHandle<SpellWin> = event_queue.handle();
        let compositor =
            CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");
        let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");
        let mut pool = SlotPool::new((window_conf.width * window_conf.height * 4) as usize, &shm)
            .expect("Failed to create pool");
        let cursor_manager =
            CursorShapeManager::bind(&globals, &qh).expect("cursor shape is not available");
        let stride = window_conf.width as i32 * 4;
        let surface = compositor.create_surface(&qh);
        let mut layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            window_conf.layer_type,
            Some(name),
            None,
        );
        // layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_size(window_conf.width, window_conf.height);
        layer.set_margin(
            window_conf.margin.0,
            window_conf.margin.1,
            window_conf.margin.2,
            window_conf.margin.3,
        );
        set_anchor(&window_conf, &mut layer);
        layer.commit();

        let (wayland_buffer, _) = pool
            .create_buffer(
                window_conf.width as i32,
                window_conf.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating Buffer");

        let memory_manager = MemoryManager {
            pool,
            shm,
            wayland_buffer,
        };

        let pointer_state = PointerState {
            pointer: None,
            pointer_data: None,
            cursor_shape: cursor_manager,
        };

        (
            SpellWin {
                adapter: window_conf.adapter,
                core: window_conf.shared_core,
                size: PhysicalSize {
                    width: window_conf.width,
                    height: window_conf.height,
                },
                memory_manager,
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),
                pointer_state,
                layer,
                keyboard_focus: false,
                first_configure: true,
            },
            event_queue,
        )
    }

    fn converter(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.size.width;
        let height: u32 = self.size.height;
        let window_adapter = self.adapter.clone();

        // Rendering from Skia
        let skia_now = std::time::Instant::now();
        window_adapter.draw_if_needed();
        println!("Skia Elapsed Time: {}", skia_now.elapsed().as_millis());

        let pool = &mut self.memory_manager.pool;
        let buffer = &self.memory_manager.wayland_buffer;
        let primary_canvas = buffer.canvas(pool).unwrap();
        // Drawing the window
        let now = std::time::Instant::now();
        {
            primary_canvas
                .iter_mut()
                .enumerate()
                .for_each(|(index, val)| {
                    *val = self.core.borrow().primary_buffer[index];
                });
        }

        println!("Normal Elapsed Time: {}", now.elapsed().as_millis());

        // Damage the entire window
        // if self.first_configure {
        self.first_configure = false;
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
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
        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());

        // Attach and commit to present.
        buffer
            .attach_to(self.layer.wl_surface())
            .expect("buffer attach");

        self.layer.commit();

        // core::mem::swap::<&mut [u8]>(&mut sec_canvas_data.as_mut_slice(), &mut primary_canvas);
        // core::mem::swap::<&mut [Rgba8Pixel]>( &mut &mut *work_buffer, &mut &mut *currently_displayed_buffer,);

        // TODO save and reuse buffer when the window size is unchanged.  This is especially
        // useful if you do damage tracking, since you don't need to redraw the undamaged parts
        // of the canvas.
    }
}

delegate_compositor!(SpellWin);
delegate_registry!(SpellWin);
delegate_output!(SpellWin);
delegate_shm!(SpellWin);
delegate_seat!(SpellWin);
// delegate_keyboard!(SpellWin);
delegate_pointer!(SpellWin);
delegate_layer!(SpellWin);

impl ShmHandler for SpellWin {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.memory_manager.shm
    }
}

impl OutputHandler for SpellWin {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl CompositorHandler for SpellWin {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.converter(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // Not needed for this example.
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // Not needed for this example.
    }
}

impl LayerShellHandler for SpellWin {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        // self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // THis error TODO find if it is necessary.
        // self.adapter.size.width = NonZeroU32::new(configure.new_size.0).map_or(256, NonZeroU32::get);
        // self.adapter.size.height =
        //     NonZeroU32::new(configure.new_size.1).map_or(256, NonZeroU32::get);

        // Initiate the first draw.
        if self.first_configure {
            self.converter(qh);
            println!("First draw called");
        }
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
        // if capability == Capability::Keyboard && self.keyboard.is_none() {
        //     println!("Set keyboard capability");
        //     let keyboard = self
        //         .seat_state
        //         .get_keyboard(qh, &seat, None)
        //         .expect("Failed to create keyboard");
        //     self.keyboard = Some(keyboard);
        // }
        //
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
        // if capability == Capability::Keyboard && self.keyboard.is_some() {
        //     println!("Unset keyboard capability");
        //     self.keyboard.take().unwrap().release();
        // }

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
                    // TODO Axis and Scroll events are still to be mapped.
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
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

fn set_anchor(window_conf: &WindowConf, layer: &mut LayerSurface) {
    match window_conf.anchor.0 {
        Some(mut first_anchor) => match window_conf.anchor.1 {
            Some(sec_anchor) => {
                first_anchor.set(sec_anchor, true);
                layer.set_anchor(first_anchor);
            }
            None => {
                layer.set_anchor(first_anchor);
                if window_conf.exclusive_zone {
                    match first_anchor {
                        Anchor::LEFT | Anchor::RIGHT => {
                            layer.set_exclusive_zone(window_conf.width as i32)
                        }
                        Anchor::TOP | Anchor::BOTTOM => {
                            layer.set_exclusive_zone(window_conf.height as i32)
                        }
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
                if window_conf.exclusive_zone {
                    match sec_anchor {
                        Anchor::LEFT | Anchor::RIGHT => {
                            layer.set_exclusive_zone(window_conf.width as i32)
                        }
                        Anchor::TOP | Anchor::BOTTOM => {
                            layer.set_exclusive_zone(window_conf.height as i32)
                        }
                        // Other Scenarios involve Calling the Anchor on 2 sides ( i.e. corners)
                        // in which case no exclusive_zone will be set.
                        _ => {}
                    }
                }
            }
        }
    }
}

// Technically, there is no requirement of pool once it is used to create the
// buffers for the window, but it may be possible that later somewhere we can
// use it to share the resources between windows.
