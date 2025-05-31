use std::{convert::TryInto, rc::Rc, time::Instant};

use slint::platform::{PointerEventButton, WindowEvent, software_renderer::PhysicalRegion};
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
    shm::{
        Shm, ShmHandler,
        slot::{Buffer, SlotPool},
    },
};

use crate::{
    configure::{Rgba8Pixel, WindowConf},
    slint_adapter::SpellWinAdapter,
};

mod window_state;
use self::window_state::PointerState;

// Most of the dealings of buffer will take place through this, hence it will manage
// new buffers when creating new buffers, changing buffer size for reseize info etc.
pub struct MemoryManger {
    pub pool: SlotPool,
    pub shm: Shm,
    pub slint_buffer: Option<Vec<Rgba8Pixel>>,
    pub ren_buffer: Box<[Rgba8Pixel]>,
}

impl MemoryManger {
    pub fn set_ren_buffers(&mut self, ren_buffer: Box<[Rgba8Pixel]>) {
        self.ren_buffer = ren_buffer;
    }
}

pub struct SpellWin {
    pub window: Rc<SpellWinAdapter>,
    pub slint_buffer: Option<Vec<Rgba8Pixel>>,
    pub primary_buffer: Buffer,
    pub secondary_buffer: Buffer,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub pointer_state: PointerState,
    pub memory_manager: MemoryManger,
    pub layer: LayerSurface,
    pub keyboard_focus: bool,
    pub first_configure: bool,
    pub damaged_part: Option<PhysicalRegion>,
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
        let pool = SlotPool::new((window_conf.width * window_conf.height * 4) as usize, &shm)
            .expect("Failed to create pool");
        let cursor_manager =
            CursorShapeManager::bind(&globals, &qh).expect("cursor shape is not available");
        let stride = window_conf.window.size.width as i32 * 4;
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

        let mut memory_manager = MemoryManger {
            slint_buffer: None,
            pool,
            shm,
            ren_buffer: Box::new([Rgba8Pixel::default()]),
        };

        let (primary_buffer, _) = memory_manager
            .pool
            .create_buffer(
                window_conf.width as i32,
                window_conf.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating Buffer");

        let (secondary_buffer, _) = memory_manager
            .pool
            .create_buffer(
                window_conf.width as i32,
                window_conf.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating secondary buffer");

        let pointer_state = PointerState {
            pointer: None,
            pointer_data: None,
            cursor_shape: cursor_manager,
        };

        (
            SpellWin {
                window: window_conf.window,
                slint_buffer: None,
                primary_buffer,
                secondary_buffer,
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),
                pointer_state,
                memory_manager,
                layer,
                keyboard_focus: false,
                first_configure: true,
                damaged_part: None,
            },
            event_queue,
        )
    }

    pub fn set_buffer(&mut self, buffer: Vec<Rgba8Pixel>) {
        self.slint_buffer = Some(buffer);
    }

    fn converter(&mut self, qh: &QueueHandle<Self>) {
        let width = self.window.size.width;
        let height = self.window.size.height;
        let window_adapter = self.window.clone();
        let mut work_buffer = self.memory_manager.ren_buffer.clone();

        slint::platform::update_timers_and_animations();
        let time_ren = Instant::now();
        window_adapter.draw_if_needed(|renderer| {
            // println!("Rendering");
            let physical_region = renderer.render(&mut work_buffer, width as usize);
            self.set_damaged(physical_region);
            self.set_buffer(work_buffer.to_vec());
        });
        println!("Render Time: {}", time_ren.elapsed().as_millis());

        // let time_gone = now.elapsed().as_millis();
        // println!("Render time: {}", time_gone);
        // core::mem::swap::<&mut [Rgba8Pixel]>(
        //     &mut &mut *work_buffer,
        //     &mut &mut *currently_displayed_buffer,
        // );

        let pool = &mut self.memory_manager.pool;
        let buffer = &self.primary_buffer;
        let sec_buffer = &self.secondary_buffer;
        let mut sec_canvas_data = {
            let sec_canvas = sec_buffer.canvas(pool).unwrap();
            sec_canvas.to_vec()
        };
        let mut primary_canvas = buffer.canvas(pool).unwrap();
        // Drawing the window
        let now = std::time::Instant::now();
        {
            primary_canvas
                .chunks_exact_mut(4)
                .enumerate()
                .for_each(|(index, chunk)| {
                    let a = self.slint_buffer.as_ref().unwrap()[index].a;
                    let r = self.slint_buffer.as_ref().unwrap()[index].r;
                    let g = self.slint_buffer.as_ref().unwrap()[index].g;
                    let b = self.slint_buffer.as_ref().unwrap()[index].b;
                    let color: u32 =
                        ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

                    let array: &mut [u8; 4] = chunk.try_into().unwrap();
                    *array = color.to_le_bytes();
                });
        }

        let elaspesed_time = now.elapsed().as_millis();
        println!("{}", elaspesed_time);

        // Damage the entire window
        if self.first_configure {
            self.first_configure = false;
            self.layer
                .wl_surface()
                .damage_buffer(0, 0, width as i32, height as i32);
        } else {
            for (position, size) in self.damaged_part.as_ref().unwrap().iter() {
                // println!(
                //     "{}, {}, {}, {}",
                //     position.x, position.y, size.width as i32, size.height as i32,
                // );
                // if size.width != width && size.height != height {
                self.layer.wl_surface().damage_buffer(
                    position.x,
                    position.y,
                    size.width as i32,
                    size.height as i32,
                );
                //}
            }
        }

        // Request our next frame
        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());

        // Attach and commit to present.
        buffer
            .attach_to(self.layer.wl_surface())
            .expect("buffer attach");

        self.layer.commit();

        core::mem::swap::<&mut [u8]>(&mut sec_canvas_data.as_mut_slice(), &mut primary_canvas);

        // TODO save and reuse buffer when the window size is unchanged.  This is especially
        // useful if you do damage tracking, since you don't need to redraw the undamaged parts
        // of the canvas.
    }

    pub fn set_damaged(&mut self, physical_region: PhysicalRegion) {
        self.damaged_part = Some(physical_region);
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
        // println!("Frame is called");
        self.converter(qh);
        // println!("Next draws called");
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
        // self.window.size.width = NonZeroU32::new(configure.new_size.0).map_or(256, NonZeroU32::get);
        // self.window.size.height =
        //     NonZeroU32::new(configure.new_size.1).map_or(256, NonZeroU32::get);

        // Initiate the first draw.
        if self.first_configure {
            self.converter(qh);
            // println!("First draw called");
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
                    self.window
                        .window
                        .try_dispatch_event(WindowEvent::PointerExited)
                        .unwrap();
                }
                Motion { .. } => {
                    // println!("Pointer entered @{:?}", event.position);
                    self.window
                        .window
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
                    self.window
                        .window
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
                    self.window
                        .window
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

// FIND What is the use of registery_handlers here?
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
