//! It provides various widget types for implementing properties
//! across various functionalities for your shell. The most common and only workable widget (or
//! window as called by many) is [SpellWin] now. Future implementation of mentioned struct will
//! take place in near future.
use slint::{
    PhysicalSize,
    platform::{PlatformError, WindowEvent},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, EventQueue, QueueHandle,
        globals::registry_queue_init,
        protocol::{wl_output, wl_shm, wl_surface},
    },
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager},
    shell::{
        WaylandSurface,
        wlr_layer::{
            KeyboardInteractivity, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use std::{cell::RefCell, rc::Rc};

use crate::{
    configure::{LayerConf, WindowConf},
    dbus_window_state::{KeyboardState, PointerState},
    shared_context::{MemoryManager, SharedCore},
    slint_adapter::{SpellLayerShell, SpellMultiWinHandler, SpellSkiaWinAdapter},
    wayland_adapter::states_and_handles::set_anchor,
};

mod states_and_handles;

// This trait helps in defining specifc functions that would be required to run
// inside the SpellWin. Benefit of this abstraction is that I am sure that every function
// I am defining works even if inside `Rc`, i.e. only using non interior mutability
// functions.
pub(crate) trait EventAdapter: std::fmt::Debug {
    fn draw_if_needed(&self) -> bool;
    fn try_dispatch_event(&self, event: WindowEvent) -> Result<(), PlatformError>;
}

/// `SpellWin` is the main type for implementing widgets, it covers various properties and trait
/// implementation, thus providing various available features. Methods of this struct can't be
/// accessed directly as it stores non-sharable types and states inside, which binds it to a single
/// instant passed to the event loop. Hence, its methods are accessed indirectly via
/// [Handle](crate::Handle) passed through a mpsc Sender. Another possible solution would be the
/// usage of `Rc<RefCell<SpellWin>>` which was avoided, as it introduces a lot of boilerplate while
/// using the library and also internally. Future effort will try to implement Copy trait on it, or
/// a better alternative to call struct methods.
///
/// ## Panics
///
/// The constructor method [conjure_spells](crate::wayland_adapter::SpellWin::conjure_spells) will
/// panic if the number of WindowConfs provided is not equal to the amount of widgets that are
/// initialised in the scope. The solution to avoid panic is to add more `let _ =
/// WidgetName::new().unwrap();` for all the widgets/window components you are declaring in your
/// slint files and adding [WindowConf]s for in [SpellMultiWinHandler].
#[derive(Debug)]
pub struct SpellWin {
    pub(crate) adapter: Rc<dyn EventAdapter>,
    pub(crate) core: Rc<RefCell<SharedCore>>,
    pub(crate) size: PhysicalSize,
    pub(crate) memory_manager: MemoryManager,
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) pointer_state: PointerState,
    pub(crate) keyboard_state: KeyboardState,
    pub(crate) layer: LayerSurface,
    pub(crate) first_configure: bool,
    pub(crate) is_hidden: bool,
    pub(crate) layer_name: String,
    pub(crate) config: WindowConf,
}

impl SpellWin {
    pub fn conjure_spells(
        windows: Rc<RefCell<SpellMultiWinHandler>>,
    ) -> Vec<(Self, EventQueue<SpellWin>)> {
        let mut win_and_queue: Vec<(SpellWin, EventQueue<SpellWin>)> = Vec::new();
        // for handler in windows.borrow()
        let window_length = windows.borrow().windows.len();
        let adapter_length = windows.borrow().adapter.len();
        let core_length = windows.borrow().core.len();
        if window_length == adapter_length && adapter_length == core_length {
            let conn = Connection::connect_to_env().unwrap();
            windows
                .borrow()
                .adapter
                .iter()
                .enumerate()
                .for_each(|(index, val)| {
                    if let LayerConf::Window(window_conf) = &windows.borrow().windows[index].1 {
                        let (globals, event_queue) = registry_queue_init(&conn).unwrap();
                        let qh: QueueHandle<SpellWin> = event_queue.handle();
                        let compositor = CompositorState::bind(&globals, &qh)
                            .expect("wl_compositor is not available");
                        let layer_shell =
                            LayerShell::bind(&globals, &qh).expect("layer shell is not available");
                        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");
                        let mut pool = SlotPool::new(
                            (window_conf.width * window_conf.height * 4) as usize,
                            &shm,
                        )
                        .expect("Failed to create pool");
                        let cursor_manager = CursorShapeManager::bind(&globals, &qh)
                            .expect("cursor shape is not available");
                        let stride = window_conf.width as i32 * 4;
                        let surface = compositor.create_surface(&qh);
                        let mut layer = layer_shell.create_layer_surface(
                            &qh,
                            surface,
                            window_conf.layer_type,
                            Some(windows.borrow().windows[index].0.clone()),
                            None,
                        );
                        // layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
                        set_config(window_conf, &mut layer);
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

                        let keyboard_state = KeyboardState {
                            board: None,
                            board_data: None,
                        };

                        win_and_queue.push((
                            SpellWin {
                                adapter: val.clone(),
                                core: windows.borrow().core[index].clone(),
                                size: PhysicalSize {
                                    width: window_conf.width,
                                    height: window_conf.height,
                                },
                                memory_manager,
                                registry_state: RegistryState::new(&globals),
                                seat_state: SeatState::new(&globals, &qh),
                                output_state: OutputState::new(&globals, &qh),
                                pointer_state,
                                keyboard_state,
                                layer,
                                first_configure: true,
                                is_hidden: false,
                                layer_name: windows.borrow().windows[index].0.clone(),
                                config: window_conf.clone(),
                            },
                            event_queue,
                        ));
                    }
                });
        } else {
            panic!(
                "The length of window configs and shared cores is not equal to activated windows"
            );
        }
        win_and_queue
    }

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
        set_config(&window_conf, &mut layer);
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

        let keyboard_state = KeyboardState {
            board: None,
            board_data: None,
        };
        // Initialisation of slint Components.
        let core = Rc::new(RefCell::new(SharedCore::new(
            window_conf.width,
            window_conf.height,
        )));
        let adapter = SpellSkiaWinAdapter::new(core.clone(), window_conf.width, window_conf.height);

        let _ = slint::platform::set_platform(Box::new(SpellLayerShell {
            window_adapter: adapter.clone(),
        }));

        (
            SpellWin {
                adapter,
                core,
                size: PhysicalSize {
                    width: window_conf.width,
                    height: window_conf.height,
                },
                memory_manager,
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),
                pointer_state,
                keyboard_state,
                layer,
                first_configure: true,
                is_hidden: false,
                layer_name: name.to_string(),
                config: window_conf,
            },
            event_queue,
        )
    }

    pub fn hide(&mut self) {
        self.is_hidden = true;
        self.layer.wl_surface().attach(None, 0, 0);
    }

    pub fn toggle(&mut self) {
        if self.is_hidden {
            self.show_again();
        } else {
            self.hide();
        }
    }

    // TODO this doesn't seem to trace.
    #[tracing::instrument]
    pub fn show_again(&mut self) {
        let width: u32 = self.size.width;
        let height: u32 = self.size.height;
        let pool = &mut self.memory_manager.pool;
        let (wayland_buffer, _) = pool
            .create_buffer(
                width as i32,
                height as i32,
                (width * 4) as i32,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating Buffer");
        // tracing::info!("tracing output: {}", buffer.canvas(pool).unwrap().len());
        {
            wayland_buffer
                .canvas(pool)
                .unwrap()
                .iter_mut()
                .enumerate()
                .for_each(|(index, val)| {
                    *val = self.core.borrow().primary_buffer[index];
                });
        }
        set_config(&self.config, &mut self.layer);

        self.is_hidden = false;
        self.layer.commit();
    }

    fn converter(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.size.width;
        let height: u32 = self.size.height;
        let window_adapter = self.adapter.clone();

        // Rendering from Skia
        if !self.is_hidden {
            let skia_now = std::time::Instant::now();
            let redraw_val: bool = window_adapter.draw_if_needed();
            println!("Skia Elapsed Time: {}", skia_now.elapsed().as_millis());

            let pool = &mut self.memory_manager.pool;
            let buffer = &self.memory_manager.wayland_buffer;
            let primary_canvas = buffer.canvas(pool).unwrap();

            // println!("{}", primary_canvas.len());
            // Drawing the window
            let now = std::time::Instant::now();
            if redraw_val || self.first_configure {
                {
                    primary_canvas
                        .iter_mut()
                        .enumerate()
                        .for_each(|(index, val)| {
                            *val = self.core.borrow().primary_buffer[index];
                        });
                }
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
            self.layer
                .wl_surface()
                .attach(Some(buffer.wl_buffer()), 0, 0);
        } else {
            println!("Is_hidden is true.");
        }

        self.layer.commit();
        // core::mem::swap::<&mut [u8]>(&mut sec_canvas_data.as_mut_slice(), &mut primary_canvas);
        // core::mem::swap::<&mut [Rgba8Pixel]>( &mut &mut *work_buffer, &mut &mut *currently_displayed_buffer,);

        // TODO save and reuse buffer when the window size is unchanged.  This is especially
        // useful if you do damage tracking, since you don't need to redraw the undamaged parts
        // of the canvas.
    }

    pub fn grab_focus(&self) {
        self.layer
            .set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        self.layer.commit();
    }

    pub fn remove_focus(&self) {
        self.layer
            .set_keyboard_interactivity(KeyboardInteractivity::None);
        self.layer.commit();
    }
}

delegate_compositor!(SpellWin);
delegate_registry!(SpellWin);
delegate_output!(SpellWin);
delegate_shm!(SpellWin);
delegate_seat!(SpellWin);
delegate_keyboard!(SpellWin);
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
        println!("New Output Source Added");
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
        println!("Output is destroyed");
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
        println!("Surface reentered");
        // Not needed for this example.
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        println!("Surface left");
        // Not needed for this example.
    }
}

impl LayerShellHandler for SpellWin {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        println!("CLosed of layer called");
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
        println!("Config event is called");
        if !self.first_configure {
            self.first_configure = true;
        } else {
            println!("First draw called");
        }
        self.converter(qh);
    }
}

fn set_config(window_conf: &WindowConf, layer: &mut LayerSurface) {
    layer.set_size(window_conf.width, window_conf.height);
    layer.set_margin(
        window_conf.margin.0,
        window_conf.margin.1,
        window_conf.margin.2,
        window_conf.margin.3,
    );
    layer.set_keyboard_interactivity(window_conf.board_interactivity);
    set_anchor(window_conf, layer);
}

/// Furture lock screen implementation will be on this type. Currently, it is redundent.
pub struct SpellLock;
/// Furture virtual keyboard implementation will be on this type. Currently, it is redundent.
pub struct SpellBoard;
