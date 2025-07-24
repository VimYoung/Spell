#![doc = include_str!("../docs/entry.md")]
// #[warn(missing_docs)]
mod configure;
mod dbus_window_state;
pub mod forge;
mod shared_context;
pub mod slint_adapter;
pub mod vault;
pub mod wayland_adapter;
pub mod layer_properties {
    pub use crate::{
        configure::WindowConf,
        // shared_context::SharedCore,
        dbus_window_state::{DataType, ForeignController},
    };
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::KeyboardInteractivity as BoardType;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
    pub use zbus::fdo::Error as BusError;
}

use dbus_window_state::{ForeignController, InternalHandle, deploy_zbus_service};
use smithay_client_toolkit::reexports::client::EventQueue;
use std::{
    error::Error,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;
use wayland_adapter::SpellWin;

use zbus::Error as BusError;

pub enum Handle {
    HideWindow,
    ShowWinAgain,
    ToggleWindow,
}

pub fn enchant_spells<F>(
    mut waywindows: Vec<(SpellWin, EventQueue<SpellWin>)>,
    window_handles: Vec<Option<std::sync::mpsc::Receiver<Handle>>>,
    states: Vec<Option<Arc<RwLock<Box<dyn ForeignController>>>>>,
    mut set_callbacks: Vec<Option<F>>,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>),
{
    if window_handles.len() == waywindows.len()
        && window_handles.len() == states.len()
        && window_handles.len() == set_callbacks.len()
    {
        let mut internal_recievers: Vec<mpsc::Receiver<InternalHandle>> = Vec::new();
        states.iter().enumerate().for_each(|(index, state)| {
            let (tx, rx) = mpsc::channel::<InternalHandle>(20);
            if let Some(some_state) = state {
                let state_clone = some_state.clone();
                let layer_name = waywindows[index].0.layer_name.clone();
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    // TODO unwrap needs to be handled here.

                    // TOTHINK result not handled as value this runnin indefinetly.
                    if let Err(error) = rt.block_on(async move {
                        println!("deploied zbus serive in thread");
                        deploy_zbus_service(state_clone, tx, layer_name).await?;
                        Ok::<_, BusError>(())
                    }) {
                        println!("Dbus Thread panicked witth the following error. \n {error}");
                        panic!("All code panicked");
                    }
                });
            }
            internal_recievers.push(rx);
        });

        loop {
            states.iter().enumerate().for_each(|(index, state)| {
                if let Some(ref mut callback) = set_callbacks[index]
                    && state.is_some()
                {
                    if let Ok(int_handle) = internal_recievers[index].try_recv() {
                        match int_handle {
                            InternalHandle::StateValChange((key, data_type)) => {
                                println!("Inside statechange");
                                //Glad I could think of this sub scope for RwLock.
                                {
                                    let mut state_inst = state.as_ref().unwrap().write().unwrap();
                                    state_inst.change_val(&key, data_type);
                                }
                                callback(state.as_ref().unwrap().clone());
                            }
                            InternalHandle::ShowWinAgain => {
                                waywindows[index].0.show_again();
                            }
                            InternalHandle::HideWindow => waywindows[index].0.hide(),
                        }
                    }
                };
            });
            window_handles
                .iter()
                .enumerate()
                .for_each(|(index, window_handle_option)| {
                    if let Some(window_handle) = window_handle_option {
                        if let Ok(handle) = window_handle.try_recv() {
                            match handle {
                                Handle::HideWindow => waywindows[index].0.hide(),
                                Handle::ShowWinAgain => waywindows[index].0.show_again(),
                                Handle::ToggleWindow => waywindows[index].0.toggle(),
                            }
                        }
                    }
                });

            let _: Vec<_> = waywindows
                .iter_mut()
                .map(|(waywindow, event_queue)| {
                    let is_first_config = waywindow.first_configure;
                    if is_first_config {
                        event_queue.roundtrip(waywindow).unwrap();
                    } else {
                        event_queue.flush().unwrap();
                        event_queue.dispatch_pending(waywindow).unwrap();
                        if let Some(read_value) = event_queue.prepare_read() {
                            let _ = read_value.read();
                        }
                    }
                })
                .collect();
        }
    } else {
        panic!(
            "The lengths of given vectors are not equal. \n Make sure that given vector lengths are equal"
        );
    }
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
    let (tx, mut rx) = mpsc::channel::<InternalHandle>(20);
    if let Some(ref some_state) = state {
        let state_clone = some_state.clone();
        let layer_name = waywindow.layer_name.clone();
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            if let Err(error) = rt.block_on(async move {
                println!("deploied zbus serive in thread");
                deploy_zbus_service(state_clone, tx, layer_name).await?;
                Ok::<_, BusError>(())
            }) {
                println!("Dbus Thread panicked witth the following error. \n {error}");
                panic!("DBus thread panicked");
            }
        });
    }

    loop {
        if let Some(ref mut callback) = set_callback
            && state.is_some()
        {
            if let Ok(int_handle) = rx.try_recv() {
                match int_handle {
                    InternalHandle::StateValChange((key, data_type)) => {
                        println!("Inside statechange");
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
                Handle::ToggleWindow => waywindow.toggle(),
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
// TODO scroll action is not implemented in Pointer touch event.
// A Bug effects multi widget setup where is invoke_callback is called, first draw
// keeps on drawing on the closed window, can only be debugged after window wise logs
// are enabled. example is saved in a bin file called bug_multi.rs
