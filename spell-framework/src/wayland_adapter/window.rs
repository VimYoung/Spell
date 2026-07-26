use tracing_subscriber::EnvFilter;

use crate::{
    PopupSlint, SpellAssociatedNew,
    configure::{Dimension, HomeHandle, PopupConf, WindowConf, set_up_tracing},
    slint_adapter::{ADAPTERS, SpellLayerShell, SpellSkiaWinAdapter},
    wayland_adapter::{
        common::PointerState,
        fractional_scaling::{FractionalScaleState, delegate_fractional_scale},
        viewporter::{Viewport, ViewporterState, delegate_viewporter},
        window,
    },
};
use i_slint_core::items::MouseCursor;
use slint::platform::WindowAdapter;
use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm, delegate_touch, delegate_xdg_popup,
    delegate_xdg_shell,
    output::OutputState,
    reexports::{
        calloop::{
            self, EventLoop, LoopHandle,
            timer::{TimeoutAction, Timer},
        },
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, EventQueue, QueueHandle,
            globals::registry_queue_init,
            protocol::{
                wl_keyboard::WlKeyboard,
                wl_output::{self, WlOutput},
                wl_region::WlRegion,
                wl_shm,
                wl_surface::WlSurface,
                wl_touch::WlTouch,
            },
        },
    },
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager},
    shell::{
        WaylandSurface,
        wlr_layer::{KeyboardInteractivity, LayerShell, LayerSurface},
        xdg::{XdgPositioner, XdgShell, popup::Popup},
    },
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    os::unix::net::UnixListener,
    rc::Rc,
    sync::{Once, OnceLock, RwLock},
    time::Duration,
};
use tracing::{Level, info, span, trace, warn};

mod input;
mod internal;
mod popup;
mod wayland;

#[allow(clippy::type_complexity)]
static AVAILABLE_MONITORS: OnceLock<RwLock<HashMap<String, (wl_output::WlOutput, i32, i32)>>> =
    OnceLock::new();
static SET_SLINT_PLATFORM: Once = Once::new();

#[derive(Debug)]
struct States {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    pointer_state: PointerState,
    keyboard_state: Option<WlKeyboard>,
    touch_state: Option<WlTouch>,
    shm: Shm,
    viewporter_state: ViewporterState,
    fractional_scale_state: FractionalScaleState,
}

/// `SpellWin` is the main type for implementing widgets, it covers various properties and trait
/// implementation, thus providing various features.
pub struct SpellWin {
    adapter: Option<Rc<SpellSkiaWinAdapter>>,
    loop_handle: LoopHandle<'static, SpellWin>,
    /// UnixListener storing remote instructions from CLI.
    pub ipc_handler: Option<UnixListener>,
    /// Name of widget's layer.
    pub layer_name: String,
    /// Span required for proper logging.
    pub span: span::Span,
    queue: QueueHandle<SpellWin>,
    buffer: Option<Buffer>,
    states: States,
    layer: Option<LayerSurface>,
    first_configure: Cell<bool>,
    natural_scroll: bool,
    is_hidden: Cell<bool>,
    config: WindowConf,
    input_region: Region,
    opaque_region: Region,
    viewport: Option<Viewport>,
    xdg_shell: XdgShell,
    popup_manager: window::popup::PopupManager,
    event_loop: Rc<RefCell<EventLoop<'static, SpellWin>>>,
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
    fn create_window(
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
        let surface = compositor.create_surface(&qh);
        let viewporter_state =
            ViewporterState::bind(&globals, &qh).expect("Couldn't set viewporter");
        let fractional_scale_state: FractionalScaleState =
            FractionalScaleState::bind(&globals, &qh).expect("Fractional Scale couldn't be set");
        let xdg_shell = XdgShell::bind(&globals, &qh).expect("Couldn't bind xdg_shell");
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
            queue: qh.clone(),
            buffer: None,
            states: States {
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),
                compositor_state: compositor,
                pointer_state,
                keyboard_state: None,
                touch_state: None,
                shm,
                viewporter_state,
                fractional_scale_state,
            },
            layer: None,
            first_configure: Cell::new(true),
            natural_scroll: window_conf.natural_scroll,
            is_hidden: Cell::new(false),
            config: window_conf.clone(),
            layer_name: layer_name.clone(),
            input_region,
            opaque_region,
            viewport: None,
            xdg_shell,
            popup_manager: window::popup::PopupManager::new(),
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
        // win.popup_manager.set_pool(pool_mut.clone());
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
        let target_output: Option<&WlOutput> = output_info.as_ref().map(|(a, _, _)| a);
        let layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            window_conf.layer_type,
            Some(layer_name.clone()),
            target_output,
        );

        window::set_config(
            &win.config,
            &layer,
            //true,
            Some(win.input_region.wl_region()),
            None,
        );
        if let Err(err) = event_queue.roundtrip(&mut win) {
            warn!("Received roundtrip error: {}", err);
        }
        win.layer = Some(layer);
        let surface: &WlSurface = win.layer.as_ref().unwrap().wl_surface();
        let fractional_scale = win.states.fractional_scale_state.get_scale(surface, &qh);
        let viewport = win
            .states
            .viewporter_state
            .get_viewport(surface, &qh, fractional_scale);
        win.viewport = Some(viewport);

        win.layer.as_ref().unwrap().commit();
        window::set_event_sources(
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
    /// This function fetches the available monitors from the Wayland registry
    /// and returns a map of the available monitors where the key is the name
    /// of the monitor and the value is [`wl_output::WlOutput`] with its assosiated
    /// logical size(width, height). Dimentions are later used in size determination.
    /// It uses an already registered event queue & spell window.
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
    // self.set_config_internal();
    pub fn set_exclusive_zone(&mut self, val: i32) {
        self.config.exclusive_zone = Some(val);
        self.layer.as_ref().unwrap().set_exclusive_zone(val);
        self.layer.as_ref().unwrap().commit();
    }

    pub fn open_popup<T: PopupSlint + 'static>(
        &mut self,
        popup_conf: PopupConf,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let popup_surface = self.states.compositor_state.create_surface(&self.queue);
        // popup_surface.commit();
        let position =
            XdgPositioner::new(&self.xdg_shell).expect("Failed to created XdgPositioner");
        position.set_size(popup_conf.width as i32, popup_conf.height as i32);
        position.set_parent_size(
            self.config.evaluated_width as i32,
            self.config.evaluated_height as i32,
        );
        position.set_anchor(popup_conf.anchor);
        position.set_anchor_rect(
            popup_conf.anchor_rect.0,
            popup_conf.anchor_rect.1,
            popup_conf.anchor_rect.2,
            popup_conf.anchor_rect.3,
        );
        // popup_surface.commit();
        if let Ok(popup) = Popup::from_surface(
            // Some(self.popup_manager.xdg_surface()),
            None,
            &position,
            &self.queue,
            popup_surface,
            &self.xdg_shell,
        ) {
            let pool = SlotPool::new(
                (popup_conf.width * popup_conf.height * 4) as usize,
                &self.states.shm,
            )
            .expect("Unable to create slot pool for popup");
            self.popup_manager.set_pool(Rc::new(RefCell::new(pool)));
            self.layer.as_ref().unwrap().get_popup(popup.xdg_popup());
            // popup.wl_surface().commit();

            let id = self.popup_manager.create_popup::<T>(
                popup,
                popup_conf,
                &self.states.fractional_scale_state,
                &self.states.viewporter_state,
                &self.queue,
            );
            info!("Popup created with id: {}", id);
            Ok(id)
        } else {
            warn!("couldn't create a popup");
            Err("Couldn't create Popup".into())
        }
    }

    pub fn close_popup(&mut self, id: u32) {
        self.popup_manager.close_popup(&id);
    }
}

delegate_compositor!(SpellWin);
delegate_xdg_shell!(SpellWin);
delegate_xdg_popup!(SpellWin);
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

    pub fn open_popup<T: PopupSlint + 'static>(
        &mut self,
        popup_conf: PopupConf,
        callback: Box<dyn FnOnce(u32)>,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        self.0.insert_idle(|win| {
            if let Ok(id) = win.open_popup::<T>(popup_conf) {
                callback(id);
            }
        });
        Ok(0)
    }

    pub fn close_popup(&self, id: u32) {
        self.0.insert_idle(move |win| {
            win.close_popup(id);
        });
    }
}

pub(super) fn set_event_sources(
    event_loop: &EventLoop<'static, SpellWin>,
    handle: HomeHandle,
    slint_event_receiver: calloop::channel::Channel<Box<dyn FnOnce() + Send>>,
) {
    // let backspace_event = event_loop
    //     .handle()
    //     .insert_source(
    //         Timer::from_duration(Duration::from_millis(1500)),
    //         |_, _, data| {
    //             data.adapter
    //                 .try_dispatch_event(slint::platform::WindowEvent::KeyPressed {
    //                     text: Key::Backspace.into(),
    //                 })
    //                 .unwrap();
    //             TimeoutAction::ToDuration(Duration::from_millis(1500))
    //         },
    //     )
    //     .unwrap();
    // event_loop.handle().disable(&backspace_event).unwrap();

    // // Inserting tracing source
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("runtime dir is not set");
    let logging_dir = runtime_dir + "/spell/";
    let socket_cli_dir = logging_dir.clone() + "/spell_cli";

    // This is currently redundent source as it is not working in any way
    event_loop
        .handle()
        .insert_source(
            Timer::from_duration(Duration::from_secs(2)),
            move |_, _, _| {
                let file = fs::File::open(&socket_cli_dir)
                    .unwrap_or_else(|_| fs::File::create_new(&socket_cli_dir).unwrap());
                let buf = BufReader::new(file);
                let file_contents: Vec<String> = buf
                    .lines()
                    .map(|l| l.expect("Could not parse line"))
                    .collect();
                if !file_contents.is_empty() {
                    match file_contents[0].as_str() {
                        "slint_log" => {
                            handle
                                .modify(|layer| {
                                    *layer.filter_mut() =
                                        EnvFilter::new("spell_framework::slint_adapter=info,warn");
                                })
                                .unwrap_or_else(|error| {
                                    warn!("Error when setting slint_log: {}", error);
                                });
                        }
                        "debug" => {
                            handle
                                .modify(|layer| {
                                    *layer.filter_mut() =
                                        EnvFilter::new("spell_framework=info,warn"); //*layer;
                                })
                                .unwrap_or_else(|error| {
                                    warn!("Error when setting slint_log: {}", error);
                                });
                        }
                        "dump" => {
                            handle
                                .modify(|layer| {
                                    *layer.filter_mut() =
                                        EnvFilter::new("spell_framework=trace,info"); //*layer;
                                })
                                .unwrap_or_else(|error| {
                                    warn!("Error when setting slint_log: {}", error);
                                });
                        }
                        "dev" => {
                            handle
                                .modify(|layer| {
                                    *layer.filter_mut() =
                                        EnvFilter::new("spell_framework=trace,warn"); //*layer;
                                })
                                .unwrap_or_else(|error| {
                                    warn!("Error when setting slint_log: {}", error);
                                });
                        }
                        val => {
                            warn!("Something else came: {}", val);
                        }
                    }
                }
                TimeoutAction::ToDuration(Duration::from_secs(2))
            },
        )
        .unwrap();

    event_loop
        .handle()
        .insert_source(slint_event_receiver, |event, _, data| {
            if let calloop::channel::Event::Msg(callback) = event {
                callback();
                data.adapter.as_ref().unwrap().request_redraw();
            }
        })
        .unwrap();
}

pub(super) fn set_config(
    window_conf: &WindowConf,
    layer: &LayerSurface,
    input_region: Option<&WlRegion>,
    opaque_region: Option<&WlRegion>,
) {
    layer.set_size(window_conf.evaluated_width, window_conf.evaluated_height);
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
    layer.set_layer(window_conf.layer_type);
    set_anchor(window_conf, layer);
}

fn set_anchor(window_conf: &WindowConf, layer: &LayerSurface) {
    let mut anchors = window_conf.anchor.into_iter().flatten();
    if let Some(mut combined) = anchors.next() {
        for a in anchors {
            combined.insert(a);
        }
        layer.set_anchor(combined);
    }
    if let Some(val) = window_conf.exclusive_zone {
        layer.set_exclusive_zone(val);
    }
}
