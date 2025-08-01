use slint::{PhysicalSize, platform::WindowAdapter};
use std::rc::Rc;
/// It is the main struct handling the rendering of pixels in the wayland window. It implements slint's
/// [WindowAdapter](https://docs.rs/slint/latest/slint/platform/trait.WindowAdapter.html) trait.
/// It is used internally by [SpellMultiWinHandler] and previously by [SpellLayerShell]. This
/// adapter internally uses [Skia](https://skia.org/) 2D graphics library for rendering.
pub struct SpellSkiaWinAdapterDummy {
    pub(crate) window: bool,
    pub(crate) size: bool,
    pub(crate) renderer: bool,
    pub(crate) needs_redraw: bool,
}

impl WindowAdapter for SpellSkiaWinAdapterDummy {
    fn window(&self) -> &slint::Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        self.size
    }

    fn renderer(&self) -> &dyn slint::platform::Renderer {
        &self.renderer
    }

    fn request_redraw(&self) {}
}

impl std::fmt::Debug for SpellSkiaWinAdapterDummy {
    // TODO this needs to be implemented properly
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl SpellSkiaWinAdapterDummy {
    pub fn new(shared_core: bool, width: u32, height: u32) -> Rc<Self> {}

    pub fn draw(&self) -> bool {}
}
