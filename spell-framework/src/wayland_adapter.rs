//! It provides various widget types for implementing properties
//! across various functionalities for your shell. The most common and only workable widget (or
//! window as called by many) is [SpellWin] now. Future implementation of mentioned struct will
//! take place in near future.
use slint::{
    PhysicalSize,
    platform::{PlatformError, WindowEvent},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, EventQueue, QueueHandle,
        globals::registry_queue_init,
        protocol::{wl_output, wl_region::WlRegion, wl_shm, wl_surface},
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
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{
    configure::{LayerConf, WindowConf},
    dbus_window_state::{KeyboardState, PointerState},
    shared_context::{MemoryManager, SharedCore},
    slint_adapter::{SpellLayerShell, SpellMultiWinHandler, SpellSkiaWinAdapter},
    wayland_adapter::states_and_handles::set_anchor,
};

mod states_and_handles;

// This trait helps in defining specifc functions that would be required to run
// inside the SpellWinInternal. Benefit of this abstraction is that I am sure that every function
// I am defining works even if inside `Rc`, i.e. only using non interior mutability
// functions.
pub(crate) trait EventAdapter: std::fmt::Debug {
    fn draw_if_needed(&self) -> bool;
    fn try_dispatch_event(&self, event: WindowEvent) -> Result<(), PlatformError>;
}

/// `SpellWin` is the main type for implementing widgets, it covers various properties and trait
/// implementation, thus providing various available features. ///
/// ## Panics
///
/// The constructor method [conjure_spells](crate::wayland_adapter::SpellWin::conjure_spells) will
/// panic if the number of WindowConfs provided is not equal to the amount of widgets that are
/// initialised in the scope. The solution to avoid panic is to add more `let _ =
/// WidgetName::new().unwrap();` for all the widgets/window components you are declaring in your
/// slint files and adding [WindowConf]s for in [SpellMultiWinHandler].
#[derive(Debug, Clone)]
pub struct SpellWin {
    pub(crate) window: Rc<RefCell<SpellWinInternal>>,
    pub(crate) queue: Rc<RefCell<EventQueue<SpellWinInternal>>>,
}

impl SpellWin {
    pub fn conjure_spells(
        windows: Rc<RefCell<SpellMultiWinHandler>>,
        // current_display_specs: Vec<(usize, usize, usize, usize)>,
    ) -> Vec<Self> {
        SpellWinInternal::conjure_spells(windows)
            .iter()
            .map(|(internal, queue)| SpellWin {
                window: internal.clone(),
                queue: queue.clone(),
            })
            .collect()
    }

    pub fn invoke_spell(
        name: &str,
        window_conf: WindowConf,
        // current_display_specs: (usize, usize, usize, usize),
    ) -> Self {
        let internal = SpellWinInternal::invoke_spell(name, window_conf);
        SpellWin {
            window: internal.0,
            queue: internal.1,
        }
    }

    pub fn toggle(&self) {
        self.window.borrow().toggle();
    }

    pub fn hide(&self) {
        self.window.borrow().hide();
    }

    pub fn add_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.window.borrow().add_input_region(x, y, width, height);
    }

    pub fn subtract_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.window
            .borrow()
            .subtract_input_region(x, y, width, height);
    }

    pub fn add_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.window.borrow().add_opaque_region(x, y, width, height);
    }

    pub fn subtract_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.window
            .borrow()
            .subtract_opaque_region(x, y, width, height);
    }

    pub fn show_again(&self) {
        self.window.borrow().show_again()
    }

    pub fn grab_focus(&self) {
        self.window.borrow().grab_focus();
    }
    pub fn remove_focus(&self) {
        self.window.borrow().remove_focus();
    }
}

#[derive(Debug)]
pub(crate) struct States {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) pointer_state: PointerState,
    pub(crate) keyboard_state: KeyboardState,
    pub(crate) shm: Shm,
}

#[derive(Debug)]
pub(crate) struct SpellWinInternal {
    pub(crate) adapter: Rc<dyn EventAdapter>,
    pub(crate) core: Rc<RefCell<SharedCore>>,
    pub(crate) size: PhysicalSize,
    pub(crate) memory_manager: RefCell<MemoryManager>,
    pub(crate) pool: RefCell<SlotPool>,
    pub(crate) states: States,
    pub(crate) layer: RefCell<LayerSurface>,
    pub(crate) first_configure: bool,
    pub(crate) is_hidden: Cell<bool>,
    pub(crate) layer_name: String,
    pub(crate) config: WindowConf,
    pub(crate) input_region: Region,
    pub(crate) opaque_region: Region,
}

impl SpellWinInternal {
    fn create_window(
        conn: &Connection,
        window_conf: WindowConf,
        layer_name: String,
        adapter: Option<Rc<SpellSkiaWinAdapter>>,
        core: Option<Rc<RefCell<SharedCore>>>,
    ) -> (Rc<RefCell<Self>>, Rc<RefCell<EventQueue<SpellWinInternal>>>) {
        let (globals, event_queue) = registry_queue_init(conn).unwrap();
        let qh: QueueHandle<SpellWinInternal> = event_queue.handle();
        let compositor =
            CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");
        let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");
        let mut pool = SlotPool::new((window_conf.width * window_conf.height * 4) as usize, &shm)
            .expect("Failed to create pool");
        let input_region = Region::new(&compositor).expect("Couldn't create region");
        let opaque_region = Region::new(&compositor).expect("Couldn't create opaque region");
        input_region.add(0, 0, window_conf.width as i32, window_conf.height as i32);
        let cursor_manager =
            CursorShapeManager::bind(&globals, &qh).expect("cursor shape is not available");
        let stride = window_conf.width as i32 * 4;
        let surface = compositor.create_surface(&qh);
        let mut layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            window_conf.layer_type,
            Some(layer_name.clone()),
            None,
        );
        set_config(
            &window_conf,
            &mut layer,
            true,
            Some(input_region.wl_region()),
            None,
        );
        layer.commit();
        let (wayland_buffer, _) = pool
            .create_buffer(
                window_conf.width as i32,
                window_conf.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating Buffer");

        let memory_manager = MemoryManager { wayland_buffer };

        let pointer_state = PointerState {
            pointer: None,
            pointer_data: None,
            cursor_shape: cursor_manager,
        };

        let keyboard_state = KeyboardState {
            board: None,
            board_data: None,
        };

        // These 2 unwrap or else statements are not connected programmitically
        // though I know that either both will be none or both will have some value.
        let core_val: Rc<RefCell<SharedCore>> = core.unwrap_or_else(|| {
            Rc::new(RefCell::new(SharedCore::new(
                window_conf.width,
                window_conf.height,
            )))
        });

        let adapter_value: Rc<SpellSkiaWinAdapter> = adapter.unwrap_or_else(|| {
            let adapter_val =
                SpellSkiaWinAdapter::new(core_val.clone(), window_conf.width, window_conf.height);

            let _ = slint::platform::set_platform(Box::new(SpellLayerShell {
                window_adapter: adapter_val.clone(),
            }));
            adapter_val
        });
        (
            Rc::new(RefCell::new(SpellWinInternal {
                adapter: adapter_value,
                core: core_val,
                size: PhysicalSize {
                    width: window_conf.width,
                    height: window_conf.height,
                },
                memory_manager: RefCell::new(memory_manager),
                pool: RefCell::new(pool),
                states: States {
                    registry_state: RegistryState::new(&globals),
                    seat_state: SeatState::new(&globals, &qh),
                    output_state: OutputState::new(&globals, &qh),
                    pointer_state,
                    keyboard_state,
                    shm,
                },
                layer: RefCell::new(layer),
                first_configure: true,
                is_hidden: Cell::new(false),
                layer_name,
                config: window_conf,
                input_region,
                opaque_region,
            })),
            Rc::new(RefCell::new(event_queue)),
        )
    }

    fn conjure_spells(
        windows: Rc<RefCell<SpellMultiWinHandler>>,
        // current_display_specs: Vec<(usize, usize, usize, usize)>,
    ) -> Vec<(Rc<RefCell<Self>>, Rc<RefCell<EventQueue<SpellWinInternal>>>)> {
        let mut win_and_queue = Vec::new();
        // for handler in windows.borrow()
        let window_length = windows.borrow().windows.len();
        let adapter_length = windows.borrow().adapter.len();
        let core_length = windows.borrow().core.len();
        if window_length == adapter_length && adapter_length == core_length
        // && adapter_length == current_display_specs.len()
        {
            let conn = Connection::connect_to_env().unwrap();
            windows
                .borrow()
                .adapter
                .iter()
                .enumerate()
                .for_each(|(index, val)| {
                    if let LayerConf::Window(window_conf) = &windows.borrow().windows[index].1 {
                        win_and_queue.push(SpellWinInternal::create_window(
                            &conn,
                            window_conf.clone(),
                            windows.borrow().windows[index].0.clone(),
                            // current_display_specs[index],
                            Some(val.clone()),
                            Some(windows.borrow().core[index].clone()),
                        ));
                    }
                });
        } else {
            panic!(
                "The length of window configs and shared cores is not equal to activated windows, {} {} {}",
                window_length, core_length, adapter_length
            );
        }
        win_and_queue
    }

    fn invoke_spell(
        name: &str,
        window_conf: WindowConf,
        // current_display_specs: (usize, usize, usize, usize),
    ) -> (Rc<RefCell<Self>>, Rc<RefCell<EventQueue<SpellWinInternal>>>) {
        // Initialisation of wayland components.
        let conn = Connection::connect_to_env().unwrap();
        SpellWinInternal::create_window(
            &conn,
            window_conf.clone(),
            name.to_string(),
            // current_display_specs,
            None,
            None,
        )
    }

    fn hide(&self) {
        self.is_hidden.set(true);
        self.layer.borrow().wl_surface().attach(None, 0, 0);
    }

    fn toggle(&self) {
        if self.is_hidden.get() {
            self.show_again();
        } else {
            self.hide();
        }
    }

    fn add_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.input_region.add(x, y, width, height);
        self.set_config_internal();
        self.layer.borrow().commit();
    }

    fn subtract_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.input_region.subtract(x, y, width, height);
        self.set_config_internal();
        self.layer.borrow().commit();
    }

    fn add_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.opaque_region.add(x, y, width, height);
        self.set_config_internal();
        self.layer.borrow().commit();
    }

    fn subtract_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.opaque_region.subtract(x, y, width, height);
        self.set_config_internal();
        self.layer.borrow().commit();
    }

    // fn resize_display(&mut self, x: usize, y: usize, width: usize, height: usize) {
    //     let pool = &mut self.memory_manager.pool;
    //     let (wayland_buffer, _) = pool
    //         .create_buffer(
    //             width as i32,
    //             height as i32,
    //             width as i32 * 4,
    //             wl_shm::Format::Argb8888,
    //         )
    //         .expect("Creating Buffer");
    //     {
    //         wayland_buffer
    //             .canvas(pool)
    //             .unwrap()
    //             .iter_mut()
    //             .enumerate()
    //             .for_each(|(index, val)| {
    //                 *val = self.core.borrow().primary_buffer[index];
    //             });
    //     }
    //     println!("Window Resized");
    //     // self.memory_manager.prima
    //     self.memory_manager.wayland_buffer = wayland_buffer;
    //     self.set_config_internal();
    //     self.layer.commit();
    // }
    //
    // TODO this doesn't seem to trace.
    // TODO have to fix buffer creation for resizeable windows.
    #[tracing::instrument]
    fn show_again(&self) {
        let width: u32 = self.size.width;
        let height: u32 = self.size.height;
        let pool = &mut self.pool.borrow_mut();
        let (wayland_buffer, _) = pool
            .create_buffer(
                width as i32,
                height as i32,
                (width * 4) as i32,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating Buffer");
        // TODO this was previously set, if rendering causes issues, uncomment this.
        // self.set_config_internal();

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
        self.set_config_internal();
        self.is_hidden.set(false);
        self.layer.borrow().commit();
    }

    fn set_config_internal(&self) {
        set_config(
            &self.config,
            &self.layer.borrow(),
            self.first_configure,
            Some(self.input_region.wl_region()),
            Some(self.opaque_region.wl_region()),
        );
    }

    fn converter(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.size.width;
        let height: u32 = self.size.height;
        let window_adapter = self.adapter.clone();

        // Rendering from Skia
        if !self.is_hidden.get() {
            // let skia_now = std::time::Instant::now();
            let redraw_val: bool = window_adapter.draw_if_needed();
            // println!("Skia Elapsed Time: {}", skia_now.elapsed().as_millis());

            let pool = &mut self.pool.borrow_mut();
            let buffer = &self.memory_manager.borrow().wayland_buffer;
            let primary_canvas = buffer.canvas(pool).unwrap();

            // println!("{}", primary_canvas.len());
            // Drawing the window
            // let now = std::time::Instant::now();
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
            // println!("Normal Elapsed Time: {}", now.elapsed().as_millis());

            // Damage the entire window
            // if self.first_configure {
            self.first_configure = false;
            self.layer
                .borrow()
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
                .borrow()
                .wl_surface()
                .frame(qh, self.layer.borrow().wl_surface().clone());
            self.layer
                .borrow()
                .wl_surface()
                .attach(Some(buffer.wl_buffer()), 0, 0);
        } else {
            // println!("Is_hidden is true.");
        }

        self.layer.borrow().commit();
        // core::mem::swap::<&mut [u8]>(&mut sec_canvas_data.as_mut_slice(), &mut primary_canvas);
        // core::mem::swap::<&mut [Rgba8Pixel]>( &mut &mut *work_buffer, &mut &mut *currently_displayed_buffer,);

        // TODO save and reuse buffer when the window size is unchanged.  This is especially
        // useful if you do damage tracking, since you don't need to redraw the undamaged parts
        // of the canvas.
    }

    fn grab_focus(&self) {
        self.config
            .board_interactivity
            .set(KeyboardInteractivity::Exclusive);
        self.layer
            .borrow()
            .set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        self.layer.borrow().commit();
    }

    fn remove_focus(&self) {
        self.config
            .board_interactivity
            .set(KeyboardInteractivity::None);
        self.layer
            .borrow()
            .set_keyboard_interactivity(KeyboardInteractivity::None);
        self.layer.borrow().commit();
    }
}

delegate_compositor!(SpellWinInternal);
delegate_registry!(SpellWinInternal);
delegate_output!(SpellWinInternal);
delegate_shm!(SpellWinInternal);
delegate_seat!(SpellWinInternal);
delegate_keyboard!(SpellWinInternal);
delegate_pointer!(SpellWinInternal);
delegate_layer!(SpellWinInternal);

impl ShmHandler for SpellWinInternal {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.states.shm
    }
}

impl OutputHandler for SpellWinInternal {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.states.output_state
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

impl CompositorHandler for SpellWinInternal {
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

impl LayerShellHandler for SpellWinInternal {
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
        // println!("Config event is called");
        if !self.first_configure {
            self.first_configure = true;
        } else {
            // println!("[{}]: First draw called", self.layer_name);
        }
        self.converter(qh);
    }
}

fn set_config(
    window_conf: &WindowConf,
    layer: &LayerSurface,
    first_configure: bool,
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
    set_anchor(window_conf, layer, first_configure);
}

// fn render_replace(
//     primary_canvas: &mut [u8],
//     shared_core: &[u8],
//     mut dimenstions: (usize, usize, usize, usize),
//     mut shared_core_original_dimentions: (u32, u32),
// ) {
//     let (ref mut core_width, ref mut core_height) = shared_core_original_dimentions;
//     let (ref mut x, y, ref mut width, ref mut height) = dimenstions;
//     if *x + *width > *core_width as usize {
//         *width = *core_width as usize - *x
//     } else if y + *height > *core_height as usize {
//         *height = *core_height as usize - y
//     }
//
//     *width *= 4;
//     *x *= 4;
//     *core_width *= 4;
//     let mut shared_buffer_index = (y * *core_width as usize) + *x;
//     let mut wayland_buffer_index = 0;
//     let jump = (*core_width as usize) - *width;
//     for _ in 0..*height as u32 {
//         for _ in 0..(*width as u32) / 4 {
//             primary_canvas[wayland_buffer_index] = shared_core[shared_buffer_index];
//             primary_canvas[wayland_buffer_index + 1] = shared_core[shared_buffer_index + 1];
//             primary_canvas[wayland_buffer_index + 2] = shared_core[shared_buffer_index + 2];
//             primary_canvas[wayland_buffer_index + 3] = shared_core[shared_buffer_index + 3];
//             shared_buffer_index += 4;
//             wayland_buffer_index += 4;
//         }
//         shared_buffer_index += jump;
//     }
// }
//
/// Furture lock screen implementation will be on this type. Currently, it is redundent.
pub struct SpellLock;
/// Furture virtual keyboard implementation will be on this type. Currently, it is redundent.
pub struct SpellBoard;

// TODO mention that panic will occur if wayland windows are initialised before slint windows.
