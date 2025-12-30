//! This module contains relevent structs for slint side backend configurations. Apart
//! from [SpellMultiLayerShell] and [SpellMultiWinHandler], rest of the structs mentioned are
//! either internal or not used anymore. Still their implementation is public because they had be
//! set by the user of library in intial iterations of spell_framework.
use crate::{
    configure::{LayerConf, WindowConf, set_up_tracing},
    wayland_adapter::SpellWin,
};
use slint::{
    PhysicalSize, Window,
    platform::{
        EventLoopProxy, Platform, WindowAdapter,
        software_renderer::{
            RepaintBufferType::{self},
            SoftwareRenderer,
        },
    },
};
use smithay_client_toolkit::reexports::client::Connection;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex},
};
use tracing::{Level, info, span, warn};

#[cfg(not(docsrs))]
#[cfg(feature = "i-slint-renderer-skia")]
use crate::skia_non_docs::SpellSkiaWinAdapterReal;

/// It is the main struct handling the rendering of pixels in the wayland window. It implements slint's
/// [WindowAdapter](https://docs.rs/slint/latest/slint/platform/trait.WindowAdapter.html) trait.
/// It is used internally by [SpellMultiWinHandler] and previously by [SpellLayerShell]. This
/// adapter internally uses [Skia](https://skia.org/) 2D graphics library for rendering.
#[cfg(not(docsrs))]
#[cfg(feature = "i-slint-renderer-skia")]
pub type SpellSkiaWinAdapter = SpellSkiaWinAdapterReal;

#[cfg(docsrs)]
use crate::dummy_skia_docs::SpellSkiaWinAdapterDummy;

/// It is the main struct handling the rendering of pixels in the wayland window. It implements slint's
/// [WindowAdapter](https://docs.rs/slint/latest/slint/platform/trait.WindowAdapter.html) trait.
/// It is used internally by [SpellMultiWinHandler] and previously by [SpellLayerShell]. This
/// adapter internally uses [Skia](https://skia.org/) 2D graphics library for rendering.
#[cfg(docsrs)]
pub type SpellSkiaWinAdapter = SpellSkiaWinAdapterDummy;
/// Previously needed to be implemented, now this struct is called and set internally
/// when [`invoke_spell`](crate::wayland_adapter::SpellWin::invoke_spell) is called.

pub struct SpellLayerShell {
    /// An instance of [SpellSkiaWinAdapter].
    pub window_adapter: Rc<SpellSkiaWinAdapter>,
    pub span: span::Span,
}

impl SpellLayerShell {
    /// Creates an instance of this Platform implementation, for internal use.
    pub fn new(window_adapter: Rc<SpellSkiaWinAdapter>) -> Self {
        SpellLayerShell {
            window_adapter,
            span: span!(Level::INFO, "slint-log",),
        }
    }
}

impl Platform for SpellLayerShell {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window_adapter.clone())
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        self.span.in_scope(|| {
            if let Some(val) = arguments.as_str() {
                info!(val);
            } else {
                info!("{}", arguments.to_string());
            }
        })
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(SlintEventProxy(
            self.window_adapter.slint_event_proxy.clone(),
        )))
    }
}

/// This struct needs to be set when multiple windows are to be started together. It is
/// used in combination with [`conjure_spells`](crate::slint_adapter::SpellMultiWinHandler::conjure_spells)
/// and is required to be set before any other initialisation with an instance of [SpellMultiWinHandler].
/// It implements slint's [Platform](https://docs.rs/slint/latest/slint/platform/trait.Platform.html) trait and is set internally.
pub struct SpellMultiLayerShell {
    /// An instance of [SpellMultiWinHandler].
    pub window_manager: Rc<RefCell<SpellMultiWinHandler>>,
    pub span: span::Span,
}

impl SpellMultiLayerShell {
    fn new(window_manager: Rc<RefCell<SpellMultiWinHandler>>) -> Self {
        SpellMultiLayerShell {
            window_manager,
            span: span!(Level::INFO, "slint-log",),
        }
    }
}

impl Platform for SpellMultiLayerShell {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let value = self.window_manager.borrow_mut().request_new_window();
        Ok(value)
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        self.span.in_scope(|| {
            if let Some(val) = arguments.as_str() {
                info!(val);
            } else {
                info!("{}", arguments.to_string());
            }
        })
    }
}

/// Used for the initialisation of [SpellMultiLayerShell], this struct is responsible
/// for handling, initialising, updating and maintaing of various widgets that are being
/// rendered simultaneously. It uses [SpellSkiaWinAdapter] internally.
pub struct SpellMultiWinHandler {
    pub(crate) windows: Vec<(String, LayerConf)>,
    pub(crate) adapter: Vec<Rc<SpellSkiaWinAdapter>>,
    pub(crate) value_given: u32,
}

impl SpellMultiWinHandler {
    /// This function is finally called to create instances of windows (in a multi
    /// window scenario). These windows are ultimately passed on to [enchant_spells](`crate::enchant_spells`)
    /// event loop, multi-window setup is unstable though and is not recommended for end use just
    /// now.
    pub fn conjure_spells(windows: Vec<(&str, WindowConf)>) -> Vec<SpellWin> {
        tracing_subscriber::fmt()
            .without_time()
            .with_target(false)
            .with_env_filter(tracing_subscriber::EnvFilter::new(
                "spell_framework=trace,info",
            ))
            .init();
        let handle = set_up_tracing("multi-window");
        let conn = Connection::connect_to_env().unwrap();
        let new_windows: Vec<(String, LayerConf)> = windows
            .iter()
            .map(|(layer_name, conf)| (layer_name.to_string(), LayerConf::Window(conf.clone())))
            .collect();

        let mut new_adapters: Vec<Rc<SpellSkiaWinAdapter>> = Vec::new();
        let mut windows_spell: Vec<SpellWin> = Vec::new();
        windows.iter().for_each(|(layer_name, conf)| {
            let window = SpellWin::create_window(
                &conn,
                conf.clone(),
                layer_name.to_string(),
                false,
                handle.clone(),
            );
            new_adapters.push(window.adapter.clone());
            windows_spell.push(window);
        });
        let windows_handler = Rc::new(RefCell::new(SpellMultiWinHandler {
            windows: new_windows,
            adapter: new_adapters,
            value_given: 0,
        }));

        if let Err(error) =
            slint::platform::set_platform(Box::new(SpellMultiLayerShell::new(windows_handler)))
        {
            warn!("Error setting multilayer platform: {}", error);
        }
        windows_spell
    }

    pub(crate) fn new_lock(lock_outputs: Vec<(String, (u32, u32))>) -> Rc<RefCell<Self>> {
        let new_locks: Vec<(String, LayerConf)> = lock_outputs
            .iter()
            .map(|(output_name, conf)| (output_name.clone(), LayerConf::Lock(conf.0, conf.1)))
            .collect();

        Rc::new(RefCell::new(SpellMultiWinHandler {
            windows: new_locks,
            adapter: Vec::new(),
            value_given: 0,
        }))
    }

    fn request_new_window(&mut self) -> Rc<dyn WindowAdapter> {
        self.value_given += 1;
        let index = self.value_given - 1;
        self.adapter[index as usize].clone()
    }

    fn request_new_lock(&mut self) -> Rc<dyn WindowAdapter> {
        self.value_given += 1;
        let index = self.value_given - 1;
        self.adapter[index as usize].clone()
    }
}

pub struct SpellLockShell {
    /// An instance of [SpellMultiWinHandler].
    pub window_manager: Rc<RefCell<SpellMultiWinHandler>>,
    pub span: span::Span,
}

impl SpellLockShell {
    pub fn new(window_manager: Rc<RefCell<SpellMultiWinHandler>>) -> Self {
        SpellLockShell {
            window_manager,
            span: span!(Level::INFO, "slint-lock-log",),
        }
    }
}

impl Platform for SpellLockShell {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let value = self.window_manager.borrow_mut().request_new_lock();
        Ok(value)
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        self.span.in_scope(|| {
            if let Some(val) = arguments.as_str() {
                info!(val);
            } else {
                info!("{}", arguments.to_string());
            }
        })
    }
}

#[allow(clippy::type_complexity)]
struct SlintEventProxy(Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>);

impl EventLoopProxy for SlintEventProxy {
    fn quit_event_loop(&self) -> Result<(), i_slint_core::api::EventLoopError> {
        Ok(())
    }

    fn invoke_from_event_loop(
        &self,
        event: Box<dyn FnOnce() + Send>,
    ) -> Result<(), i_slint_core::api::EventLoopError> {
        if let Ok(mut list_of_event) = self.0.try_lock() {
            (*list_of_event).push(event);
        } else {
            warn!("Slint proxy event could not be processed");
        }
        Ok(())
    }
}
