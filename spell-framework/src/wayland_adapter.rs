//! It provides various widget types for implementing properties
//! across various functionalities for your shell. The most common widget (or
//! window as called by many) is [SpellWin]. You can also implement a lock screen
//! with [`SpellLock`].
use slint::PhysicalSize;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_session_lock, delegate_shm,
    output::{self, OutputHandler, OutputState},
    reexports::{
        calloop::{EventLoop, LoopHandle},
        client::{
            Connection, EventQueue, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_output, wl_shm, wl_surface},
        },
    },
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager},
    session_lock::{SessionLock, SessionLockState, SessionLockSurface},
    shell::{
        WaylandSurface,
        wlr_layer::{
            KeyboardInteractivity, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{
        Shm, ShmHandler,
        slot::{Buffer, Slot, SlotPool},
    },
};

use crate::{
    Handle,
    configure::{LayerConf, WindowConf},
    slint_adapter::{SpellLayerShell, SpellLockShell, SpellMultiWinHandler, SpellSkiaWinAdapter},
    wayland_adapter::way_helper::{KeyboardState, PointerState, set_config},
};
pub use lock_impl::{SpellSlintLock, run_lock};
use nonstick::{
    AuthnFlags, Conversation, ConversationAdapter, Result as PamResult, Transaction,
    TransactionBuilder,
};
use std::{
    cell::{Cell, RefCell},
    ffi::{OsStr, OsString},
    process::Command,
    rc::Rc,
    sync::mpsc::{self, Receiver, Sender},
};
mod lock_impl;
mod way_helper;
mod win_impl;

#[derive(Debug)]
pub(crate) struct States {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) pointer_state: PointerState,
    pub(crate) keyboard_state: KeyboardState,
    pub(crate) shm: Shm,
}

/// `SpellWin` is the main type for implementing widgets, it covers various properties and trait
/// implementation, thus providing various features.
/// ## Panics
///
/// The constructor method [conjure_spells](crate::wayland_adapter::SpellWin::conjure_spells) will
/// panic if the number of WindowConfs provided is not equal to the amount of widgets that are
/// initialised in the scope. The solution to avoid panic is to add more `let _ =
/// WidgetName::new().unwrap();` for all the widgets/window components you are declaring in your
/// slint files and adding [WindowConf]s for in [SpellMultiWinHandler].
#[derive(Debug)]
pub struct SpellWin {
    pub(crate) adapter: Rc<SpellSkiaWinAdapter>,
    pub(crate) size: PhysicalSize,
    pub(crate) buffer: Buffer,
    pub(crate) states: States,
    pub(crate) layer: LayerSurface,
    pub(crate) first_configure: bool,
    pub(crate) is_hidden: Cell<bool>,
    pub(crate) layer_name: String,
    pub(crate) config: WindowConf,
    pub(crate) input_region: Region,
    pub(crate) opaque_region: Region,
    pub(crate) queue: Rc<RefCell<EventQueue<SpellWin>>>,
    pub(crate) handler: Option<Receiver<Handle>>,
}

impl SpellWin {
    pub(crate) fn create_window(
        conn: &Connection,
        window_conf: WindowConf,
        layer_name: String,
        if_single: bool,
    ) -> Self {
        let (globals, event_queue) = registry_queue_init(conn).unwrap();
        let qh: QueueHandle<SpellWin> = event_queue.handle();
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
        let layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            window_conf.layer_type,
            Some(layer_name.clone()),
            None,
        );
        set_config(
            &window_conf,
            &layer,
            true,
            Some(input_region.wl_region()),
            None,
        );
        layer.commit();
        let (way_pri_buffer, _) = pool
            .create_buffer(
                window_conf.width as i32,
                window_conf.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating Buffer");

        let primary_slot = way_pri_buffer.slot();

        let pointer_state = PointerState {
            pointer: None,
            pointer_data: None,
            cursor_shape: cursor_manager,
        };

        let keyboard_state = KeyboardState {
            board: None,
            board_data: None,
        };

        let adapter_value: Rc<SpellSkiaWinAdapter> = SpellSkiaWinAdapter::new(
            Rc::new(RefCell::new(pool)),
            RefCell::new(primary_slot),
            window_conf.width,
            window_conf.height,
        );

        if if_single {
            let _ = slint::platform::set_platform(Box::new(SpellLayerShell {
                window_adapter: adapter_value.clone(),
            }));
        }

        SpellWin {
            adapter: adapter_value,
            size: PhysicalSize {
                width: window_conf.width,
                height: window_conf.height,
            },
            buffer: way_pri_buffer,
            states: States {
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),
                pointer_state,
                keyboard_state,
                shm,
            },
            layer,
            first_configure: true,
            is_hidden: Cell::new(false),
            layer_name,
            config: window_conf,
            input_region,
            opaque_region,
            queue: Rc::new(RefCell::new(event_queue)),
            handler: None,
        }
    }

    /// retrive the sender handle to pass [`Handle`] events to the windows.
    pub fn get_handler(&mut self) -> Sender<Handle> {
        let (tx, rx) = mpsc::channel::<Handle>();
        self.handler = Some(rx);
        tx
    }

    /// This function is called to create a instance of window. This window is then
    /// finally called by [`cast_spell`](crate::cast_spell) event loop.
    ///
    /// # Panics
    ///
    /// This function needs to be called "before" initialising the slint window to avoid
    /// panicing of this function.
    pub fn invoke_spell(
        name: &str,
        window_conf: WindowConf,
        // current_display_specs: (usize, usize, usize, usize),
    ) -> Self {
        // Initialisation of wayland components.
        let conn = Connection::connect_to_env().unwrap();
        SpellWin::create_window(&conn, window_conf.clone(), name.to_string(), true)
    }

    /// Hides the layer (aka the widget) if it is visible in screen.
    pub fn hide(&self) {
        self.is_hidden.set(true);
        self.layer.wl_surface().attach(None, 0, 0);
    }

    /// Brings back the layer (aka the widget) back on screen if it is hidden.
    #[tracing::instrument]
    pub fn show_again(&mut self) {
        let primary_buf = self.adapter.refersh_buffer();
        self.buffer = primary_buf;
        self.set_config_internal();
        self.is_hidden.set(false);
        self.layer.commit();
    }

    /// Hides the widget if visible or shows the widget back if hidden.
    pub fn toggle(&mut self) {
        if self.is_hidden.get() {
            self.show_again();
        } else {
            self.hide();
        }
    }

    /// This function adds specific rectangular regions of your complete layer to receive
    /// input events from pointer and/or touch. The coordinates are in surface local
    /// format from top left corener. By default, The whole layer is considered for input
    /// events. Adding existing areas again as input region has no effect. This function
    /// combined with transparent base widgets can be used to mimic resizable widgets.
    pub fn add_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.input_region.add(x, y, width, height);
        self.set_config_internal();
        self.layer.commit();
    }

    /// This function subtracts specific rectangular regions of your complete layer from receiving
    /// input events from pointer and/or touch. The coordinates are in surface local
    /// format from top left corener. By default, The whole layer is considered for input
    /// events. Substracting input areas which are already not input regions has no effect.
    pub fn subtract_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.input_region.subtract(x, y, width, height);
        self.set_config_internal();
        self.layer.commit();
    }

    /// This function marks specific rectangular regions of your complete layer as opaque.
    /// This can result in specific optimisations from your wayland compositor, setting
    /// this property is optional. The coordinates are in surface local format from top
    /// left corener. Not adding opaque regions in it has no isuues but adding transparent
    /// regions of layer as opaque can cause weird behaviour and glitches.
    pub fn add_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.opaque_region.add(x, y, width, height);
        self.set_config_internal();
        self.layer.commit();
    }

    /// This function removes specific rectangular regions of your complete layer from being opaque.
    /// This can result in specific optimisations from your wayland compositor, setting
    /// this property is optional. The coordinates are in surface local format from top
    /// left corener.
    pub fn subtract_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.opaque_region.subtract(x, y, width, height);
        self.set_config_internal();
        self.layer.commit();
    }

    fn set_config_internal(&self) {
        set_config(
            &self.config,
            &self.layer,
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

            let buffer = &self.buffer;
            if self.first_configure || redraw_val {
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
                    .attach(Some(buffer.wl_buffer()), 0, 0);
            }
        } else {
            // println!("Is_hidden is true.");
        }

        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());
        self.layer.commit();
    }

    /// Grabs the focus of keyboard. Can be used in combination with other functions
    /// to make the widgets keyboard navigable.
    pub fn grab_focus(&self) {
        self.config
            .board_interactivity
            .set(KeyboardInteractivity::Exclusive);
        self.layer
            .set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        self.layer.commit();
    }

    /// Removes the focus of keyboard from window if it currently has it.
    pub fn remove_focus(&self) {
        self.config
            .board_interactivity
            .set(KeyboardInteractivity::None);
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
        &mut self.states.shm
    }
}

impl OutputHandler for SpellWin {
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
        // println!("Config event is called");
        if !self.first_configure {
            self.first_configure = true;
        } else {
            // println!("[{}]: First draw called", self.layer_name);
        }
        self.converter(qh);
    }
}

/// SpellLock is a struct which represents a window lock. It can be run and initialised
/// on a custom lockscreen implementation with slint.
/// <div class="warning">
/// Remember, with great power comes great responsibility. The struct doen't implement
/// pointer events so you would need to make sure that your lock screen has a text input field
/// and it is in focus on startup. Also spell doesn't add any
/// restrictions on what you can have in your lockscreen so that is a bonus
/// </div>
/// Know limitations include the abscence to verify from fingerprints and unideal issues on
/// multi-monitor setup. You can add the path of binary of your lock in your compositor config and idle
/// manager config to use the program. It will be linked to spell-cli directly in coming releases.
///
/// ## Example
/// Here is a minimal example.
///
/// ```rust
/// use std::{env, error::Error, time::Duration};
///
/// use slint::ComponentHandle;
/// use spell_framework::{
///     layer_properties::{TimeoutAction, Timer},
///     wayland_adapter::{run_lock, SpellLock, SpellSlintLock},
/// };
/// slint::include_modules!();
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let (mut lock, event_loop, event_queue, handle) = SpellLock::invoke_lock_spell();
///     let lock_ui = LockScreen::new().unwrap();
///     let looop_handle = event_loop.handle().clone();
///     SpellSlintLock::build(&mut lock, handle);
///     lock_ui.on_check_pass({
///         let lock_handle = lock_ui.as_weak();
///         move |string_val| {
///             // let lock_handle_a = lock_handle.clone();
///             let lock_handle_a = lock_handle.clone().unwrap();
///             looop_handle
///                 .insert_source(
///                     Timer::from_duration(Duration::from_secs(5)),
///                     move |_, _, app_data| {
///                         if app_data.unlock(None, string_val.as_str()).is_err() {
///                             lock_handle_a.set_lock_error(true);
///                         }
///                         TimeoutAction::Drop
///                     },
///                 )
///                 .unwrap();
///         }
///     });
///
///     run_lock(lock, event_loop, event_queue)
/// }
/// ```
pub struct SpellLock {
    pub(crate) loop_handle: LoopHandle<'static, SpellLock>,
    pub(crate) conn: Connection,
    pub(crate) compositor_state: CompositorState,
    pub(crate) registry_state: RegistryState,
    pub(crate) output_state: OutputState,
    pub(crate) keyboard_state: KeyboardState,
    pub(crate) seat_state: SeatState,
    pub(crate) shm: Shm,
    pub(crate) session_lock_state: SessionLockState,
    pub(crate) session_lock: Option<SessionLock>,
    pub(crate) lock_surfaces: Vec<SessionLockSurface>,
    pub(crate) slint_part: Option<SpellSlintLock>,
    pub(crate) is_locked: bool,
}

impl SpellLock {
    pub fn invoke_lock_spell() -> (Self, EventLoop<'static, SpellLock>, EventQueue<SpellLock>) {
        let conn = Connection::connect_to_env().unwrap();

        let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
        let qh: QueueHandle<SpellLock> = event_queue.handle();
        let registry_state = RegistryState::new(&globals);
        let shm = Shm::bind(&globals, &qh).unwrap();
        let loop_handle: EventLoop<'static, SpellLock> =
            EventLoop::try_new().expect("Failed to initialize the event loop!");
        let output_state = OutputState::new(&globals, &qh);
        let session_lock_state = SessionLockState::new(&globals, &qh);
        let compositor_state =
            CompositorState::bind(&globals, &qh).expect("Faild to create compositor state");
        let mut win_handler_vec: Vec<(String, (u32, u32))> = Vec::new();
        let lock_surfaces = Vec::new();

        let session_lock = Some(
            session_lock_state
                .lock(&qh)
                .expect("ext-session-lock not supported"),
        );

        let keyboard_state = KeyboardState {
            board: None,
            board_data: None,
        };
        let mut spell_lock = SpellLock {
            loop_handle: loop_handle.handle(),
            conn: conn.clone(),
            compositor_state,
            output_state,
            keyboard_state,
            registry_state,
            seat_state: SeatState::new(&globals, &qh),
            slint_part: None,
            shm,
            session_lock_state,
            session_lock,
            lock_surfaces,
            is_locked: true,
        };

        let _ = event_queue.roundtrip(&mut spell_lock);

        for output in spell_lock.output_state.outputs() {
            let output_info: output::OutputInfo = spell_lock.output_state.info(&output).unwrap();
            let output_name: String = output_info.name.unwrap_or_else(|| "SomeOutput".to_string());
            let dimensions = (
                output_info.logical_size.unwrap().0 as u32,
                output_info.logical_size.unwrap().1 as u32,
            );
            win_handler_vec.push((output_name, dimensions));

            let session_lock = spell_lock.session_lock.as_ref().unwrap();
            let surface = spell_lock.compositor_state.create_surface(&qh);

            // It's important to keep the `SessionLockSurface` returned here around, as the
            // surface will be destroyed when the `SessionLockSurface` is dropped.
            let lock_surface = session_lock.create_lock_surface(surface, &output, &qh);
            spell_lock.lock_surfaces.push(lock_surface);
        }
        let multi_handler = SpellMultiWinHandler::new_lock(win_handler_vec);
        let sizes: Vec<PhysicalSize> = multi_handler
            .borrow()
            .windows
            .iter()
            .map(|(_, conf)| {
                if let LayerConf::Lock(width, height) = conf {
                    PhysicalSize {
                        width: *width,
                        height: *height,
                    }
                } else {
                    panic!("Shouldn't enter here");
                }
            })
            .collect();

        let mut pool = SlotPool::new(
            (sizes[0].width * sizes[0].height * 4) as usize,
            &spell_lock.shm,
        )
        .expect("Couldn't create pool");
        let mut buffer_slots: Vec<RefCell<Slot>> = Vec::new();
        let buffers: Vec<Buffer> = sizes
            .iter()
            .map(|physical_size| {
                let stride = physical_size.width as i32 * 4;
                let (wayland_buffer, _) = pool
                    .create_buffer(
                        physical_size.width as i32,
                        physical_size.height as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    )
                    .expect("Creating Buffer");
                buffer_slots.push(RefCell::new(wayland_buffer.slot()));
                wayland_buffer
            })
            .collect();

        let pool: Rc<RefCell<SlotPool>> = Rc::new(RefCell::new(pool));
        let mut adapters: Vec<Rc<SpellSkiaWinAdapter>> = Vec::new();
        buffer_slots
            .into_iter()
            .enumerate()
            .for_each(|(index, slot)| {
                let adapter = SpellSkiaWinAdapter::new(
                    pool.clone(),
                    slot,
                    sizes[index].width,
                    sizes[index].height,
                );
                adapters.push(adapter);
            });

        multi_handler.borrow_mut().adapter = adapters.clone();
        spell_lock.slint_part = Some(SpellSlintLock {
            adapters,
            size: sizes,
            wayland_buffer: buffers,
        });

        let _ = slint::platform::set_platform(Box::new(SpellLockShell {
            window_manager: multi_handler,
        }));

        (spell_lock, loop_handle, event_queue)
    }

    fn converter_lock(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.slint_part.as_ref().unwrap().size[0].width;
        let height: u32 = self.slint_part.as_ref().unwrap().size[0].height;
        let window_adapter = self.slint_part.as_ref().unwrap().adapters[0].clone();

        // Rendering from Skia
        // if self.is_locked {
        // let skia_now = std::time::Instant::now();
        let redraw_val: bool = window_adapter.draw_if_needed();
        // println!("Skia Elapsed Time: {}", skia_now.elapsed().as_millis());

        let buffer = &self.slint_part.as_ref().unwrap().wayland_buffer[0];
        // Damage the entire window
        // if self.first_configure {
        // self.first_configure = false;
        self.lock_surfaces[0]
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
        self.lock_surfaces[0]
            .wl_surface()
            .frame(qh, self.lock_surfaces[0].wl_surface().clone());
        self.lock_surfaces[0]
            .wl_surface()
            .attach(Some(buffer.wl_buffer()), 0, 0);
        // } else {
        // println!("Is_hidden is true.");
        // }

        self.lock_surfaces[0].wl_surface().commit();
        // core::mem::swap::<&mut [u8]>(&mut sec_canvas_data.as_mut_slice(), &mut primary_canvas);
        // core::mem::swap::<&mut [Rgba8Pixel]>( &mut &mut *work_buffer, &mut &mut *currently_displayed_buffer,);

        // TODO save and reuse buffer when the window size is unchanged.  This is especially
        // useful if you do damage tracking, since you don't need to redraw the undamaged parts
        // of the canvas.
    }
    /// call this method to unlock Spelllock
    pub fn unlock(&mut self, username: Option<&str>, password: &str) -> PamResult<()> {
        let user_name;
        if let Some(username) = username {
            user_name = username.to_string();
        } else {
            let output = Command::new("sh")
                .arg("-c")
                .arg("last | awk '{print $1}' | sort | uniq -c | sort -nr")
                .output()
                .expect("Couldn't retrive username");

            let val = String::from_utf8_lossy(&output.stdout);
            let val_2 = val.split('\n').collect::<Vec<_>>()[0].trim();
            user_name = val_2.split(" ").collect::<Vec<_>>()[1].to_string();
        }

        let user_pass = UsernamePassConvo {
            username: user_name.clone(),
            password: password.into(),
        };

        let mut txn = TransactionBuilder::new_with_service("login")
            .username(user_name)
            .build(user_pass.into_conversation())?;
        // If authentication fails, this will return an error.
        // We immediately give up rather than re-prompting the user.
        txn.authenticate(AuthnFlags::empty())?;
        txn.account_management(AuthnFlags::empty())?;

        if let Some(locked_val) = self.session_lock.take() {
            locked_val.unlock();
        }
        self.is_locked = false;
        self.conn.roundtrip().unwrap();
        Ok(())
    }
}

delegate_keyboard!(SpellLock);
delegate_compositor!(SpellLock);
delegate_output!(SpellLock);
delegate_shm!(SpellLock);
delegate_registry!(SpellLock);
delegate_session_lock!(SpellLock);
delegate_seat!(SpellLock);
// TODO have to add no auth allowed after 3 consecutive wrong attempts feature.

/// A basic Conversation that assumes that any "regular" prompt is for
/// the username, and that any "masked" prompt is for the password.
///
/// A typical Conversation will provide the user with an interface
/// to interact with PAM, e.g. a dialogue box or a terminal prompt.
struct UsernamePassConvo {
    username: String,
    password: String,
}

// ConversationAdapter is a convenience wrapper for the common case
// of only handling one request at a time.
impl ConversationAdapter for UsernamePassConvo {
    fn prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(OsString::from(&self.username))
    }

    fn masked_prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(OsString::from(&self.password))
    }

    fn error_msg(&self, _message: impl AsRef<OsStr>) {
        // Normally you would want to display this to the user somehow.
        // In this case, we're just ignoring it.
    }

    fn info_msg(&self, _message: impl AsRef<OsStr>) {
        // ibid.
    }
}

/// Furture virtual keyboard implementation will be on this type. Currently, it is redundent.
pub struct SpellBoard;
