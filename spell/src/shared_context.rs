use i_slint_renderer_skia::{
    skia_safe::{self, ColorType},
    software_surface::RenderBuffer,
};
use slint::{PhysicalSize, Window};
use smithay_client_toolkit::shm::{
    Shm,
    slot::{Buffer, SlotPool},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct SharedCore {
    pub primary_buffer: Box<[u8]>,
    pub secondary_buffer: Box<[u8]>,
}

impl SharedCore {
    pub fn new(width: u32, height: u32) -> Self {
        SharedCore {
            primary_buffer: get_spell_ingredients(width, height),
            secondary_buffer: get_spell_ingredients(width, height),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.primary_buffer = get_spell_ingredients(width, height);
        self.secondary_buffer = get_spell_ingredients(width, height);
    }
}

#[derive(Debug)]
pub struct MemoryManager {
    pub shm: Shm,
    pub pool: SlotPool,
    pub wayland_buffer: Buffer,
}

pub struct SkiaSoftwareBuffer {
    pub core: Rc<RefCell<SharedCore>>,
    pub last_dirty_region: RefCell<Option<i_slint_core::item_rendering::DirtyRegion>>,
}

#[allow(unused_variables)]
impl RenderBuffer for SkiaSoftwareBuffer {
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

fn get_spell_ingredients(width: u32, height: u32) -> Box<[u8]> {
    let a: u8 = 0xFF;
    // vec![Rgba8Pixel::new(a, 0, 0, 0); width as usize * height as usize].into_boxed_slice()
    vec![a; width as usize * height as usize * 4].into_boxed_slice()
}
