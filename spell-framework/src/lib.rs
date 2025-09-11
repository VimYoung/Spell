#![doc(
    html_logo_url = "https://raw.githubusercontent.com/VimYoung/Spell/main/spell-framework/assets/spell_trans.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/VimYoung/Spell/main/spell-framework/assets/spell_trans.ico"
)]
// #![doc(html_favicon_url = "https://github.com/VimYoung/Spell/blob/bb01ae94a365d237ebb0db1df1b6eb37aea25367/spell-framework/assets/Spell.png")]
#![doc = include_str!("../docs/entry.md")]
#[warn(missing_docs)]
mod configure;
mod dbus_window_state;
#[cfg(docsrs)]
mod dummy_skia_docs;
pub mod forge;
#[cfg(feature = "i-slint-renderer-skia")]
// #[cfg(feature = "pam-client2")]
#[cfg(not(docsrs))]
#[doc(hidden)]
mod skia_non_docs;
pub mod slint_adapter;
pub mod vault;
pub mod wayland_adapter;
/// It contains related enums and struct which are used to manage,
/// define and update various properties of a widget(viz a viz layer). You can import necessary
/// types from this module to implement relevant features. See docs of related objects for
/// their overview.
pub mod layer_properties {
    pub use crate::{
        configure::WindowConf,
        dbus_window_state::{DataType, ForeignController},
    };
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::KeyboardInteractivity as BoardType;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
}
use dbus_window_state::{ForeignController, InternalHandle, deploy_zbus_service};
use smithay_client_toolkit::reexports::calloop::channel::{Channel, Event, channel};
use std::{
    cell::RefCell,
    error::Error,
    rc::Rc,
    sync::{Arc, RwLock},
    time::Duration,
};
// use tokio::sync::mpsc;
use wayland_adapter::SpellWin;

use zbus::Error as BusError;

type State = Arc<RwLock<Box<dyn ForeignController>>>;

pub fn enchant_spells<F>(
    mut waywindows: Vec<SpellWin>,
    states: Vec<Option<State>>,
    mut set_callbacks: Vec<Option<F>>,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>) + 'static,
{
    if waywindows.len() == states.len() && waywindows.len() == set_callbacks.len() {
        let mut internal_recievers: Vec<Channel<InternalHandle>> = Vec::new();
        states.iter().enumerate().for_each(|(index, state)| {
            internal_recievers.push(helper_fn_for_deploy(
                waywindows[index].layer_name.clone(),
                state,
            ));
        });
        states.into_iter().enumerate().for_each(|(index, state)| {
            let set_call = set_callbacks.remove(0);
            if let Some(mut callback) = set_call {
                let event_loop = waywindows[index].event_loop.clone();
                event_loop
                    .borrow()
                    .handle()
                    .insert_source(
                        internal_recievers.remove(0),
                        move |event_msg, _, state_data| match event_msg {
                            Event::Msg(int_handle) => {
                                match int_handle {
                                    InternalHandle::StateValChange((key, data_type)) => {
                                        println!("Inside statechange");
                                        //Glad I could think of this sub scope for RwLock.
                                        {
                                            let mut state_inst =
                                                state.as_ref().unwrap().write().unwrap();
                                            state_inst.change_val(&key, data_type);
                                        }
                                        callback(state.as_ref().unwrap().clone());
                                    }
                                    InternalHandle::ShowWinAgain => {
                                        state_data.show_again();
                                    }
                                    InternalHandle::HideWindow => {
                                        state_data.hide();
                                        println!("Hide called");
                                    }
                                }
                            }
                            // TODO have to handle it properly.
                            Event::Closed => {}
                        },
                    )
                    .unwrap();
            }
        });

        loop {
            waywindows.iter_mut().for_each(|waywindow| {
                let event_loop = waywindow.event_loop.clone();
                event_loop
                    .borrow_mut()
                    .dispatch(Duration::from_millis(1), waywindow)
                    .unwrap();
            });
        }
    } else {
        panic!(
            "The lengths of given vectors are not equal. \n Make sure that given vector lengths are equal"
        );
    }
}

pub fn cast_spell<
    S: SpellAssociated,
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>) + 'static,
>(
    mut waywindow: S,
    state: Option<State>,
    set_callback: Option<F>,
) -> Result<(), Box<dyn Error>> {
    waywindow.on_call(state, set_callback)
}

fn helper_fn_for_deploy(
    layer_name: String,
    state: &Option<Arc<RwLock<Box<dyn ForeignController>>>>,
) -> Channel<InternalHandle> {
    let (tx, rx) = channel::<InternalHandle>();
    if let Some(some_state) = state {
        let state_clone = some_state.clone();
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
                panic!("All code panicked");
            }
        });
    }
    rx
}

pub trait SpellAssociated {
    fn on_call<F>(
        &mut self,
        state: Option<Arc<RwLock<Box<dyn ForeignController>>>>,
        set_callback: Option<F>,
    ) -> Result<(), Box<dyn Error>>
    where
        F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>) + 'static;
}

// This function has become redundent as event_queue is internally being managed by callloop.
// fn report_error(error_value: DispatchError) {
//     match error_value {
//         DispatchError::Backend(backend) => match backend {
//             backend::WaylandError::Io(std_error) => panic!("{}", std_error),
//             backend::WaylandError::Protocol(protocol) => {
//                 if protocol.code == 2 && protocol.object_id == 6 {
//                     panic!("Maybe the width or height zero");
//                 }
//             }
//         },
//         DispatchError::BadMessage {
//             sender_id,
//             interface,
//             opcode,
//         } => panic!("BadMessage Error: sender: {sender_id} interface: {interface} opcode {opcode}"),
//     }
// }
// TODO it is necessary to call join unwrap on spawned threads to ensure
// that they are closed when main thread closes.
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
// A Bug effects multi widget setup where is invoke_callback is called, first draw
// keeps on drawing on the closed window, can only be debugged after window wise logs
// are enabled. example is saved in a bin file called bug_multi.rs
// TODO to check what will happen to my dbus network if windows with same layer name will be
// present. To check causes for errors as well as before implenenting muliple layers in same
// window.
// TODO lock screen behaviour in a multi-monitor setup needs to be tested.
// TODO merge cast_Spell with run_lock after implementing calloop in normal windows.
