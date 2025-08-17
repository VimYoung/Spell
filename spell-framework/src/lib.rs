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
mod shared_context;
#[cfg(feature = "i-slint-renderer-skia")]
#[doc(hidden)]
#[cfg(not(docsrs))]
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
    pub use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::KeyboardInteractivity as BoardType;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
}

use dbus_window_state::{ForeignController, InternalHandle, deploy_zbus_service};
use smithay_client_toolkit::reexports::client::{DispatchError, backend};
use std::{
    error::Error,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;
use wayland_adapter::SpellWin;

use zbus::Error as BusError;

/// It is a enum which is passed over to the sender for invoking wayland specific
/// method calls.
#[derive(Debug)]
pub enum Handle {
    /// Internally calls [`wayland_adapter::SpellWin::hide`]
    HideWindow,
    /// Internally calls [`wayland_adapter::SpellWin::show_again`]
    ShowWinAgain,
    /// Internally calls [`wayland_adapter::SpellWin::toggle`]
    ToggleWindow,
    /// Internally calls [`wayland_adapter::SpellWin::grab_focus`]
    GrabKeyboardFocus,
    /// Internally calls [`wayland_adapter::SpellWin::remove_focus`]
    RemoveKeyboardFocus,
    /// Internally calls [`wayland_adapter::SpellWin::add_input_region`]
    AddInputRegion(i32, i32, i32, i32),
    /// Internally calls [`wayland_adapter::SpellWin::subtract_input_region`]
    SubtractInputRegion(i32, i32, i32, i32),
    /// Internally calls [`wayland_adapter::SpellWin::add_opaque_region`]
    AddOpaqueRegion(i32, i32, i32, i32),
    /// Internally calls [`wayland_adapter::SpellWin::subtract_opaque_region`]
    SubtractOpaqueRegion(i32, i32, i32, i32),
}

type State = Arc<RwLock<Box<dyn ForeignController>>>;

pub fn enchant_spells<F>(
    mut waywindows: Vec<SpellWin>,
    // window_handles: Vec<Option<std::sync::mpsc::Receiver<Handle>>>,
    states: Vec<Option<State>>,
    mut set_callbacks: Vec<Option<F>>,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>),
{
    if waywindows.len() == states.len() && waywindows.len() == set_callbacks.len() {
        let mut internal_recievers: Vec<mpsc::Receiver<InternalHandle>> = Vec::new();
        states.iter().enumerate().for_each(|(index, state)| {
            internal_recievers.push(helper_fn_for_deploy(
                waywindows[index].layer_name.clone(),
                state,
            ));
        });

        loop {
            states.iter().enumerate().for_each(|(index, state)| {
                helper_fn_internal_handle(
                    state,
                    &mut set_callbacks[index],
                    &mut internal_recievers[index],
                    &mut waywindows[index],
                );
            });
            waywindows.iter_mut().for_each(|win| {
                helper_fn_win_handle(win);
            });

            let _: Vec<_> = waywindows
                .iter_mut()
                .map(|waywindow| {
                    run_loop_once(waywindow);
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
    // mut event_queue: EventQueue<SpellWinInternal>,
    // window_handle_option: Option<std::sync::mpsc::Receiver<Handle>>,
    state: Option<Arc<RwLock<Box<dyn ForeignController>>>>,
    mut set_callback: Option<F>,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>),
{
    let mut rx = helper_fn_for_deploy(waywindow.layer_name.clone(), &state);
    loop {
        helper_fn_internal_handle(&state, &mut set_callback, &mut rx, &mut waywindow);
        helper_fn_win_handle(&mut waywindow);
        run_loop_once(&mut waywindow);
    }
}

fn helper_fn_internal_handle<F>(
    state: &Option<Arc<RwLock<Box<dyn ForeignController>>>>,
    set_callback: &mut Option<F>,
    rx: &mut mpsc::Receiver<InternalHandle>,
    waywindow: &mut SpellWin,
) where
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>),
{
    if let Some(callback) = set_callback
        && state.is_some()
        && let Ok(int_handle) = rx.try_recv()
    {
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
    };
}

fn helper_fn_for_deploy(
    layer_name: String,
    state: &Option<Arc<RwLock<Box<dyn ForeignController>>>>,
) -> mpsc::Receiver<InternalHandle> {
    let (tx, rx) = mpsc::channel::<InternalHandle>(20);
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

fn helper_fn_win_handle(waywindow: &mut SpellWin) {
    if let Some(window_handle) = &waywindow.handler
        && let Ok(handle) = window_handle.try_recv()
    {
        match handle {
            Handle::HideWindow => waywindow.hide(),
            Handle::ShowWinAgain => waywindow.show_again(),
            Handle::ToggleWindow => waywindow.toggle(),
            Handle::GrabKeyboardFocus => waywindow.grab_focus(),
            Handle::RemoveKeyboardFocus => waywindow.remove_focus(),
            Handle::AddInputRegion(x, y, width, height) => {
                waywindow.add_input_region(x, y, width, height);
            }
            Handle::SubtractInputRegion(x, y, width, height) => {
                waywindow.subtract_input_region(x, y, width, height);
            }
            Handle::AddOpaqueRegion(x, y, width, height) => {
                waywindow.add_opaque_region(x, y, width, height);
            }
            Handle::SubtractOpaqueRegion(x, y, width, height) => {
                waywindow.subtract_opaque_region(x, y, width, height);
            }
        }
    }
}

fn run_loop_once(waywindow: &mut SpellWin) {
    let is_first_config = waywindow.first_configure;
    let queue = waywindow.queue.clone();
    if is_first_config {
        // Primary erros are handled here in the first configration itself.
        if let Err(err_value) = queue.borrow_mut().roundtrip(waywindow) {
            report_error(err_value);
        }
        // event_queue.roundtrip(&mut waywindow).unwrap();
    } else {
        if let Err(err_val) = queue.borrow_mut().blocking_dispatch(waywindow) {
            panic!("{}", err_val);
        }
        // waywindow.queue.borrow().flush().unwrap();
        // queue.borrow_mut().dispatch_pending(waywindow).unwrap();
        // if let Some(read_value) = waywindow.queue.borrow().prepare_read() {
        //     let _ = read_value.read();
        // }
    }
}

fn report_error(error_value: DispatchError) {
    match error_value {
        DispatchError::Backend(backend) => match backend {
            backend::WaylandError::Io(std_error) => panic!("{}", std_error),
            backend::WaylandError::Protocol(protocol) => {
                if protocol.code == 2 && protocol.object_id == 6 {
                    panic!("Maybe the width or height zero");
                }
            }
        },
        DispatchError::BadMessage {
            sender_id,
            interface,
            opcode,
        } => panic!("BadMessage Error: sender: {sender_id} interface: {interface} opcode {opcode}"),
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
// TODO to check what will happen to my dbus network if windows with same layer name will be
// present. To check causes for errors as well as before implenenting muliple layers in same
// window.
// TODO lock screen behaviour in a multi-monitor setup needs to be tested.
