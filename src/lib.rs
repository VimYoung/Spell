mod configure;
pub mod constantvals;
pub mod shared_context;
pub mod slint_adapter;
pub mod wayland_adapter;
pub mod layer_properties {
    pub use crate::{
        configure::WindowConf,
        wayland_adapter::window_state::{DataType, ForeignController},
    };
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
    pub use zbus::fdo::Error as BusError;
}

use smithay_client_toolkit::reexports::client::EventQueue;
use std::{
    error::Error,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;
use wayland_adapter::{
    SpellWin,
    window_state::{ForeignController, InternalHandle, deploy_zbus_service},
};

use zbus::Error as BusError;

pub enum Handle {
    HideWindow,
}

pub fn cast_spell<F>(
    mut waywindow: SpellWin,
    mut event_queue: EventQueue<SpellWin>,
    window_handle: std::sync::mpsc::Receiver<Handle>,
    state: Box<dyn ForeignController>,
    set_callback: &mut F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>),
{
    // TODO I don't know but seems like 5 would be a good size given the low size.
    let (tx, mut rx) = mpsc::channel::<InternalHandle>(20);
    let state = Arc::new(RwLock::new(state));
    let state_clone = state.clone();
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // TODO unwrap needs to be handled here.

        // TOTHINK result not handled as value this runnin indefinetly.
        let _ = rt.block_on(async move {
            println!("deploied zbus serive in thread");
            deploy_zbus_service(state_clone, tx).await?;
            Ok::<_, BusError>(())
        });
    });

    loop {
        if let Ok(int_handle) = rx.try_recv() {
            match int_handle {
                InternalHandle::StateValChange((key, data_type)) => {
                    println!("INside statechange");
                    //Glad I could think of this sub scope for RwLock.
                    {
                        let mut state_inst = state.write().unwrap();
                        state_inst.change_val(&key, data_type);
                    }
                    set_callback(state.clone());
                }
                InternalHandle::ShowWinAgain => {
                    waywindow.show_again();
                }
            }
        };

        if let Ok(handle) = window_handle.try_recv() {
            match handle {
                Handle::HideWindow => waywindow.hide(),
            }
        }

        let is_first_config = waywindow.first_configure;
        if is_first_config {
            event_queue.roundtrip(&mut waywindow).unwrap();
        } else {
            // println!("Running the loop");
            event_queue.flush().unwrap();
            // event_queue.roundtrip(&mut waywindow).unwrap();
            event_queue.dispatch_pending(&mut waywindow).unwrap();
            if let Some(read_value) = event_queue.prepare_read() {
                let _ = read_value.read();
            }
            // event_queue.blocking_dispatch(&mut waywindow).unwrap();
        }
    }
}
// TODO it is necessary to call join unwrap on spawned threads to ensure
// that they are closed when main thread closes.
// TODO make the state and callback optional, so that if someone don't want to
// implement state, they wouldn't.
// TODO see if the above loop can be replaced by callloop for better idomicity and
// performance in any sense.

pub fn get_spell_ingredients(width: u32, height: u32) -> Box<[u8]> {
    let a: u8 = 0xFF;
    // vec![Rgba8Pixel::new(a, 0, 0, 0); width as usize * height as usize].into_boxed_slice()
    vec![a; width as usize * height as usize * 4].into_boxed_slice()
}
