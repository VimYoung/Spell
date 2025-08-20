use crate::shared_context::SharedCore;
#[cfg(not(docsrs))]
use crate::wayland_adapter::SpellLock;
use slint::{PhysicalSize, Window, platform::WindowAdapter};
use std::{
    cell::Cell,
    cell::RefCell,
    process::Command,
    rc::{Rc, Weak},
};

#[cfg(feature = "i-slint-renderer-skia")]
use i_slint_renderer_skia::{
    skia_safe::{self, ColorType},
    software_surface::RenderBuffer,
};

#[cfg(feature = "pam")]
pub use pam_client::Error as PamError;
#[cfg(feature = "pam")]
use pam_client::{Context, Flag, conv_mock::Conversation};

#[cfg(feature = "i-slint-renderer-skia")]
#[cfg(not(docsrs))]
pub struct SkiaSoftwareBufferReal {
    pub core: Rc<RefCell<SharedCore>>,
    pub last_dirty_region: RefCell<Option<i_slint_core::item_rendering::DirtyRegion>>,
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
        println!("This trait implementation of RenderBuffer is Run");
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
        let native_buffer = &mut self.core.borrow_mut().primary_buffer;

        let bytes = bytemuck::cast_slice_mut(native_buffer);
        *self.last_dirty_region.borrow_mut() =
            render_callback(width, height, skia_safe::ColorType::BGRA8888, 1, bytes).unwrap();

        // *self.pixels.borrow_mut() = shared_pixel_buffer;

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

impl std::fmt::Debug for SpellSkiaWinAdapterReal {
    // TODO this needs to be implemented properly
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl SpellSkiaWinAdapterReal {
    pub fn new(shared_core: Rc<RefCell<SharedCore>>, width: u32, height: u32) -> Rc<Self> {
        let buffer = Rc::new(SkiaSoftwareBufferReal {
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
            renderer,
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

#[cfg(feature = "pam")]
pub fn unlock(
    mut lock: &mut SpellLock,
    username: Option<&str>,
    password: &str,
) -> Result<(), PamError> {
    let mut user_name = String::new();
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

    let mut context = Context::new(
        "login", // Service name
        None,
        Conversation::with_credentials(&user_name, password),
    )?;
    context.authenticate(Flag::NONE)?;
    context.acct_mgmt(Flag::NONE)?;

    if let Some(locked_val) = lock.session_lock.take() {
        locked_val.unlock();
    }
    lock.is_locked = false;
    lock.conn.roundtrip().unwrap();

    Ok(())
}
// #[cfg(docsrs)]
// use crate::dummy_skia_docs;
//
// #[cfg(feature = "i-slint-renderer-skia")]
// #[cfg(not(docsrs))]
// use crate::skia_non_docs::SkiaSoftwareBufferReal;
//
// #[cfg(not(docsrs))]
// pub type SkiaSoftwareBuffer = SkiaSoftwareBufferReal;
//
// #[cfg(docsrs)]
// pub type SkiaSoftwareBuffer = SkiaSoftwareBufferDummy;
