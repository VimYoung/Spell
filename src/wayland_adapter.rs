use std::{cell::Cell, convert::TryInto, rc::Rc};

use slint::platform::{PointerEventButton, WindowEvent, software_renderer::TargetPixel};
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            Connection, EventQueue, QueueHandle,
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
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};

use crate::slint_adapter::SpellWinAdapter;
use smithay_client_toolkit::{
    compositor::CompositorState,
    reexports::client::globals::registry_queue_init,
    shell::wlr_layer::{Anchor, Layer, LayerShell},
};

pub struct Rgba8Pixel {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgba8Pixel {
    pub fn new(a: u8, r: u8, g: u8, b: u8) -> Self {
        Rgba8Pixel { a, r, g, b }
    }
}

impl TargetPixel for Rgba8Pixel {
    fn blend(&mut self, color: slint::platform::software_renderer::PremultipliedRgbaColor) {
        let a: u16 = (u8::MAX - color.alpha) as u16;
        // self.a = a as u8;
        self.r = (self.r as u16 * a / 255) as u8 + color.red;
        self.g = (self.g as u16 * a / 255) as u8 + color.green;
        self.b = (self.b as u16 * a / 255) as u8 + color.blue;
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        let a = 0xFF;
        Self::new(a, red, green, blue)
    }

    fn background() -> Self {
        // TODO This needs to be decided to see how it should be 0xFF or 0x00;
        let a: u8 = 0x00;
        Self::new(a, 0, 0, 0)
    }
}

impl std::marker::Copy for Rgba8Pixel {}
impl std::clone::Clone for Rgba8Pixel {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct SpellWin {
    pub window: Rc<SpellWinAdapter>,
    pub slint_buffer: Option<Vec<Rgba8Pixel>>,
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub shm: Shm,
    pub pool: SlotPool,
    pub layer: LayerSurface,
    pub cursor_shape: CursorShapeManager,
    pub _keyboard_focus: bool,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_data: Option<PointerData>,
    pub exit: bool,
    pub first_configure: bool,
    pub render_again: Cell<bool>,
}

impl SpellWin {
    fn new(
        window: Rc<SpellWinAdapter>,
        slint_buffer: Option<Vec<Rgba8Pixel>>,
        registry_state: RegistryState,
        seat_state: SeatState,
        output_state: OutputState,
        shm: Shm,
        pool: SlotPool,
        layer: LayerSurface,
        cursor_shape: CursorShapeManager,
        _keyboard_focus: bool,
        pointer: Option<wl_pointer::WlPointer>,
        pointer_data: Option<PointerData>,
        exit: bool,
        first_configure: bool,
        render_again: Cell<bool>,
    ) -> Self {
        SpellWin {
            window,
            slint_buffer,
            registry_state,
            seat_state,
            output_state,
            shm,
            pool,
            layer,
            cursor_shape,
            _keyboard_focus,
            pointer,
            pointer_data,
            exit,
            first_configure,
            render_again,
        }
    }

    pub fn invoke_spell<'a>(
        name: &str,
        width: u32,
        height: u32,
        buffer1: &'a mut [Rgba8Pixel],
        buffer2: &'a mut [Rgba8Pixel],
        anchor: Anchor,
        layer_type: Layer,
        window: Rc<SpellWinAdapter>,
        exclusive_zone: bool,
    ) -> (
        Self,
        &'a mut [Rgba8Pixel],
        &'a mut [Rgba8Pixel],
        EventQueue<SpellWin>,
    ) {
        //configure wayland to use these bufferes.
        let currently_displayed_buffer: &mut [_] = buffer1;
        let work_buffer: &mut [_] = buffer2;

        // Initialisation of wayland components.
        let conn = Connection::connect_to_env().unwrap();
        let (globals, event_queue) = registry_queue_init(&conn).unwrap();
        let qh: QueueHandle<SpellWin> = event_queue.handle();

        let compositor =
            CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");
        let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");
        let cursor_manager =
            CursorShapeManager::bind(&globals, &qh).expect("cursor shape is not available");
        let surface = compositor.create_surface(&qh);

        let layer = layer_shell.create_layer_surface(&qh, surface, layer_type, Some(name), None);
        layer.set_anchor(anchor);
        // layer.set_anchor(Anchor::LEFT);
        // layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_size(width, height);
        if exclusive_zone {
            match anchor {
                Anchor::LEFT | Anchor::RIGHT => layer.set_exclusive_zone(width as i32),
                Anchor::TOP | Anchor::BOTTOM => layer.set_exclusive_zone(height as i32),
                // TODO This needs to be handled.
                _ => todo!(),
            }
        }
        // layer.set_exclusive_zone(400);
        layer.commit();
        let pool =
            SlotPool::new((width * height * 4) as usize, &shm).expect("Failed to create pool");

        (
            SpellWin::new(
                // (width, height),
                window,
                None,
                RegistryState::new(&globals),
                SeatState::new(&globals, &qh),
                OutputState::new(&globals, &qh),
                shm,
                pool,
                layer,
                cursor_manager,
                false,
                None,
                None,
                false,
                true,
                Cell::new(true),
            ),
            work_buffer,
            currently_displayed_buffer,
            event_queue,
        )
    }

    pub fn set_buffer(&mut self, buffer: Vec<Rgba8Pixel>) {
        self.slint_buffer = Some(buffer);
    }

    fn converter(&mut self, qh: &QueueHandle<Self>) {
        let width = self.window.size.width;
        let height = self.window.size.height;
        let stride = self.window.size.width as i32 * 4;
        let (buffer, canvas) = self
            .pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("create buffer");
        // Drawing the window
        {
            canvas
                .chunks_exact_mut(4)
                .enumerate()
                .for_each(|(index, chunk)| {
                    // let a: u8 = 0xFF;
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

        // Damage the entire window
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);

        // Request our next frame
        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());

        // Attach and commit to present.
        buffer
            .attach_to(self.layer.wl_surface())
            .expect("buffer attach");
        self.layer.commit();

        // TODO save and reuse buffer when the window size is unchanged.  This is especially
        // useful if you do damage tracking, since you don't need to redraw the undamaged parts
        // of the canvas.
    }

    // fn initialise_application(&mut self, mut event_queue: EventQueue<Self>) {
    //     self.event_queue.blocking_dispatch(self).unwrap();
    // }
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
        &mut self.shm
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
        self.render_again.set(true);
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
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // THis error
        // self.window.size.width = NonZeroU32::new(configure.new_size.0).map_or(256, NonZeroU32::get);
        // self.window.size.height =
        //     NonZeroU32::new(configure.new_size.1).map_or(256, NonZeroU32::get);

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.converter(qh);
            self.render_again.set(true);
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
        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            let pointer_data = PointerData::new(seat);
            self.pointer = Some(pointer);
            self.pointer_data = Some(pointer_data);
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

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
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
                    println!("Pointer entered @{:?}", event.position);

                    // TODO this code is redundent, as it doesn't set the cursor shape.
                    let pointer = &self.pointer.as_ref().unwrap();
                    let serial_no: Option<u32> =
                        self.pointer_data.as_ref().unwrap().latest_enter_serial();
                    if let Some(no) = serial_no {
                        println!("Cursor Shape set");
                        self.cursor_shape
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
