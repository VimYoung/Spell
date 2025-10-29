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
    pub use crate::{configure::WindowConf, dbus_window_state::DataType};
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::KeyboardInteractivity as BoardType;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
}
use dbus_window_state::{DataType, InternalHandle, deploy_zbus_service};
use smithay_client_toolkit::reexports::calloop::channel::{Channel, Event, channel};
use std::{
    any::Any,
    error::Error,
    sync::{Arc, RwLock},
    time::Duration,
};

use tracing::{Level, error, info, instrument, span, trace, warn};
use wayland_adapter::SpellWin;
use zbus::Error as BusError;

/// This a boilerplate trait for connection with CLI, it will be replaced by a procedural
/// macro in the future.
/// In the mean time, this function is implemented on a struct you would define in
/// your `.slint` file. Then state of widgets should be stored as single property
/// of that data type rather than on individual values.
///
/// ## Example
///
/// ```slint
/// // Wrong Method is you want to handle on-close and is-expanded locally.
/// export component MyWindow inherits Window {
///   in-out property <bool> on-close: false;
///   in-out property <bool> is-expanded: true;
///   Rectangle {
///      // Other widgets will come here.
///   }
/// }
///
/// // Correct method
/// export component MyWindow inherits Window {
///   in-out property <MyWinState> state: {on-close: false, is-expanded: true};
///   Rectangle {
///      // Other widgets will come here.
///   }
/// }
/// export struct MyWinState {
///   on-close: bool,
///   is-expanded: true,
/// }
/// ```
pub trait ForeignController: Send + Sync + std::fmt::Debug {
    /// On calling `spell-cli -l layer_name look
    /// var_name`, the cli calls `get_type` method of the trait with `var_name` as input.
    fn get_type(&self, key: &str) -> DataType;
    /// It is called on `spell-cli -l layer_name update key value`. `as_any` is for syncing the changes
    /// internally for now and need not be implemented by the end user.
    fn change_val(&mut self, key: &str, val: DataType);
    /// It is a type needed internally, it's implementation should return `self` to
    /// avoid undefined behaviour.
    fn as_any(&self) -> &dyn Any;
}

type State = Arc<RwLock<dyn ForeignController>>;
type States = Vec<Option<Box<dyn FnMut(State)>>>;
/// This is the event loop which is to be called when initialising multiple windows through
/// a single `main` file. It is important to remember that Each value of these vectors corresponds
/// to the number on which a widget is initialised. So, this function will panic if the length of
/// vectors of various types mentioned here are not equal.For more information on checking the
/// arguments, view [cast_spell].
pub fn enchant_spells(
    mut waywindows: Vec<SpellWin>,
    states: Vec<Option<State>>,
    mut set_callbacks: States,
) -> Result<(), Box<dyn Error>> {
    if waywindows.len() == states.len() && waywindows.len() == set_callbacks.len() {
        info!("Starting windows");
        let spans: Vec<span::Span> = waywindows.iter().map(|win| win.span.clone()).collect();
        let mut internal_recievers: Vec<Channel<InternalHandle>> = Vec::new();
        states.iter().enumerate().for_each(|(index, state)| {
            internal_recievers.push(helper_fn_for_deploy(
                waywindows[index].layer_name.clone(),
                state,
                waywindows[index].span.clone(),
            ));
            trace!("{:?}", &waywindows[index]);
        });
        trace!("Grabbed Internal recievers");
        states.into_iter().enumerate().for_each(|(index, state)| {
            let _guard = spans[index].enter();
            let set_call = set_callbacks.remove(0);
            if let Some(mut callback) = set_call {
                let event_loop = waywindows[index].event_loop.clone();
                event_loop
                    .borrow()
                    .handle()
                    .insert_source(
                        internal_recievers.remove(0),
                        move |event_msg, _, state_data| {
                            match event_msg {
                                Event::Msg(int_handle) => {
                                    match int_handle {
                                        InternalHandle::StateValChange((key, data_type)) => {
                                            trace!("Internal variable change called");
                                            //Glad I could think of this sub scope for RwLock.
                                            {
                                                let mut state_inst =
                                                    state.as_ref().unwrap().write().unwrap();
                                                state_inst.change_val(&key, data_type);
                                            }
                                            callback(state.as_ref().unwrap().clone());
                                        }
                                        InternalHandle::ShowWinAgain => {
                                            trace!("Internal show Called");
                                            state_data.show_again();
                                        }
                                        InternalHandle::HideWindow => {
                                            trace!("Internal hide called");
                                            state_data.hide();
                                        }
                                    }
                                }
                                // TODO have to handle it properly.
                                Event::Closed => {
                                    info!("Internal Channel closed");
                                }
                            }
                        },
                    )
                    .unwrap();
            }
        });
        trace!("Setting internal handles as events and calling event loop.");

        loop {
            for (index, waywindow) in waywindows.iter_mut().enumerate() {
                spans[index].in_scope(|| -> Result<(), Box<dyn Error>> {
                    let event_loop = waywindow.event_loop.clone();
                    event_loop
                        .borrow_mut()
                        .dispatch(Duration::from_millis(1), waywindow)?;
                    Ok(())
                })?;
            }
        }
    } else {
        error!("Lengths are unequal");
        panic!(
            "The lengths of given vectors are not equal. \n Make sure that given vector lengths are equal"
        );
    }
}

/// This is the primary function used for starting the event loop after creating the widgets,
/// setting values and initialising windows. Example of the use can be found [here](https://github.com/VimYoung/Young-Shell/tree/main/src/bin).
/// The function takes in the following function arguments:-
/// 1. Wayland side of widget corresponding to it's slint window.
/// 2. A instance of struct implementing [ForeignController]. This will be wrapped in `Arc` and
///    `RwLock` as it would be used across threads internally, if the widget is static in nature
///    and doesn't need state that needs to be changed remotely via CLI. You can parse in None.
/// 3. A callback which is called when a CLI command is invoked changing the value. The closure
///    gets an updated value of your state struct. The common method is to take the updated value
///    and replace your existing state with it to reflect back the changes in the slint code. If
///    state is provided, then it is important for now to pass a callback corresponding to it too.
///    You can use this callback for example.
/// ```rust
/// move |state_value| {
///     let controller_val = state_value.read().unwrap();
///     let val = controller_val
///         .as_any()
///         .downcast_ref::<State>()
///         .unwrap()
///         .clone();
///     ui_clone.unwrap().set_state(val);
/// }
/// // here `ui_clone` is weak pointer to my slint window for setting back the `state` property.
/// ```
pub fn cast_spell<S: SpellAssociated + std::fmt::Debug>(
    mut waywindow: S,
    state: Option<State>,
    set_callback: Option<Box<dyn FnMut(State)>>,
) -> Result<(), Box<dyn Error>> {
    let span = waywindow.get_span();
    let s = span.clone();
    span.in_scope(|| {
        trace!("{:?}", &waywindow);
        waywindow.on_call(state, set_callback, s)
    })
}

#[instrument(skip(state))]
fn helper_fn_for_deploy(
    layer_name: String,
    state: &Option<State>,
    span_log: span::Span,
) -> Channel<InternalHandle> {
    let (tx, rx) = channel::<InternalHandle>();
    if let Some(some_state) = state {
        let state_clone = some_state.clone();
        std::thread::spawn(move || {
            span_log.in_scope(move || {
                let span_bus = span!(Level::INFO, "Zbus Logs",);
                let _guard = span_bus.enter();
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                if let Err(error) = rt.block_on(async move {
                    trace!("Started Zbus service in a thread");
                    deploy_zbus_service(state_clone, tx, layer_name).await?;
                    Ok::<_, BusError>(())
                }) {
                    error!("Zbus panicked with following error: {}", error);
                    Err(error)
                } else {
                    Ok(())
                }
            })
        });
    }
    rx
}

/// Internal function for running event loops, implemented by [SpellWin] and
/// [SpellLock][`crate::wayland_adapter::SpellLock`].
pub trait SpellAssociated {
    fn on_call(
        &mut self,
        state: Option<State>,
        set_callback: Option<Box<dyn FnMut(State)>>,
        span_log: tracing::span::Span,
    ) -> Result<(), Box<dyn Error>>;

    fn get_span(&self) -> span::Span;
}

// TODO set logging values in Option so that only a single value reads or writes.
// TODO it is necessary to call join unwrap on spawned threads to ensure
// that they are closed when main thread closes.
// TODO The project should have a live preview feature. It can be made by leveraging
// slint's preview and moving the output of debug to spell_cli.
// TODO linux's DNF Buffers needs to be used to improve rendering and avoid conversions
// from CPU to GPU and vice versa.
// TODO needs to have multi monitor support.
// TO REMEMBER I removed dirty region from spellskiawinadapter but it can be added
// if I want to make use of the dirty region information to strengthen my rendering.
// TODO to check what will happen to my dbus network if windows with same layer name will be
// present. To check causes for errors as well as before implenenting muliple layers in same
// window.
// TODO lock screen behaviour in a multi-monitor setup needs to be tested.
// TODO merge cast_Spell with run_lock after implementing calloop in normal windows.
// TODO t add tracing in following functions:
// 1. secondary and primary services
// TODO implement logiing for SpellLock.
// TODO check if the dbus setup is working for more than 2 widgets when one is
// primary and 2 are secondary.
// Provide a method in the macro to disable tracing_subsriber completely for some project
// which want's to do it themselves.
// cast spell macro should be having following values.
// 1. Disable log: should disable setting subscriber, generally for the project to use or for
// someone to set their own.
// 2. forge: provide a forge instance to run independently.
// 3. exclusive_zone: true or false or with specified value.
// 4. it should have the option to take a window_conf or directly the window configurations
// into the macro, removing the need to define it previously.
//
// Also, a procedural macro to mimic the functionalities of ForeignController.
