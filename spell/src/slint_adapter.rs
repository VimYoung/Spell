use crate::{
    configure::{LayerConf, WindowConf},
    shared_context::{SharedCore, SkiaSoftwareBuffer},
    wayland_adapter::EventAdapter,
};
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext, software_surface::SoftwareSurface};
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
    rc::{Rc, Weak},
};

pub struct SpellWinAdapter {
    pub window: Window,
    pub rendered: SoftwareRenderer,
    pub size: PhysicalSize, //I am not adding any more properties for now and not puttinting it in a
    pub needs_redraw: Cell<bool>,
}

impl SpellWinAdapter {
    pub fn new(width: u32, height: u32) -> Rc<Self> {
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

pub struct SpellLayerShell {
    pub window_adapter: Rc<SpellSkiaWinAdapter>,
}

impl Platform for SpellLayerShell {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window_adapter.clone())
    }
}

pub struct SpellMultiLayerShell {
    pub window_manager: Rc<RefCell<SpellMultiWinHandler>>,
}

impl Platform for SpellMultiLayerShell {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let value = self.window_manager.borrow_mut().request_new_window();
        Ok(value)
    }
}

pub struct SpellMultiWinHandler {
    pub windows: Vec<(String, LayerConf)>,
    pub adapter: Vec<Rc<SpellSkiaWinAdapter>>,
    pub core: Vec<Rc<RefCell<SharedCore>>>,
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

pub struct SpellSkiaWinAdapter {
    pub window: Window,
    pub size: PhysicalSize,
    pub renderer: SkiaRenderer,
    pub buffer: Rc<RefCell<SharedCore>>,
    pub needs_redraw: Cell<bool>,
}

impl WindowAdapter for SpellSkiaWinAdapter {
    fn window(&self) -> &slint::Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        self.size
    }

    fn renderer(&self) -> &dyn slint::platform::Renderer {
        &self.renderer
    }

    fn set_size(&self, size: slint::WindowSize) {
        let physical_size = size.to_physical(1.);
        self.buffer
            .borrow_mut()
            .resize(physical_size.width, physical_size.height);
        self.window
            .dispatch_event(slint::platform::WindowEvent::Resized {
                size: size.to_logical(1.),
            })
    }

    fn request_redraw(&self) {
        self.needs_redraw.set(true);
    }
}

impl std::fmt::Debug for SpellSkiaWinAdapter {
    // TODO this needs to be implemented properly
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl SpellSkiaWinAdapter {
    pub fn new(shared_core: Rc<RefCell<SharedCore>>, width: u32, height: u32) -> Rc<Self> {
        let buffer = Rc::new(SkiaSoftwareBuffer {
            core: shared_core.clone(),
            last_dirty_region: Default::default(),
        });
        let renderer = SkiaRenderer::new_with_surface(
            &SkiaSharedContext::default(),
            Box::new(SoftwareSurface::from(buffer.clone())),
        );
        Rc::new_cyclic(|w: &Weak<Self>| Self {
            window: slint::Window::new(w.clone()),
            size: PhysicalSize { width, height },
            renderer,
            buffer: shared_core,
            needs_redraw: Cell::new(true),
        })
    }

    pub fn draw(&self) -> bool {
        if self.needs_redraw.replace(false) {
            self.renderer.render().unwrap();
            true
        } else {
            false
        }
    }

    // fn last_dirty_region_bounding_box_size(&self) -> Option<slint::LogicalSize> {
    //     self.buffer.last_dirty_region.borrow().as_ref().map(|r| {
    //         let size = r.bounding_rect().size;
    //         slint::LogicalSize::new(size.width as _, size.height as _)
    //     })
    // }
    // fn last_dirty_region_bounding_box_origin(&self) -> Option<slint::LogicalPosition> {
    //     self.buffer.last_dirty_region.borrow().as_ref().map(|r| {
    //         let origin = r.bounding_rect().origin;
    //         slint::LogicalPosition::new(origin.x as _, origin.y as _)
    //     })
    // }
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

    // An idea could be gathered by multiplying core's width
    // and height but that doesn't help in reciezing involving only dimention
    // changes.
    fn size(&self) -> PhysicalSize {
        self.size
    }

    fn set_widget_size(&self, size: PhysicalSize) {
        self.set_size(size.into());
    }
}
