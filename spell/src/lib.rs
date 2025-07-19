#![doc = include_str!("../docs/entry.md")]
// #[warn(missing_docs)]
mod configure;
mod shared_context;
mod slint_adapter;
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
    ShowWinAgain,
}

pub fn cast_spell<F>(
    mut waywindow: SpellWin,
    mut event_queue: EventQueue<SpellWin>,
    window_handle: std::sync::mpsc::Receiver<Handle>,
    state: Option<Arc<RwLock<Box<dyn ForeignController>>>>,
    mut set_callback: Option<F>,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>),
{
    // TODO I don't know but seems like 5 would be a good size given the low size.
    let (tx, mut rx) = mpsc::channel::<InternalHandle>(20);
    if let Some(ref some_state) = state {
        let state_clone = some_state.clone();
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
    }

    loop {
        if let Some(ref mut callback) = set_callback
            && state.is_some()
        {
            if let Ok(int_handle) = rx.try_recv() {
                match int_handle {
                    InternalHandle::StateValChange((key, data_type)) => {
                        println!("INside statechange");
                        //Glad I could think of this sub scope for RwLock.
                        {
                            let mut state_inst = state.as_ref().unwrap().write().unwrap();
                            state_inst.change_val(&key, data_type);
                        }
                        callback(state.as_ref().unwrap().clone());
                    }
                    InternalHandle::ShowWinAgain => {
                        waywindow.show_again();
                    }
                    InternalHandle::HideWindow => waywindow.hide(),
                }
            }
        };

        if let Ok(handle) = window_handle.try_recv() {
            match handle {
                Handle::HideWindow => waywindow.hide(),
                Handle::ShowWinAgain => waywindow.show_again(),
            }
        }

        let is_first_config = waywindow.first_configure;
        if is_first_config {
            event_queue.roundtrip(&mut waywindow).unwrap();
        } else {
            event_queue.flush().unwrap();
            event_queue.dispatch_pending(&mut waywindow).unwrap();
            if let Some(read_value) = event_queue.prepare_read() {
                let _ = read_value.read();
            }
        }
    }
}
// TODO it is necessary to call join unwrap on spawned threads to ensure
// that they are closed when main thread closes.
// TODO make the state and callback optional, so that if someone don't want to
// implement state, they wouldn't.
// TODO see if the above loop can be replaced by callloop for better idomicity and
// performance in any sense.
// TODO The project should have a live preview feature. It can be made by leveraging
// slint's preview and moving the output of debug to spell_cli.
// TODO linux's DNF Buffers needs to be used to improve rendering and avoid conversions
// from CPU to GPU and vice versa.
// Replace the expect statements in the code with tracing statements.
// TODO needs to have multi monitor support.
// TO REMEMBER I removed dirty region from spellskiawinadapter but it can be added
// if I want to make use of the dirty region information to strengthen my rendering.
