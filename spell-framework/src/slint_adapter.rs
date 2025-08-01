//! This module contains relevent structs for slint side backend configurations. Apart
//! from [SpellMultiLayerShell] and [SpellMultiWinHandler], rest of the structs mentioned are
//! either internal or not used anymore. Still their implementation is public because they had be
//! set by the user of library in intial iterations of spell_framework.
use crate::{
    configure::{LayerConf, WindowConf},
    shared_context::SharedCore,
    wayland_adapter::EventAdapter,
};
use slint::{
    PhysicalSize, Window,
    platform::{
        Platform, WindowAdapter,
        software_renderer::{
            RepaintBufferType::{self},
            SoftwareRenderer,
        },
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[cfg(not(docsrs))]
#[cfg(feature = "i-slint-renderer-skia")]
use crate::skia_non_docs::SpellSkiaWinAdapterReal;

#[cfg(not(docsrs))]
#[cfg(feature = "i-slint-renderer-skia")]
pub type SpellSkiaWinAdapter = SpellSkiaWinAdapterReal;

#[cfg(docsrs)]
use crate::dummy_skia_docs::SpellSkiaWinAdapterDummy;

#[cfg(docsrs)]
pub type SpellSkiaWinAdapter = SpellSkiaWinAdapterDummy;

/// This was the previous struct used internally for rendering, its use is stopped in favour of
/// [SpellSkiaWinAdapter] which provides faster and more reliable rendering. It implements slint's
/// [WindowAdapter](https://docs.rs/slint/latest/slint/platform/trait.WindowAdapter.html) trait.
pub struct SpellWinAdapter {
    pub window: Window,
    pub rendered: SoftwareRenderer,
    pub size: PhysicalSize, //I am not adding any more properties for now and not puttinting it in a
    pub needs_redraw: Cell<bool>,
}

// TODO this dead code needs to be either removed or imporoved.
#[allow(dead_code)]
impl SpellWinAdapter {
    fn new(width: u32, height: u32) -> Rc<Self> {
        Rc::<SpellWinAdapter>::new_cyclic(|adapter| SpellWinAdapter {
            window: Window::new(adapter.clone()),
            rendered: SoftwareRenderer::new_with_repaint_buffer_type(
                RepaintBufferType::ReusedBuffer,
            ),
            size: PhysicalSize { width, height },
            needs_redraw: Default::default(),
        })
    }

    fn draw_if_needed(&self, render_callback: impl FnOnce(&SoftwareRenderer)) -> bool {
        if self.needs_redraw.replace(false) {
            render_callback(&self.rendered);
            true
        } else {
            false
        }
    }
}

impl WindowAdapter for SpellWinAdapter {
    fn window(&self) -> &Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        // This value have to be made dynamic by using `xandr`
        PhysicalSize {
            width: self.size.width,
            height: self.size.height,
        }
    }

    fn renderer(&self) -> &dyn slint::platform::Renderer {
        &self.rendered
    }

    fn request_redraw(&self) {
        self.needs_redraw.set(true);
    }
}

/// Previously needed to be implemented, now this struct is called and set internally
/// when [`invoke_spell`](crate::wayland_adapter::SpellWin::invoke_spell) is called.
pub struct SpellLayerShell {
    /// An instance of [SpellSkiaWinAdapter].
    pub window_adapter: Rc<SpellSkiaWinAdapter>,
}

impl Platform for SpellLayerShell {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window_adapter.clone())
    }
}

/// This struct needs to be set when multiple windows are to be started together. It is
/// used in combination with [`conjure_spells`](crate::wayland_adapter::SpellWin::conjure_spells)
/// and is required to be set before any other initialisation with an instance of [SpellMultiWinHandler].
/// It implements slint's [Platform](https://docs.rs/slint/latest/slint/platform/trait.Platform.html) trait.
/// Example of setting it is as follows:
/// ```rust
/// slint::platform::set_platform(Box::new(SpellMultiLayerShell {
///     window_manager: windows_handler.clone(),
/// })).unwrap();
/// ```
pub struct SpellMultiLayerShell {
    /// An instance of [SpellMultiWinHandler].
    pub window_manager: Rc<RefCell<SpellMultiWinHandler>>,
}

impl Platform for SpellMultiLayerShell {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let value = self.window_manager.borrow_mut().request_new_window();
        Ok(value)
    }
}

/// Used for the initialisation of [SpellMultiLayerShell], this struct is responsible
/// for handling, initialising, updating and maintaing of various widgets that are being
/// rendered simultaneously. It uses [SpellSkiaWinAdapter] internally.
pub struct SpellMultiWinHandler {
    pub(crate) windows: Vec<(String, LayerConf)>,
    pub(crate) adapter: Vec<Rc<SpellSkiaWinAdapter>>,
    pub(crate) core: Vec<Rc<RefCell<SharedCore>>>,
}

impl SpellMultiWinHandler {
    pub fn new(windows: Vec<(&str, WindowConf)>) -> Rc<RefCell<Self>> {
        let new_windows: Vec<(String, LayerConf)> = windows
            .iter()
            .map(|(layer_name, conf)| (layer_name.to_string(), LayerConf::Window(conf.clone())))
            .collect();

        let cores: Vec<Rc<RefCell<SharedCore>>> = windows
            .iter()
            .map(|(_, conf)| Rc::new(RefCell::new(SharedCore::new(conf.width, conf.height))))
            .collect();

        Rc::new(RefCell::new(SpellMultiWinHandler {
            windows: new_windows,
            adapter: Vec::new(),
            core: cores,
        }))
    }

    fn request_new_window(&mut self) -> Rc<dyn WindowAdapter> {
        let length = self.adapter.len();
        let core = &self.core[length];
        if let LayerConf::Window(conf) = &self.windows[length].1 {
            let adapter = SpellSkiaWinAdapter::new(core.clone(), conf.width, conf.height);
            self.adapter.push(adapter.clone());
            adapter
        } else {
            panic!("Panicked here");
        }
    }
}

impl EventAdapter for SpellSkiaWinAdapter {
    fn draw_if_needed(&self) -> bool {
        self.draw()
    }

    fn try_dispatch_event(
        &self,
        event: slint::platform::WindowEvent,
    ) -> Result<(), slint::PlatformError> {
        self.window.try_dispatch_event(event)
    }
}
