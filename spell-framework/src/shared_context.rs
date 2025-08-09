use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};

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
}

#[derive(Debug)]
pub struct MemoryManager {
    pub wayland_buffer: Buffer,
}

fn get_spell_ingredients(width: u32, height: u32) -> Box<[u8]> {
    let a: u8 = 0xFF;
    // vec![Rgba8Pixel::new(a, 0, 0, 0); width as usize * height as usize].into_boxed_slice()
    vec![a; width as usize * height as usize * 4].into_boxed_slice()
}
