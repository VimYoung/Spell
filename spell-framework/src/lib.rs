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
pub use paste;
pub use smithay_client_toolkit;
use smithay_client_toolkit::reexports::calloop::channel::{Channel, Event, channel};
use std::{
    any::Any,
    error::Error,
    sync::{Arc, RwLock},
    time::Duration,
};
pub use tracing;
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

pub type State = Arc<RwLock<dyn ForeignController>>;
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

    fn get_span(&self) -> span::Span {
        span!(Level::INFO, "unnamed-widget")
    }
}

/// Experimental code not ready for end use
#[macro_export]
macro_rules! invoke_spell {
    ($slint_win:ty, $name:expr, $window_conf:ident) => {{
        use $crate::wayland_adapter::WinHandle;
        $crate::paste::paste! {
        struct [<$slint_win Spell>] {
            ui: $slint_win ,
            way: SpellWin,
        }

        impl std::fmt::Debug for [<$slint_win Spell>] {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("Spell")
                .field("wayland_side:", &self.way) // Add fields by name
                .finish() // Finalize the struct formatting
            }
        }

        impl [<$slint_win Spell>] {
            /// Internally calls [`crate::wayland_adapter::SpellWin::hide`]
            pub fn hide(&self) {
                self.way.hide();
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::show_again`]
            pub fn show_again(&mut self) {
                self.way.show_again();
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::toggle`]
            pub fn toggle(&mut self) {
                self.way.toggle();
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::grab_focus`]
            pub fn grab_focus(&self) {
                self.way.grab_focus();
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::remove_focus`]
            pub fn remove_focus(&self) {
                self.way.remove_focus();
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::add_input_region`]
            pub fn add_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
                self.way.add_input_region(x, y, width, height);
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::subtract_input_region`]
            pub fn subtract_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
                self.way.subtract_input_region(x, y, width, height);
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::add_opaque_region`]
            pub fn add_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
                self.way.add_opaque_region(x, y, width, height);
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::subtract_opaque_region`]
            pub fn subtract_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
                self.way.subtract_opaque_region(x, y, width, height);
            }

            /// Internally calls [`crate::wayland_adapter::SpellWin::set_exclusive_zone`]
            pub fn set_exclusive_zone(&mut self, val: i32) {
                self.way.set_exclusive_zone(val);
            }
            /// Returns a handle of [`crate::wayland_adapter::WinHandle`] to invoke wayland specific features.
            pub fn get_handler(&self) -> WinHandle {
                WinHandle(self.way.loop_handle.clone())
            }
        }

        impl $crate::SpellAssociated for [<$slint_win Spell>] {
            fn on_call(
                &mut self,
                state: Option<$crate::State>,
                set_callback: Option<Box<dyn FnMut($crate::State)>>,
                span_log: $crate::tracing::span::Span,
            ) -> Result<(), Box<dyn std::error::Error>> {
                    self.way.on_call(state, set_callback, span_log)
            }

            fn get_span(&self) -> $crate::tracing::span::Span {
                self.way.span.clone()
            }
        }

        impl std::ops::Deref for [<$slint_win Spell>] {
            type Target = [<$slint_win>];
            fn deref(&self) -> &Self::Target {
                &self.ui
            }
        }
        let way_win = SpellWin::invoke_spell($name, $window_conf);

        [<$slint_win Spell>] {
            ui: $slint_win::new().unwrap(),
            way: way_win
        }
        }
    }};
}

#[macro_export]
macro_rules! cast_spell {
    (
    $waywindow:expr
    // $(, $state:expr)?
    // $(, $set_callback:expr)?
    $(, callbacks: {
            $(
                fn $name:ident ( $( $arg:ident : $ty:ty ),* $(,)? );
            )*
        })?
    $(, Notification:$noti_state:expr)?
    $(,)?
    ) => {{
        $(
            use $crate::smithay_client_toolkit::{
                reexports::{
                    calloop::{
                        self,
                        generic::Generic,
        PostAction
                        EventLoop,
                        timer::{TimeoutAction, Timer},
                    }
                }
            };
            use std::os::unix::{net::UnixListener, io::AsRawFd};
            let ui_weak = $waywindow.ui.as_weak();
            let socket_path = "/tmp/calloop_test.sock";
            let _ = std::fs::remove_file(socket_path); // Cleanup old socket
            let listener = UnixListener::bind(socket_path)?;
            listener.set_nonblocking(true)?;

            $waywindow.way.loop_handle.insert_source(
                Generic::new(listener, calloop::Interest::READ, calloop::Mode::Level),
                |event_msg, _, data| {
                    $(
                        println!("{}", stringify!($name));
                    );*
                    // TimeoutAction::ToDuration(std::time::Duration::from_millis(1000))
                    PostAction::Continue
                },
            );
        )?
        cast_spell($waywindow, None, None)
    }};
}

// TODO set logging values in Option so that only a single value reads or writes.
// TODO it is necessary to call join unwrap on spawned threads to ensure
// that they are closed when main thread closes.
// TODO linux's DNF Buffers needs to be used to improve rendering and avoid conversions
// from CPU to GPU and vice versa.
// TODO needs to have multi monitor support.
// TO REMEMBER I removed dirty region from spellskiawinadapter but it can be added
// if I want to make use of the dirty region information to strengthen my rendering.
// TODO lock screen behaviour in a multi-monitor setup needs to be tested.
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
// Also, a procedural macro to mimic the functionalities of ForeignController.
// Build a consistent error type to deal with CLI, dbus and window creation errors
// (including window conf) more gracefully.
// Provide natural scrolling option in SpellLock also.
