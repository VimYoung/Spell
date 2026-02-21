//! This module contains relevent structs for slint side backend configurations.
//! All structs mentioned are either internal or not used anymore. Still their
//! implementation is public because they had to be set by the user of library
//! in intial iterations of spell_framework.
use crate::configure::LayerConf;
use slint::platform::{EventLoopProxy, Platform, WindowAdapter};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tracing::{Level, info, span, warn};

thread_local! {
    pub(crate) static ADAPTERS: RefCell<Vec<Rc<SpellSkiaWinAdapter>>> = const { RefCell::new(Vec::new()) };
}

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
    pub span: span::Span,
}

impl Default for SpellLayerShell {
    /// Creates an instance of this Platform implementation, for internal use.
    fn default() -> Self {
        SpellLayerShell {
            span: span!(Level::INFO, "slint-log",),
        }
    }
}

impl Platform for SpellLayerShell {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let adapter = ADAPTERS.with(|v| v.borrow().last().unwrap().clone());
        Ok(adapter)
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
        Some(Box::new(SlintEventProxy(ADAPTERS.with(|v| {
            v.borrow().last().unwrap().slint_event_proxy.clone()
        }))))
    }
}

/// This struct is responsible for handling, initialising, updating and maintaining
/// of various widgets that are being rendered simultaneously across monitors for
/// your lock. It uses [SpellSkiaWinAdapter] internally. This struct is made public
/// for documentation purposes (and was previously used by end user of library) but
/// it is now not to be used directly.
pub struct SpellMultiWinHandler {
    pub(crate) windows: Vec<(String, LayerConf)>,
    pub(crate) adapter: Vec<Rc<SpellSkiaWinAdapter>>,
    pub(crate) value_given: u32,
}

impl SpellMultiWinHandler {
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
