use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use crate::{
    shared_context::{SharedCore, SkiaSoftwareBuffer},
    wayland_adapter::EventAdapter,
};
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext, software_surface::SoftwareSurface};
use slint::{PhysicalSize, Window, platform::WindowAdapter};

pub struct SpellSkiaWinAdapter {
    pub window: Window,
    pub size: PhysicalSize,
    pub skia_buffer: Rc<SkiaSoftwareBuffer>,
    pub renderer: SkiaRenderer,
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

    // fn set_size(&self, size: slint::WindowSize) {
    //     self.size.set(size.to_physical(1.));
    //     self.window
    //         .dispatch_event(slint::platform::WindowEvent::Resized {
    //             size: size.to_logical(1.),
    //         })
    // }
    //
    fn request_redraw(&self) {
        self.needs_redraw.set(true);
    }
}

impl SpellSkiaWinAdapter {
    pub fn new(shared_core: Rc<RefCell<SharedCore>>, width: u32, height: u32) -> Rc<Self> {
        let buffer = Rc::new(SkiaSoftwareBuffer {
            core: shared_core,
            last_dirty_region: Default::default(),
        });
        let renderer = SkiaRenderer::new_with_surface(
            &SkiaSharedContext::default(),
            Box::new(SoftwareSurface::from(buffer.clone())),
        );
        Rc::new_cyclic(|w: &Weak<Self>| Self {
            window: slint::Window::new(w.clone()),
            size: PhysicalSize { width, height },
            skia_buffer: buffer,
            renderer,
            needs_redraw: Default::default(),
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
}
