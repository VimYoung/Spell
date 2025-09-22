//! It provides various widget types for implementing properties
//! across various functionalities for your shell. The most common widget (or
//! window as called by many) is [SpellWin]. You can also implement a lock screen
//! with [`SpellLock`].
use crate::{
    SpellAssociated,
    configure::{LayerConf, WindowConf},
    dbus_window_state::InternalHandle,
    helper_fn_for_deploy,
    slint_adapter::{SpellLayerShell, SpellLockShell, SpellMultiWinHandler, SpellSkiaWinAdapter},
    wayland_adapter::way_helper::{KeyboardState, PointerState, set_config},
};
pub use lock_impl::SpellSlintLock;
use nonstick::{
    AuthnFlags, ConversationAdapter, Result as PamResult, Transaction, TransactionBuilder,
};
use slint::{PhysicalSize, platform::Key};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_session_lock, delegate_shm,
    output::{self, OutputHandler, OutputState},
    reexports::{
        calloop::{
            self, EventLoop, LoopHandle, RegistrationToken,
            channel::Event,
            timer::{TimeoutAction, Timer},
        },
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, QueueHandle,
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
use std::{
    cell::{Cell, RefCell},
    process::Command,
    rc::Rc,
    time::Duration,
};
use tracing::{Level, debug, field, info, span, trace};
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
/// The constructor method [conjure_spells](crate::wayland_adapter::SpellMultiWinHandler::conjure_spells) will
/// panic if the number of WindowConfs provided is not equal to the amount of widgets that are
/// initialised in the scope. The solution to avoid panic is to add more `let _ =
/// WidgetName::new().unwrap();` for all the widgets/window components you are declaring in your
/// slint files and adding [WindowConf]s for in [SpellMultiWinHandler].
pub struct SpellWin {
    pub(crate) adapter: Rc<SpellSkiaWinAdapter>,
    pub(crate) loop_handle: LoopHandle<'static, SpellWin>,
    pub(crate) queue: QueueHandle<SpellWin>,
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
    pub(crate) event_loop: Rc<RefCell<EventLoop<'static, SpellWin>>>,
    pub(crate) span: span::Span,
    pub(crate) backspace: calloop::RegistrationToken,
}

impl std::fmt::Debug for SpellWin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpellWin")
            .field("adapter", &self.adapter)
            .field("first_configure", &self.first_configure)
            .field("is_hidden", &self.is_hidden)
            .field("config", &self.config)
            .finish()
    }
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
        let event_loop: EventLoop<'static, SpellWin> =
            EventLoop::try_new().expect("Failed to initialize the event loop!");
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
            // board_data: None,
        };

        let adapter_value: Rc<SpellSkiaWinAdapter> = SpellSkiaWinAdapter::new(
            Rc::new(RefCell::new(pool)),
            RefCell::new(primary_slot),
            window_conf.width,
            window_conf.height,
        );

        if if_single {
            trace!("Single window layer platform set");
            let _ = slint::platform::set_platform(Box::new(SpellLayerShell {
                window_adapter: adapter_value.clone(),
            }));
        }

        let backspace_event = event_loop
            .handle()
            .insert_source(
                Timer::from_duration(Duration::from_millis(300)),
                |_, _, data| {
                    data.adapter
                        .try_dispatch_event(slint::platform::WindowEvent::KeyPressed {
                            text: Key::Backspace.into(),
                        })
                        .unwrap();
                    TimeoutAction::ToDuration(Duration::from_millis(300))
                },
            )
            .unwrap();
        event_loop.handle().disable(&backspace_event).unwrap();

        let win = SpellWin {
            adapter: adapter_value,
            loop_handle: event_loop.handle(),
            queue: qh.clone(),
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
            layer_name: layer_name.clone(),
            config: window_conf,
            input_region,
            opaque_region,
            event_loop: Rc::new(RefCell::new(event_loop)),
            span: span!(
                Level::INFO,
                "widget",
                name = layer_name.as_str(),
                zbus = field::Empty
            ),
            backspace: backspace_event,
        };

        info!("Win: {} layer created successfully.", layer_name);

        WaylandSource::new(conn.clone(), event_queue)
            .insert(win.loop_handle.clone())
            .unwrap();
        win
    }

    /// Returns a handle of [`WinHandle`] to invoke wayland specific features.
    pub fn get_handler(&mut self) -> WinHandle {
        info!("Win: Handle provided.");
        WinHandle(self.loop_handle.clone())
    }

    /// This function is called to create a instance of window. This window is then
    /// finally called by [`cast_spell`](crate::cast_spell) event loop.
    ///
    /// # Panics
    ///
    /// This function needs to be called "before" initialising the slint window to avoid
    /// panicing of this function.
    pub fn invoke_spell(name: &str, window_conf: WindowConf) -> Self {
        tracing_subscriber::fmt()
            .without_time()
            .with_target(false)
            .with_env_filter(tracing_subscriber::EnvFilter::new(
                "spell_framework=trace,info",
            ))
            // .with_max_level(tracing::Level::TRACE)
            .init();
        let conn = Connection::connect_to_env().unwrap();
        SpellWin::create_window(&conn, window_conf.clone(), name.to_string(), true)
    }

    /// Hides the layer (aka the widget) if it is visible in screen.
    pub fn hide(&self) {
        if !self.is_hidden.replace(true) {
            info!("Win: Hiding window");
            self.layer.wl_surface().attach(None, 0, 0);
        }
    }

    /// Brings back the layer (aka the widget) back on screen if it is hidden.
    pub fn show_again(&mut self) {
        if self.is_hidden.replace(false) {
            info!("Win: Showing window again");
            let qh = self.queue.clone();
            self.converter(&qh);
            // let primary_buf = self.adapter.refersh_buffer();
            // self.buffer = primary_buf;
            // self.layer.commit();
            // self.set_config_internal();
            // self.layer
            //     .wl_surface()
            //     .attach(Some(self.buffer.wl_buffer()), 0, 0);
            // self.layer.commit();
        }
    }

    /// Hides the widget if visible or shows the widget back if hidden.
    pub fn toggle(&mut self) {
        info!("Win: view toggled");
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
        info!(
            "Win: input region added: [x: {}, y: {}, width: {}, height: {}]",
            x, y, width, height
        );
        self.input_region.add(x, y, width, height);
        self.set_config_internal();
        self.layer.commit();
    }

    /// This function subtracts specific rectangular regions of your complete layer from receiving
    /// input events from pointer and/or touch. The coordinates are in surface local
    /// format from top left corener. By default, The whole layer is considered for input
    /// events. Substracting input areas which are already not input regions has no effect.
    pub fn subtract_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        info!(
            "Win: input region removed: [x: {}, y: {}, width: {}, height: {}]",
            x, y, width, height
        );
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
        info!(
            "Win: opaque region added: [x: {}, y: {}, width: {}, height: {}]",
            x, y, width, height
        );
        self.opaque_region.add(x, y, width, height);
        self.set_config_internal();
        self.layer.commit();
    }

    /// This function removes specific rectangular regions of your complete layer from being opaque.
    /// This can result in specific optimisations from your wayland compositor, setting
    /// this property is optional. The coordinates are in surface local format from top
    /// left corener.
    pub fn subtract_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        info!(
            "Win: opaque region removed: [x: {}, y: {}, width: {}, height: {}]",
            x, y, width, height
        );
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
            let skia_now = std::time::Instant::now();
            let redraw_val: bool = window_adapter.draw_if_needed();
            let elasped_time = skia_now.elapsed().as_millis();
            if elasped_time != 0 {
                // debug!("Skia Elapsed Time: {}", skia_now.elapsed().as_millis());
            }

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
            // debug!("Is hidden is true, window is true");
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

    /// This method is used to set exclusive zone. Generally, useful when
    /// dimensions of width are different than exclusive zone you want.
    pub fn set_exclusive_zone(&self, val: i32) {
        self.layer.set_exclusive_zone(val);
        self.layer.commit();
    }
}

impl SpellAssociated for SpellWin {
    fn on_call<F>(
        &mut self,
        state: Option<
            std::sync::Arc<std::sync::RwLock<Box<dyn crate::layer_properties::ForeignController>>>,
        >,
        set_callback: Option<F>,
        span_log: tracing::span::Span,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(
                std::sync::Arc<
                    std::sync::RwLock<Box<dyn crate::layer_properties::ForeignController>>,
                >,
            ) + 'static,
    {
        let rx = helper_fn_for_deploy(self.layer_name.clone(), &state, span_log);
        let event_loop = self.event_loop.clone();
        if let Some(mut callback) = set_callback {
            self.event_loop
                .borrow()
                .handle()
                .insert_source(rx, move |event_msg, _, state_data| {
                    trace!("Internal event recieved");
                    match event_msg {
                        Event::Msg(int_handle) => match int_handle {
                            InternalHandle::StateValChange((key, data_type)) => {
                                trace!("Internal variable change called");
                                {
                                    let mut state_inst = state.as_ref().unwrap().write().unwrap();
                                    state_inst.change_val(&key, data_type);
                                }
                                callback(state.as_ref().unwrap().clone());
                            }
                            InternalHandle::ShowWinAgain => {
                                trace!("Internal show Called");
                                state_data.show_again();
                            }
                            InternalHandle::HideWindow => {
                                trace!("Internal hide called");
                                state_data.hide();
                            }
                        },
                        // TODO have to handle it properly.
                        Event::Closed => {
                            info!("Internal Channel closed");
                        }
                    }
                })
                .unwrap();
        }
        info!("Internal reciever set with start of event loop.");
        loop {
            event_loop
                .borrow_mut()
                .dispatch(std::time::Duration::from_millis(1), self)
                .unwrap();
        }
    }

    fn get_span(&self) -> tracing::span::Span {
        self.span.clone()
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
        trace!("New output Source Added");
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        trace!("Existing output is updated");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        trace!("Output is destroyed");
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
        trace!("Scale factor changed");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        trace!("Compositor transformation changed");
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
        trace!("Surface entered");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        trace!("Surface left");
    }
}

impl LayerShellHandler for SpellWin {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        trace!("Closure of layer called");
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
            // debug!("[{}]: First draw called", self.layer_name);
            self.first_configure = true;
        } else {
            // debug!("[{}]: First draw called", self.layer_name);
        }
        self.converter(qh);
    }
}

/// This is a wrapper around calloop's [loop_handle](https://docs.rs/calloop/latest/calloop/struct.LoopHandle.html)
/// for calling wayland specific features of `SpellWin`. It can be accessed from
/// [`crate::wayland_adapter::SpellWin::get_handler`].
#[derive(Clone, Debug)]
pub struct WinHandle(pub(crate) LoopHandle<'static, SpellWin>);

impl WinHandle {
    /// Internally calls [`crate::wayland_adapter::SpellWin::hide`]
    pub fn hide(&self) {
        self.0.insert_idle(|win| win.hide());
    }

    /// Internally calls [`crate::wayland_adapter::SpellWin::show_again`]
    pub fn show_again(&self) {
        self.0.insert_idle(|win| win.show_again());
    }

    /// Internally calls [`crate::wayland_adapter::SpellWin::toggle`]
    pub fn toggle(&self) {
        self.0.insert_idle(|win| win.toggle());
    }

    /// Internally calls [`crate::wayland_adapter::SpellWin::grab_focus`]
    pub fn grab_focus(&self) {
        self.0.insert_idle(|win| win.grab_focus());
    }

    /// Internally calls [`crate::wayland_adapter::SpellWin::remove_focus`]
    pub fn remove_focus(&self) {
        self.0.insert_idle(|win| win.remove_focus());
    }

    /// Internally calls [`crate::wayland_adapter::SpellWin::add_input_region`]
    pub fn add_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.0
            .insert_idle(move |win| win.add_input_region(x, y, width, height));
    }

    /// Internally calls [`crate::wayland_adapter::SpellWin::subtract_input_region`]
    pub fn subtract_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.0
            .insert_idle(move |win| win.subtract_input_region(x, y, width, height));
    }

    /// Internally calls [`crate::wayland_adapter::SpellWin::add_opaque_region`]
    pub fn add_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.0
            .insert_idle(move |win| win.add_opaque_region(x, y, width, height));
    }

    /// Internally calls [`crate::wayland_adapter::SpellWin::subtract_opaque_region`]
    pub fn subtract_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
        self.0
            .insert_idle(move |win| win.subtract_opaque_region(x, y, width, height));
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
/// Here is a minimal example of rust side, for complete code of slint, check
/// the codebase of young-shell.
///
/// ```rust
/// use spell_framework::cast_spell;
/// use std::{error::Error, sync::{Arc, RwLock}};
/// use slint::ComponentHandle;
/// use spell_framework::{layer_properties::ForeignController, wayland_adapter::SpellLock};
/// slint::include_modules!();
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let lock = SpellLock::invoke_lock_spell();
///     let lock_ui = LockScreen::new().unwrap();
///     let looop_handle = lock.get_handler();
///     lock_ui.on_check_pass({
///         let lock_handle = lock_ui.as_weak();
///         move |string_val| {
///             let lock_handle_a = lock_handle.clone().unwrap();
///             let lock_handle_b = lock_handle.clone().unwrap();
///             looop_handle.unlock(
///                 None,
///                 string_val.to_string(),
///                 Box::new(move || {
///                     lock_handle_a.set_lock_error(true);
///                 }),
///                 Box::new(move || {
///                     lock_handle_b.set_is_lock_activated(false);
///                 }),
///             );
///         }
///     });
///     lock_ui.set_is_lock_activated(true);
///     cast_spell(
///         lock,
///         None,
///         None::<fn(Arc<RwLock<Box<dyn ForeignController>>>)>,
///     )
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
    pub(crate) session_lock: Option<SessionLock>,
    pub(crate) lock_surfaces: Vec<SessionLockSurface>,
    pub(crate) slint_part: Option<SpellSlintLock>,
    pub(crate) is_locked: bool,
    pub(crate) event_loop: Rc<RefCell<EventLoop<'static, SpellLock>>>,
    pub(crate) backspace: Option<RegistrationToken>,
}

impl std::fmt::Debug for SpellLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpellLock")
            .field("is_locked", &self.is_locked)
            .finish()
    }
}
impl SpellLock {
    pub fn invoke_lock_spell() -> Self {
        let conn = Connection::connect_to_env().unwrap();

        let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
        let qh: QueueHandle<SpellLock> = event_queue.handle();
        let registry_state = RegistryState::new(&globals);
        let shm = Shm::bind(&globals, &qh).unwrap();
        let event_loop: EventLoop<'static, SpellLock> =
            EventLoop::try_new().expect("Failed to initialize the event loop!");
        let output_state = OutputState::new(&globals, &qh);
        let session_lock_state = SessionLockState::new(&globals, &qh);
        let compositor_state =
            CompositorState::bind(&globals, &qh).expect("Faild to create compositor state");
        let mut win_handler_vec: Vec<(String, (u32, u32))> = Vec::new();
        let lock_surfaces = Vec::new();

        let keyboard_state = KeyboardState {
            board: None,
            // board_data: None,
        };
        let mut spell_lock = SpellLock {
            loop_handle: event_loop.handle().clone(),
            conn: conn.clone(),
            compositor_state,
            output_state,
            keyboard_state,
            registry_state,
            seat_state: SeatState::new(&globals, &qh),
            slint_part: None,
            shm,
            session_lock: None,
            lock_surfaces,
            is_locked: true,
            event_loop: Rc::new(RefCell::new(event_loop)),
            backspace: None,
        };

        let _ = event_queue.roundtrip(&mut spell_lock);

        let session_lock = Some(
            session_lock_state
                .lock(&qh)
                .expect("ext-session-lock not supported"),
        );

        spell_lock.session_lock = session_lock;
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

        WaylandSource::new(spell_lock.conn.clone(), event_queue)
            .insert(spell_lock.loop_handle.clone())
            .unwrap();
        spell_lock
    }

    fn converter_lock(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.slint_part.as_ref().unwrap().size[0].width;
        let height: u32 = self.slint_part.as_ref().unwrap().size[0].height;
        let window_adapter = self.slint_part.as_ref().unwrap().adapters[0].clone();

        // Rendering from Skia
        // if self.is_locked {
        // let skia_now = std::time::Instant::now();
        let _redraw_val: bool = window_adapter.draw_if_needed();
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

    fn unlock(
        &mut self,
        username: Option<&str>,
        password: &str,
        on_unlock_callback: Box<dyn FnOnce()>,
    ) -> PamResult<()> {
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

        on_unlock_callback();
        if let Some(locked_val) = self.session_lock.take() {
            locked_val.unlock();
        }
        self.is_locked = false;
        self.conn.roundtrip().unwrap();
        Ok(())
    }

    pub fn get_handler(&self) -> LockHandle {
        LockHandle(self.loop_handle.clone())
    }
}

impl SpellAssociated for SpellLock {
    fn on_call<F>(
        &mut self,
        _: Option<
            std::sync::Arc<std::sync::RwLock<Box<dyn crate::layer_properties::ForeignController>>>,
        >,
        _: Option<F>,
        _: tracing::span::Span,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(
                std::sync::Arc<
                    std::sync::RwLock<Box<dyn crate::layer_properties::ForeignController>>,
                >,
            ) + 'static,
    {
        let event_loop = self.event_loop.clone();
        while self.is_locked {
            event_loop
                .borrow_mut()
                .dispatch(std::time::Duration::from_millis(1), self)
                .unwrap();
        }
        Ok(())
    }

    fn get_span(&self) -> tracing::span::Span {
        span!(Level::INFO, "widget")
    }
}

/// Struct to handle unlocking of a SpellLock instance.
pub struct LockHandle(LoopHandle<'static, SpellLock>);

impl LockHandle {
    /// Call this method to unlock Spelllock. It also takes two callbacks which
    /// are invoked when the password parsed is wrong and when the lock is
    /// opened respectively. It can be used to invoke UI specific changes for
    /// your slint frontend.
    pub fn unlock(
        &self,
        username: Option<String>,
        password: String,
        on_err_callback: Box<dyn FnOnce()>,
        on_unlock_callback: Box<dyn FnOnce()>,
    ) {
        self.0.insert_idle(move |app_data| {
            if app_data
                .unlock(username.as_deref(), &password, on_unlock_callback)
                .is_err()
            {
                on_err_callback();
            }
        });
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
    fn prompt(&self, _request: impl AsRef<std::ffi::OsStr>) -> PamResult<std::ffi::OsString> {
        Ok(std::ffi::OsString::from(&self.username))
    }

    fn masked_prompt(
        &self,
        _request: impl AsRef<std::ffi::OsStr>,
    ) -> PamResult<std::ffi::OsString> {
        Ok(std::ffi::OsString::from(&self.password))
    }

    fn error_msg(&self, _message: impl AsRef<std::ffi::OsStr>) {}

    fn info_msg(&self, _message: impl AsRef<std::ffi::OsStr>) {}
}

/// Furture virtual keyboard implementation will be on this type. Currently, it is redundent.
pub struct SpellBoard;
