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
    pub pool: SlotPool,
}

fn get_spell_ingredients(width: u32, height: u32) -> Box<[u8]> {
    let a: u8 = 0xFF;
    // vec![Rgba8Pixel::new(a, 0, 0, 0); width as usize * height as usize].into_boxed_slice()
    vec![a; width as usize * height as usize * 4].into_boxed_slice()
}

// #[derive(Debug)]
// pub struct SpellWin {
//     pub(crate) window: SpellWinInternal,
//     pub(crate) queue: Rc<RefCell<EventQueue<SpellWinInternal>>>,
// }
//
// impl SpellWin {
//     pub fn conjure_spells(
//         windows: Rc<RefCell<SpellMultiWinHandler>>,
//         // current_display_specs: Vec<(usize, usize, usize, usize)>,
//     ) -> Vec<Rc<Self>> {
//         SpellWinInternal::conjure_spells(windows)
//             .into_iter()
//             .map(|(internal, queue)| {
//                 Rc::new(SpellWin {
//                     window: internal,
//                     queue,
//                 })
//             })
//             .collect()
//     }
//
//     pub fn invoke_spell(
//         name: &str,
//         window_conf: WindowConf,
//         // current_display_specs: (usize, usize, usize, usize),
//     ) -> Rc<Self> {
//         let internal = SpellWinInternal::invoke_spell(name, window_conf);
//         Rc::new(SpellWin {
//             window: internal.0,
//             queue: internal.1,
//         })
//     }
//
//     pub fn toggle(&self) {
//         self.window.toggle();
//     }
//
//     pub fn hide(&self) {
//         self.window.hide();
//     }
//
//     pub fn add_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
//         self.window.add_input_region(x, y, width, height);
//     }
//
//     pub fn subtract_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
//         self.window.subtract_input_region(x, y, width, height);
//     }
//
//     pub fn add_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
//         self.window.add_opaque_region(x, y, width, height);
//     }
//
//     pub fn subtract_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
//         self.window.subtract_opaque_region(x, y, width, height);
//     }
//
//     pub fn show_again(&self) {
//         self.window.show_again()
//     }
//
//     pub fn grab_focus(&self) {
//         self.window.grab_focus();
//     }
//     pub fn remove_focus(&self) {
//         self.window.remove_focus();
//     }
// }
