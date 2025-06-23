use crate::{shared_context::SharedCore, slint_adapter::SpellSkiaWinAdapter};
use slint::platform::software_renderer::TargetPixel;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
pub struct Rgba8Pixel {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgba8Pixel {
    pub fn new(a: u8, r: u8, g: u8, b: u8) -> Self {
        Rgba8Pixel { a, r, g, b }
    }
}

impl TargetPixel for Rgba8Pixel {
    fn blend(&mut self, color: slint::platform::software_renderer::PremultipliedRgbaColor) {
        let a: u16 = (u8::MAX - color.alpha) as u16;
        // self.a = a as u8;
        let out_a = color.alpha as u16 + (self.a as u16 * (255 - color.alpha) as u16) / 255;
        self.a = out_a as u8;
        self.r = (self.r as u16 * a / 255) as u8 + color.red;
        self.g = (self.g as u16 * a / 255) as u8 + color.green;
        self.b = (self.b as u16 * a / 255) as u8 + color.blue;
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        let a = 0xFF;
        Self::new(a, red, green, blue)
    }

    fn background() -> Self {
        // TODO This needs to be decided to see how it should be 0xFF or 0x00;
        // I think there is a bug in slint which is causing the leak of This
        // value.
        let a: u8 = 0x00;
        Self::new(a, 0, 0, 0)
    }
}

impl std::marker::Copy for Rgba8Pixel {}
impl std::clone::Clone for Rgba8Pixel {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct WindowConf {
    pub width: u32,
    pub height: u32,
    pub anchor: (Option<Anchor>, Option<Anchor>),
    pub margin: (i32, i32, i32, i32),
    pub layer_type: Layer,
    pub shared_core: Rc<RefCell<SharedCore>>,
    pub adapter: Rc<SpellSkiaWinAdapter>,
    pub exclusive_zone: bool,
}

impl WindowConf {
    pub fn new(
        width: u32,
        height: u32,
        anchor: (Option<Anchor>, Option<Anchor>),
        margin: (i32, i32, i32, i32),
        layer_type: Layer,
        shared_core: Rc<RefCell<SharedCore>>,
        adapter: Rc<SpellSkiaWinAdapter>,
        exclusive_zone: bool,
    ) -> Self {
        WindowConf {
            width,
            height,
            anchor,
            margin,
            layer_type,
            shared_core,
            adapter,
            exclusive_zone,
        }
    }
}
