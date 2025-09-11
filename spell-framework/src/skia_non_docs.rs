#[cfg(not(docsrs))]
use slint::{PhysicalSize, Window, platform::WindowAdapter};
use smithay_client_toolkit::shm::slot::Buffer;
#[cfg(not(docsrs))]
#[cfg(feature = "i-slint-renderer-skia")]
use smithay_client_toolkit::{
    reexports::client::protocol::wl_shm,
    shm::slot::{Slot, SlotPool},
};
use std::{
    cell::Cell,
    cell::RefCell,
    rc::{Rc, Weak},
};

#[cfg(feature = "i-slint-renderer-skia")]
use i_slint_renderer_skia::{
    skia_safe::{self, ColorType},
    software_surface::RenderBuffer,
};

#[cfg(feature = "i-slint-renderer-skia")]
#[cfg(not(docsrs))]
pub struct SkiaSoftwareBufferReal {
    pub primary_slot: RefCell<Slot>,
    pub pool: Rc<RefCell<SlotPool>>,
    pub last_dirty_region: RefCell<Option<i_slint_core::item_rendering::DirtyRegion>>,
}

impl SkiaSoftwareBufferReal {
    pub(crate) fn refresh_buffer(&self, width: u32, height: u32) -> Buffer {
        let (wayland_buffer, _) = self
            .pool
            .borrow_mut()
            .create_buffer(
                width as i32,
                height as i32,
                (width * 4) as i32,
                wl_shm::Format::Argb8888,
            )
            .expect("Creating Buffer");
        // TODO this was previously set, if rendering causes issues, uncomment this.
        // self.set_config_internal();
        *self.primary_slot.borrow_mut() = wayland_buffer.slot();
        wayland_buffer
    }
}

#[allow(unused_variables)]
impl RenderBuffer for SkiaSoftwareBufferReal {
    fn with_buffer(
        &self,
        window: &Window,
        size: PhysicalSize,
        render_callback: &mut dyn for<'a> FnMut(
            std::num::NonZero<u32>,
            std::num::NonZero<u32>,
            ColorType,
            u8,
            &'a mut [u8],
        ) -> Result<
            Option<i_slint_core::item_rendering::DirtyRegion>,
            slint::PlatformError,
        >,
    ) -> std::result::Result<(), slint::PlatformError> {
        // println!("This trait implementation of RenderBuffer is Run");
        let Some((width, height)): Option<(std::num::NonZeroU32, std::num::NonZeroU32)> =
            size.width.try_into().ok().zip(size.height.try_into().ok())
        else {
            // Nothing to render
            return Ok(());
        };

        // let mut shared_pixel_buffer = self.pixels.borrow_mut().take();
        //
        // if shared_pixel_buffer.as_ref().is_some_and(|existing_buffer| {
        //     existing_buffer.width() != width.get() || existing_buffer.height() != height.get()
        // }) {
        //     shared_pixel_buffer = None;
        // }

        // This code ensures that the value need not be null. This can't be a case with
        // box as the value is ensured to be defined by itself during the creation.
        // let mut age = 1;
        // let pixels = shared_pixel_buffer.get_or_insert_with(|| {
        //     age = 0;
        //     SharedPixelBuffer::new(width.get(), height.get())
        // });
        let pool = &mut self.pool.borrow_mut();
        // let mut native_buffer = {
        //     let x = self.secondary_slot.borrow().canvas(pool).unwrap();
        //     // creates a copy
        //     x.to_vec()
        // };

        // let bytes = bytemuck::cast_slice_mut(&mut native_buffer);
        *self.last_dirty_region.borrow_mut() = render_callback(
            width,
            height,
            skia_safe::ColorType::BGRA8888,
            1,
            self.primary_slot.borrow_mut().canvas(pool).unwrap(),
        )
        .unwrap();
        Ok(())
    }
}

#[cfg(feature = "i-slint-renderer-skia")]
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext, software_surface::SoftwareSurface};
#[cfg(feature = "i-slint-renderer-skia")]
/// It is the main struct handling the rendering of pixels in the wayland window. It implements slint's
/// [WindowAdapter](https://docs.rs/slint/latest/slint/platform/trait.WindowAdapter.html) trait.
/// It is used internally by [SpellMultiWinHandler] and previously by [SpellLayerShell]. This
/// adapter internally uses [Skia](https://skia.org/) 2D graphics library for rendering.
pub struct SpellSkiaWinAdapterReal {
    pub(crate) window: Window,
    pub(crate) size: PhysicalSize,
    pub(crate) renderer: SkiaRenderer,
    pub(crate) buffer_slint: Rc<SkiaSoftwareBufferReal>,
    pub(crate) needs_redraw: Cell<bool>,
}

impl WindowAdapter for SpellSkiaWinAdapterReal {
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
//
// impl std::fmt::Debug for SpellSkiaWinAdapterReal {
//     // TODO this needs to be implemented properly
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         Ok(())
//     }
// }

impl SpellSkiaWinAdapterReal {
    pub fn new(
        pool: Rc<RefCell<SlotPool>>,
        primary_slot: RefCell<Slot>,
        width: u32,
        height: u32,
    ) -> Rc<Self> {
        let buffer = Rc::new(SkiaSoftwareBufferReal {
            primary_slot,
            pool,
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
            buffer_slint: buffer,
            needs_redraw: Cell::new(true),
        })
    }

    fn draw(&self) -> bool {
        if self.needs_redraw.replace(false) {
            self.renderer.render().unwrap();
            true
        } else {
            false
        }
    }

    pub(crate) fn draw_if_needed(&self) -> bool {
        self.draw()
    }

    pub(crate) fn try_dispatch_event(
        &self,
        event: slint::platform::WindowEvent,
    ) -> Result<(), slint::PlatformError> {
        self.window.try_dispatch_event(event)
    }

    pub(crate) fn refersh_buffer(&self) -> Buffer {
        let width: u32 = self.size.width;
        let height: u32 = self.size.height;
        // self.needs_redraw.set(true);
        self.buffer_slint.refresh_buffer(width, height)
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
