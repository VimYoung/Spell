use slint::{PhysicalSize, SharedString, platform::WindowEvent};
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::EventLoop,
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, EventQueue, QueueHandle,
            protocol::{wl_output, wl_seat, wl_shm, wl_surface},
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyboardHandler, Keysym},
    },
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockSurface, SessionLockSurfaceConfigure,
    },
    shm::{
        Shm, ShmHandler,
        slot::{Buffer, SlotPool},
    },
};
use std::{cell::RefCell, error::Error, rc::Rc, time::Duration};

use crate::{
    configure::LayerConf,
    slint_adapter::{SpellMultiWinHandler, SpellSkiaWinAdapter},
    wayland_adapter::{SpellLock, way_helper::get_string},
};

impl ProvidesRegistryState for SpellLock {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState,];
}

impl ShmHandler for SpellLock {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl OutputHandler for SpellLock {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        println!("New Output Source Added");
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        println!("Output is destroyed");
    }
}

impl CompositorHandler for SpellLock {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.converter_lock(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        println!("Surface reentered");
        // Not needed for this example.
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        println!("Surface left");
    }
}

impl SessionLockHandler for SpellLock {
    fn locked(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {}

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        println!("Session could not be locked");
        self.is_locked = true;
    }
    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: SessionLockSurface,
        _configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        self.converter_lock(qh);
    }
}

pub fn run_lock(
    mut lock: SpellLock,
    mut event_loop: EventLoop<SpellLock>,
    event_queue: EventQueue<SpellLock>,
) -> Result<(), Box<dyn Error>> {
    WaylandSource::new(lock.conn.clone(), event_queue)
        .insert(lock.loop_handle.clone())
        .unwrap();

    while lock.is_locked {
        event_loop
            .dispatch(Duration::from_millis(1), &mut lock)
            .unwrap();

        // if lock.is_locked {
        //     break;
        // }
    }
    Ok(())
}

impl KeyboardHandler for SpellLock {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
        println!("Keyboard focus entered");
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _surface: &smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        _serial: u32,
    ) {
        println!("Keyboard focus left");
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        println!("A key is pressed");
        let string_val: SharedString = get_string(event);
        println!("Value of key: {:?}", string_val);
        self.slint_part.as_ref().unwrap().adapters[0]
            .try_dispatch_event(WindowEvent::KeyPressed { text: string_val })
            .unwrap();
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        mut event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        println!("A key is released");
        let key_sym = Keysym::new(event.raw_code);
        event.keysym = key_sym;
        let string_val: SharedString = get_string(event);
        self.slint_part.as_ref().unwrap().adapters[0]
            .try_dispatch_event(WindowEvent::KeyReleased { text: string_val })
            .unwrap();
        // let value = event.keysym.key_char();
        // if let Some(val) = value {
        //     println!("Value getting out :{}", val);
        //     self.adapter
        //         .try_dispatch_event(WindowEvent::KeyReleased {
        //             text: SharedString::from(val /*event.keysym.key_char().unwrap()*/),
        //         })
        //         .unwrap();
        // }
    }

    // TODO needs to be implemented to enable functionalities of ctl, shift, alt etc.
    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        _raw_modifiers: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _layout: u32,
    ) {
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
    }
    // TODO This method needs to be implemented after the looping mecha is changed to calloop.
    fn update_repeat_info(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _info: smithay_client_toolkit::seat::keyboard::RepeatInfo,
    ) {
    }
}

impl SeatHandler for SpellLock {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard_state.board.is_none() {
            println!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            // let keyboard_data = KeyboardData::new(seat);
            self.keyboard_state.board = Some(keyboard);
            // TODO keyboard Data needs to be set later.
            // self.keyboard_state.board_data = Some(keyboard_data::<Self>);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard_state.board.is_some() {
            println!("Unset keyboard capability");
            self.keyboard_state.board.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

/// It is an internal struct used by [`SpellLock`] internally. After the window is initialised,
/// this struct's [build](`SpellSlintLock::build`) is used to complete the lock making process.
pub struct SpellSlintLock {
    pub(crate) adapters: Vec<Rc<SpellSkiaWinAdapter>>,
    pub(crate) size: Vec<PhysicalSize>,
    pub(crate) wayland_buffer: Vec<Buffer>,
}

// impl SpellSlintLock {
//     /// Used to complete the lock making process after the initialisation of your slint windows.
//     /// This method takes SpellLock's mutable reference along with the window handler to complete
//     /// the lock creation process by setting appropriate slint properties.
//     pub fn build(spell_lock: &mut SpellLock, multi_handler: Rc<RefCell<SpellMultiWinHandler>>) {
//         let adapter_length = multi_handler.borrow().adapter.len();
//         let window_length = multi_handler.borrow().windows.len();
//         if adapter_length == window_length {
//             let sizes: Vec<PhysicalSize> = multi_handler
//                 .borrow()
//                 .windows
//                 .iter()
//                 .map(|(_, conf)| {
//                     if let LayerConf::Lock(width, height) = conf {
//                         PhysicalSize {
//                             width: *width,
//                             height: *height,
//                         }
//                     } else {
//                         panic!("Shouldn't enter here");
//                     }
//                 })
//                 .collect();
//             // TODO This hould be passed with the max dimensions of the monitors.
//             let mut pool = SlotPool::new(
//                 (sizes[0].width * sizes[0].height * 4) as usize,
//                 &spell_lock.shm,
//             )
//             .expect("COuldn't create pool");
//
//             let buffers: Vec<Buffer> = sizes
//                 .iter()
//                 .map(|physical_size| {
//                     let stride = physical_size.width as i32 * 4;
//                     let (wayland_buffer, _) = pool
//                         .create_buffer(
//                             physical_size.width as i32,
//                             physical_size.height as i32,
//                             stride,
//                             wl_shm::Format::Argb8888,
//                         )
//                         .expect("Creating Buffer");
//                     wayland_buffer
//                 })
//                 .collect();
//
//             let adapter = multi_handler.borrow().adapter.clone();
//             // spell_lock.slint_part = Some(SpellSlintLock {
//             //     adapter: adapter
//             //         .into_iter()
//             //         .map(|vl| vl as Rc<dyn EventAdapter>)
//             //         .collect(),
//             //     cores: multi_handler.borrow().core.clone(),
//             //     size: sizes,
//             //     wayland_buffer: buffers,
//             // })
//             todo!();
//         } else {
//             panic!("No of initialised window is not equal to no of outputs");
//         }
//     }
// }
