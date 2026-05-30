//! It provides various widget types for implementing properties
//! across various functionalities for your shell. The most common widget (or
//! window as called by many) is [SpellWin]. You can also implement a lock screen
//! with [`SpellLock`].
use crate::{
    SpellAssociatedNew,
    configure::{Dimension, HomeHandle, LayerConf, WindowConf, set_up_tracing},
    slint_adapter::{
        ADAPTERS, SpellLayerShell, SpellLockShell, SpellMultiWinHandler, SpellSkiaWinAdapter,
    },
    wayland_adapter::{
        fractional_scaling::{
            FractionalScaleHandler, FractionalScaleState, delegate_fractional_scale,
        },
        viewporter::{Viewport, ViewporterState, delegate_viewporter},
        way_helper::{
            FingerprintInfo, PointerState, UsernamePassConvo, set_config, set_event_sources,
        },
    },
};
use i_slint_core::items::MouseCursor;
pub use lock_impl::SpellSlintLock;
use nonstick::{
    AuthnFlags, ConversationAdapter, Result as PamResult, Transaction, TransactionBuilder,
};
use slint::{
    PhysicalSize,
    platform::{Key, WindowAdapter},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_session_lock, delegate_shm, delegate_touch,
    output::{self, OutputHandler, OutputState},
    reexports::{
        calloop::{
            self, EventLoop, LoopHandle, RegistrationToken,
            channel::{self, Sender},
            timer::{TimeoutAction, Timer},
        },
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, EventQueue, QueueHandle,
            globals::registry_queue_init,
            protocol::{
                wl_keyboard::WlKeyboard,
                wl_output::{self, WlOutput},
                wl_shm, wl_surface,
                wl_touch::WlTouch,
            },
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
    collections::HashMap,
    os::unix::net::UnixListener,
    process::Command,
    rc::Rc,
    sync::{Once, OnceLock, RwLock},
};
use tracing::{Level, info, span, trace, warn};

mod fractional_scaling;
mod lock_impl;
mod slint_to_wl_cursor_mapping;
mod viewporter;
mod way_helper;
mod win_impl;

static AVAILABLE_MONITORS: OnceLock<RwLock<HashMap<String, (wl_output::WlOutput, i32, i32)>>> =
    OnceLock::new();
static SET_SLINT_PLATFORM: Once = Once::new();

#[derive(Debug)]
pub(crate) struct States {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) pointer_state: PointerState,
    pub(crate) keyboard_state: Option<WlKeyboard>,
    pub(crate) touch_state: Option<WlTouch>,
    pub(crate) shm: Shm,
    pub(crate) viewporter: Option<Viewport>,
}

/// `SpellWin` is the main type for implementing widgets, it covers various properties and trait
/// implementation, thus providing various features.
pub struct SpellWin {
    pub(crate) adapter: Option<Rc<SpellSkiaWinAdapter>>,
    /// loop handle provided in a wrapper by [get_handler](crate::wayland_adapter::SpellWin::get_handler).
    pub loop_handle: LoopHandle<'static, SpellWin>,
    /// UnixListener storing remote instructions from CLI.
    pub ipc_handler: Option<UnixListener>,
    // pub(crate) queue: QueueHandle<SpellWin>,
    pub(crate) buffer: Option<Buffer>,
    pub(crate) states: States,
    pub(crate) layer: Option<LayerSurface>,
    pub(crate) first_configure: Cell<bool>,
    pub(crate) natural_scroll: bool,
    pub(crate) is_hidden: Cell<bool>,
    /// Name of widget's layer.
    pub layer_name: String,
    pub(crate) config: WindowConf,
    pub(crate) input_region: Region,
    pub(crate) opaque_region: Region,
    /// Event loop which runs and refreshes UI.
    pub event_loop: Rc<RefCell<EventLoop<'static, SpellWin>>>,
    /// Span required for proper logging.
    pub span: span::Span,
    // #[allow(dead_code)]
    // pub(crate) backspace: calloop::RegistrationToken,
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
        mut window_conf: WindowConf,
        layer_name: String,
        handle: HomeHandle,
    ) -> Self {
        let (globals, mut event_queue) = registry_queue_init(conn).unwrap();
        let qh: QueueHandle<SpellWin> = event_queue.handle();
        let compositor =
            CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");
        let event_loop: EventLoop<'static, SpellWin> =
            EventLoop::try_new().expect("Failed to initialize the event loop!");
        let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");
        let cursor_manager =
            CursorShapeManager::bind(&globals, &qh).expect("cursor shape is not available");
        let fractional_scale_state: FractionalScaleState =
            FractionalScaleState::bind(&globals, &qh).expect("Fractional Scale couldn't be set");
        let surface = compositor.create_surface(&qh);
        let viewporter_state =
            ViewporterState::bind(&globals, &qh).expect("Couldn't set viewporter");
        let pointer_state = PointerState {
            pointer: None,
            pointer_data: None,
            cursor_shape: cursor_manager,
            current_wayland_cursor: MouseCursor::Default,
            last_cursor_enter_serial: None,
        };
        let input_region = Region::new(&compositor).expect("Couldn't create region");
        let opaque_region = Region::new(&compositor).expect("Couldn't create opaque region");

        let mut win = SpellWin {
            adapter: None,
            loop_handle: event_loop.handle(),
            ipc_handler: None,
            // queue: qh.clone(),
            buffer: None,
            states: States {
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),
                pointer_state,
                keyboard_state: None,
                touch_state: None,
                shm,
                viewporter: None,
            },
            layer: None,
            first_configure: Cell::new(true),
            natural_scroll: window_conf.natural_scroll,
            is_hidden: Cell::new(false),
            layer_name: layer_name.clone(),
            config: window_conf.clone(),
            input_region,
            opaque_region,
            event_loop: Rc::new(RefCell::new(event_loop)),
            span: span!(Level::INFO, "widget", name = layer_name.as_str(),),
        };

        if AVAILABLE_MONITORS.get().is_none() {
            match SpellWin::get_available_monitors(&mut event_queue, &mut win) {
                Some(monitors) => {
                    let _ = AVAILABLE_MONITORS.get_or_init(|| RwLock::new(monitors));
                }
                None => warn!("Failed to get available monitors"),
            }
        }

        let mut output_info: Option<(wl_output::WlOutput, i32, i32)> =
            if let Some(name) = &window_conf.monitor_name {
                let output = AVAILABLE_MONITORS
                    .get()
                    .and_then(|monitors| monitors.read().ok())
                    .and_then(|monitors| monitors.get(name).cloned());
                if output.is_none() {
                    warn!("Monitor '{}' not found, using default monitor", name);
                }
                output
            } else {
                None
            };

        match window_conf.width {
            Dimension::Pixel(x) => window_conf.evaluated_width = x,
            Dimension::Full => {
                window_conf.evaluated_width = output_info
                    .as_ref()
                    .expect("Output info couldn't be retrieved")
                    .1 as u32
            }
            Dimension::Percentage(y) => {
                window_conf.evaluated_width = output_info
                    .as_mut()
                    .expect("Output info couldn't be retrieved")
                    .1 as u32
                    / y;
            }
        }

        match window_conf.height {
            Dimension::Pixel(x) => window_conf.evaluated_height = x,
            Dimension::Full => {
                window_conf.evaluated_height = output_info
                    .as_ref()
                    .expect("Output info couldn't be retrieved")
                    .1 as u32
            }
            Dimension::Percentage(y) => {
                window_conf.evaluated_height = output_info
                    .as_ref()
                    .expect("Output info couldn't be retrieved")
                    .1 as u32
                    / y;
            }
        }
        win.config = window_conf.clone();

        info!(
            "Evaluated width: {}, evaluated_height: {}",
            window_conf.evaluated_width, window_conf.evaluated_height
        );

        let mut pool = SlotPool::new(
            (window_conf.evaluated_width * window_conf.evaluated_height * 4) as usize,
            &win.states.shm,
        )
        .expect("Failed to create pool");
        win.input_region.add(
            0,
            0,
            window_conf.evaluated_width as i32,
            window_conf.evaluated_height as i32,
        );

        let stride = window_conf.evaluated_width as i32 * 4;
        let (way_pri_buffer, _) = pool
            .create_buffer(
                window_conf.evaluated_width as i32,
                window_conf.evaluated_height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating Buffer");

        let primary_slot = way_pri_buffer.slot();
        let adapter_value: Rc<SpellSkiaWinAdapter> = SpellSkiaWinAdapter::new(
            Rc::new(RefCell::new(pool)),
            RefCell::new(primary_slot),
            window_conf.evaluated_width,
            window_conf.evaluated_height,
        );
        win.adapter = Some(adapter_value.clone());
        win.buffer = Some(way_pri_buffer);

        let (slint_event_sender, slint_event_receiver) =
            calloop::channel::channel::<Box<dyn FnOnce() + Send>>();

        ADAPTERS.with_borrow_mut(|v| v.push(adapter_value.clone()));
        SET_SLINT_PLATFORM.call_once(|| {
            trace!("Slint platform set");
            if let Err(err) =
                slint::platform::set_platform(Box::new(SpellLayerShell::new(slint_event_sender)))
            {
                warn!("Error setting slint platform: {err}");
            }
        });
        win.adapter = Some(adapter_value);
        // set_event_sources(&event_loop, handle, slint_event_receiver);
        // ADAPTERS.with_borrow_mut(|v| v.push(adapter_value));
        // SET_SLINT_PLATFORM.call_once(|| {
        //     trace!("Slint platform set");
        //     if let Err(err) = slint::platform::set_platform(Box::new(SpellLayerShell::default())) {
        //         warn!("Error setting slint platform: {err}");
        //     }
        // });
        let target_output: Option<&WlOutput> = output_info.as_ref().map(|(a, _, _)| a);
        let layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            window_conf.layer_type,
            Some(layer_name.clone()),
            target_output,
        );
        // layer.commit();
        let fractional_scale = fractional_scale_state.get_scale(layer.wl_surface(), &qh);

        let viewporter = viewporter_state.get_viewport(layer.wl_surface(), &qh, fractional_scale);

        set_config(
            &win.config,
            &layer,
            //true,
            Some(win.input_region.wl_region()),
            None,
        );
        if let Err(err) = event_queue.roundtrip(&mut win) {
            warn!("Received roundtrip error: {}", err);
        }
        layer.commit();

        win.layer = Some(layer);
        win.states.viewporter = Some(viewporter);

        set_event_sources(
            &win.event_loop.as_ref().borrow(),
            handle,
            slint_event_receiver,
        );

        info!("Win: {} layer created successfully.", layer_name);

        WaylandSource::new(conn.clone(), event_queue)
            .insert(win.loop_handle.clone())
            .unwrap();
        win
    }

    /// Fetches the available monitors from the Wayland registry.
    ///
    /// This function fetches the available monitors from the Wayland registry and returns a map of
    /// the available monitors where the key is the name of the monitor and the value is the
    /// [`wl_output::WlOutput`]. It uses an already registered event queue & spell window.
    ///
    /// # Errors
    ///
    /// Returns `None` if the registry queue could not be initialized.
    fn get_available_monitors(
        event_queue: &mut EventQueue<SpellWin>,
        win: &mut SpellWin,
    ) -> Option<HashMap<String, (wl_output::WlOutput, i32, i32)>> {
        // roundtrip to get all available monitors from Wayland
        event_queue.roundtrip(win).ok()?;

        Some(
            win.states
                .output_state
                .outputs()
                .filter_map(|output| {
                    let info = win.states.output_state.info(&output)?;
                    Some((
                        info.name?,
                        (output, info.logical_size?.0, info.logical_size?.1),
                    ))
                })
                .collect(),
        )
    }

    /// Returns a handle of [`WinHandle`] to invoke wayland specific features.
    pub fn get_handler(&self) -> WinHandle {
        info!("Win: Handle provided.");
        WinHandle(self.loop_handle.clone())
    }

    /// This function is called to create a instance of window. This window is then
    /// finally called by [`cast_spell`](crate::cast_spell) event loop.
    ///
    /// # Panics
    ///
    /// This function needs to be called "before" initialising your slint window to avoid
    /// panicing of this function.
    pub fn invoke_spell(name: &str, window_conf: WindowConf) -> Self {
        let handle = set_up_tracing(name);
        let conn = Connection::connect_to_env().unwrap();
        SpellWin::create_window(&conn, window_conf.clone(), name.to_string(), handle)
    }

    /// Hides the layer (aka the widget) if it is visible in screen.
    pub fn hide(&self) {
        if !self.is_hidden.replace(true) {
            info!("Win: Hiding window");
            self.layer.as_ref().unwrap().wl_surface().attach(None, 0, 0);
        }
    }

    /// Brings back the layer (aka the widget) back on screen if it is hidden.
    pub fn show_again(&self) {
        if self.is_hidden.replace(false) {
            info!("Win: Showing window again");
            self.set_config_internal();
            self.first_configure.set(true);
            self.layer.as_ref().unwrap().commit();
        }
    }

    /// Hides the widget if visible or shows the widget back if hidden.
    pub fn toggle(&self) {
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
        self.layer.as_ref().unwrap().commit();
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
        self.layer.as_ref().unwrap().commit();
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
        self.layer.as_ref().unwrap().commit();
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
        self.layer.as_ref().unwrap().commit();
    }

    fn set_config_internal(&self) {
        set_config(
            &self.config,
            self.layer.as_ref().unwrap(),
            Some(self.input_region.wl_region()),
            Some(self.opaque_region.wl_region()),
        );
    }

    fn converter(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.adapter.as_ref().unwrap().size.get().width;
        let height: u32 = self.adapter.as_ref().unwrap().size.get().height;
        let window_adapter = self.adapter.clone();

        // Rendering from Skia
        if !self.is_hidden.get() {
            // let skia_now = std::time::Instant::now();
            let redraw_val: bool = window_adapter.unwrap().draw_if_needed();
            // let elasped_time = skia_now.elapsed().as_millis();
            // if elasped_time != 0 {
            //     debug!("Skia Elapsed Time: {}", skia_now.elapsed().as_millis());
            // }

            self.states
                .pointer_state
                .update_cursor(self.adapter.as_ref().unwrap().current_cursor.get(), &qh);

            let buffer = &self.buffer;
            if self.first_configure.get() || redraw_val {
                // if self.first_configure {
                self.first_configure.set(false);
                self.layer.as_ref().unwrap().wl_surface().damage_buffer(
                    0,
                    0,
                    width as i32,
                    height as i32,
                );
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
                self.layer.as_ref().unwrap().wl_surface().attach(
                    Some(buffer.as_ref().unwrap().wl_buffer()),
                    0,
                    0,
                );
            }

            self.layer
                .as_ref()
                .unwrap()
                .wl_surface()
                .frame(qh, self.layer.as_ref().unwrap().wl_surface().clone());
            self.layer.as_ref().unwrap().commit();
        } else {
            self.layer.as_ref().unwrap().commit();
        }
    }

    /// Grabs the focus of keyboard. Can be used in combination with other functions
    /// to make the widgets keyboard navigable.
    pub fn grab_focus(&self) {
        if !self.is_hidden.get()
            && self.config.board_interactivity.get() != KeyboardInteractivity::Exclusive
        {
            self.config
                .board_interactivity
                .set(KeyboardInteractivity::Exclusive);
            self.layer
                .as_ref()
                .unwrap()
                .set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
            self.layer.as_ref().unwrap().commit();
        }
    }

    /// Removes the focus of keyboard from window if it currently has it.
    pub fn remove_focus(&self) {
        if !self.is_hidden.get()
            && self.config.board_interactivity.get() != KeyboardInteractivity::None
        {
            self.config
                .board_interactivity
                .set(KeyboardInteractivity::None);
            self.layer
                .as_ref()
                .unwrap()
                .set_keyboard_interactivity(KeyboardInteractivity::None);
            self.layer.as_ref().unwrap().commit();
        }
    }

    /// This method is used to set exclusive zone. Generally, useful when
    /// dimensions of width are different than exclusive zone you want.
    pub fn set_exclusive_zone(&mut self, val: i32) {
        // self.set_config_internal();
        self.config.exclusive_zone = Some(val);
        self.layer.as_ref().unwrap().set_exclusive_zone(val);
        self.layer.as_ref().unwrap().commit();
    }
}

impl SpellAssociatedNew for SpellWin {
    fn on_call(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = self.event_loop.clone();
        event_loop
            .borrow_mut()
            .dispatch(std::time::Duration::from_millis(1), self)?;
        Ok(())
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
delegate_touch!(SpellWin);
delegate_layer!(SpellWin);
delegate_fractional_scale!(SpellWin);
delegate_viewporter!(SpellWin);

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
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
        info!("Scale factor changed, compositor msg");
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

impl FractionalScaleHandler for SpellWin {
    fn preferred_scale(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        scale: u32,
    ) {
        info!("Scale factor changed, invoked from custom trait. {}", scale);
        let width_old = self.adapter.as_ref().unwrap().size_original.get().width;
        let height_old = self.adapter.as_ref().unwrap().size_original.get().height;
        self.layer.as_ref().unwrap().wl_surface().damage_buffer(
            0,
            0,
            self.adapter.as_ref().unwrap().size.get().width as i32,
            self.adapter.as_ref().unwrap().size.get().height as i32,
        );
        let (buffer, width, height, scale_factor) =
            self.adapter.as_ref().unwrap().changed_scale_factor(scale);
        self.config.evaluated_width = width;
        self.config.evaluated_height = height;
        self.buffer = Some(buffer);
        self.adapter
            .as_ref()
            .unwrap()
            .try_dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged { scale_factor })
            .unwrap();
        self.states.viewporter.as_ref().unwrap().set_source(
            0.,
            0.,
            self.adapter.as_ref().unwrap().size.get().width.into(),
            self.adapter.as_ref().unwrap().size.get().height.into(),
        );

        self.states
            .viewporter
            .as_ref()
            .unwrap()
            .set_destination(width_old as i32, height_old as i32);
        self.adapter.as_ref().unwrap().request_redraw();
        self.layer.as_ref().unwrap().commit();
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
        self.converter(qh);
    }
}

/// This is a wrapper around calloop's [loop_handle](https://docs.rs/calloop/latest/calloop/struct.LoopHandle.html)
/// for calling wayland specific features of `SpellWin`. It can be accessed from
/// [`crate::wayland_adapter::SpellWin::get_handler`].
#[derive(Clone, Debug)]
pub struct WinHandle(pub LoopHandle<'static, SpellWin>);

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

    /// Internally calls [`crate::wayland_adapter::SpellWin::set_exclusive_zone`]
    pub fn set_exclusive_zone(&self, val: i32) {
        self.0.insert_idle(move |win| win.set_exclusive_zone(val));
    }
}

/// SpellLock is a struct which represents a window lock. It can be run and initialised
/// on a custom lockscreen implementation with slint.
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
    pub(crate) keyboard_state: Option<WlKeyboard>,
    pub(crate) pointer_state: PointerState,
    pub(crate) touch_state: Option<WlTouch>,
    pub(crate) seat_state: SeatState,
    pub(crate) shm: Shm,
    pub(crate) session_lock: Option<SessionLock>,
    pub(crate) lock_surfaces: Vec<SessionLockSurface>,
    pub(crate) slint_part: Option<SpellSlintLock>,
    pub(crate) is_locked: bool,
    pub(crate) unlock_screen: Sender<bool>,
    // TODO, check if it need internal mutability?
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
    /// This function creates an instance of SpellLock which can be combined with
    /// slint windows to create a lockscreen.
    pub fn invoke_lock_spell() -> Self {
        let conn = Connection::connect_to_env().unwrap();
        let _ = set_up_tracing("SpellLock");
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
        let cursor_manager =
            CursorShapeManager::bind(&globals, &qh).expect("cursor shape is not available");
        let mut win_handler_vec: Vec<(String, (u32, u32))> = Vec::new();
        let lock_surfaces = Vec::new();

        let pointer_state = PointerState {
            pointer: None,
            pointer_data: None,
            cursor_shape: cursor_manager,
            last_cursor_enter_serial: None,
            current_wayland_cursor: MouseCursor::Default,
        };
        let (sender, rx) = channel::channel::<bool>();
        let mut spell_lock = SpellLock {
            loop_handle: event_loop.handle().clone(),
            conn: conn.clone(),
            compositor_state,
            output_state,
            keyboard_state: None,
            touch_state: None,
            pointer_state,
            registry_state,
            seat_state: SeatState::new(&globals, &qh),
            slint_part: None,
            shm,
            session_lock: None,
            lock_surfaces,
            unlock_screen: sender,
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
        let (slint_event_sender, slint_event_receiver) =
            calloop::channel::channel::<Box<dyn FnOnce() + Send>>();
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

        spell_lock
            .loop_handle
            .insert_source(slint_event_receiver, |event, _, data| {
                if let calloop::channel::Event::Msg(callback) = event {
                    callback();

                    if let Some(slint_part) = &data.slint_part {
                        for adapter in &slint_part.adapters {
                            adapter.request_redraw();
                        }
                    }
                }
            })
            .unwrap();

        spell_lock.backspace = Some(
            spell_lock
                .loop_handle
                .insert_source(
                    Timer::from_duration(std::time::Duration::from_millis(1500)),
                    |_, _, data| {
                        data.slint_part.as_ref().unwrap().adapters[0]
                            .try_dispatch_event(slint::platform::WindowEvent::KeyPressed {
                                text: Key::Backspace.into(),
                            })
                            .unwrap();
                        TimeoutAction::ToDuration(std::time::Duration::from_millis(1500))
                    },
                )
                .unwrap(),
        );

        let _ =
            spell_lock
                .loop_handle
                .clone()
                .insert_source(rx, move |event, _, data| match event {
                    channel::Event::Msg(msg) => {
                        if msg {
                            if let Some(locked_val) = data.session_lock.take() {
                                locked_val.unlock();
                            } else {
                                warn!("Authentication verified but couldn't unlock");
                            }
                            data.is_locked = false;
                            data.conn.roundtrip().unwrap();
                        }
                    }
                    channel::Event::Closed => {
                        warn!("Unlock channel to open thread is closed.");
                    }
                });

        spell_lock
            .loop_handle
            .disable(&spell_lock.backspace.unwrap())
            .unwrap();
        let _ = slint::platform::set_platform(Box::new(SpellLockShell::new(
            multi_handler,
            slint_event_sender,
        )));

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
    }

    fn unlock_finger(&mut self, error_callback: Box<dyn FnOnce() + Send>) -> PamResult<()> {
        let sender = self.unlock_screen.clone();
        std::thread::spawn(move || {
            fn unlock_internal(sender: Sender<bool>) -> PamResult<()> {
                let finger = FingerprintInfo;
                let output = Command::new("sh")
                    .arg("-c")
                    .arg("last | awk '{print $1}' | sort | uniq -c | sort -nr")
                    .output()
                    .expect("Couldn't retrive username");

                let val = String::from_utf8_lossy(&output.stdout);
                let val_2 = val.split('\n').collect::<Vec<_>>()[0].trim();
                let user_name = val_2.split(" ").collect::<Vec<_>>()[1].to_string();

                let mut txn = TransactionBuilder::new_with_service("login")
                    .username(user_name)
                    .build(finger.into_conversation())?;
                // If authentication fails, this will return an error.
                // We immediately give up rather than re-prompting the user.
                txn.authenticate(AuthnFlags::empty())?;
                txn.account_management(AuthnFlags::empty())?;
                if let Err(err) = sender.send(true) {
                    warn!("Error sending unlock via sender: {err}");
                }
                Ok(())
            }
            if let Err(err) = unlock_internal(sender) {
                println!("{:?}", err);
                error_callback();
            } else {
                println!("Passed");
            }
        });
        Ok(())
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
        } else {
            warn!("Authentication verified but couldn't unlock");
        }
        self.is_locked = false;
        self.conn.roundtrip().unwrap();
        Ok(())
    }

    /// Provides a lockscreen handler used to invoke the unlock
    /// callback with the user entered password.For more details
    /// view [`LockHandle`].
    pub fn get_handler(&self) -> LockHandle {
        LockHandle(self.loop_handle.clone())
    }
}

impl SpellAssociatedNew for SpellLock {
    fn on_call(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = self.event_loop.clone();
        event_loop
            .borrow_mut()
            .dispatch(std::time::Duration::from_millis(1), self)?;
        Ok(())
    }
    fn is_locked(&self) -> bool {
        self.is_locked
    }
}

/// Struct to handle unlocking of a SpellLock instance. It can be captured from
/// [`SpellLock::get_handler`].
#[derive(Debug, Clone)]
pub struct LockHandle(LoopHandle<'static, SpellLock>);

impl LockHandle {
    /// Call this method to unlock Spelllock. It also takes two callbacks which
    /// are invoked when the password parsed is wrong or right (i.e. resulting
    /// in an screen unlock) respectively. Callbacks can be used to invoke UI
    /// specific changes for your slint frontend.
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

    /// Function which opens fingerprint device for authentication.
    /// error_callback is executed when fingerprint is not registered and fails
    /// to unlock the lockscreen.
    pub fn verify_fingerprint(&self, error_callback: Box<dyn FnOnce() + Send>) {
        self.0.insert_idle(move |app_data| {
            // if let Err(err) = app_data.unlock_finger() {
            //     println!("{:?}", err);
            //     error_callback();
            // } else {
            //     println!("Passed");
            // }
            app_data.unlock_finger(error_callback);
        });
    }
}
delegate_keyboard!(SpellLock);
delegate_compositor!(SpellLock);
delegate_output!(SpellLock);
delegate_shm!(SpellLock);
delegate_registry!(SpellLock);
delegate_pointer!(SpellLock);
delegate_touch!(SpellLock);
delegate_session_lock!(SpellLock);
delegate_seat!(SpellLock);

/// Future XDGpopup implementation will occur on this struct;
pub struct SpellXdg;

/// Furture virtual keyboard implementation will be on this type. Currently, it is redundent.
pub struct SpellBoard;
