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
    error::Error,
    sync::{Arc, RwLock},
    time::Duration,
};

use tracing::{error, info, instrument, span, trace, warn};
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
        info!("Starting windows");
        let spans: Vec<span::Span> = waywindows.iter().map(|win| win.span.clone()).collect();
        let mut internal_recievers: Vec<Channel<InternalHandle>> = Vec::new();
        states.iter().enumerate().for_each(|(index, state)| {
            internal_recievers.push(helper_fn_for_deploy(
                waywindows[index].layer_name.clone(),
                state,
                waywindows[index].span.clone(),
            ));
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
                            trace!("Internal event recieved");
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
        warn!("Lengths unequal coming");
        panic!(
            "The lengths of given vectors are not equal. \n Make sure that given vector lengths are equal"
        );
    }
}

// #[instrument(skip(set_callback, state, waywindow))]
pub fn cast_spell<
    S: SpellAssociated + std::fmt::Debug,
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>) + 'static,
>(
    mut waywindow: S,
    state: Option<State>,
    set_callback: Option<F>,
) -> Result<(), Box<dyn Error>> {
    // tracing_subscriber::fmt()
    //     .without_time()
    //     .with_env_filter(tracing_subscriber::EnvFilter::new(
    //         "spell_framework=trace,info",
    //     ))
    //     // .with_max_level(tracing::Level::TRACE)
    //     .init();
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
    state: &Option<Arc<RwLock<Box<dyn ForeignController>>>>,
    span_log: span::Span,
) -> Channel<InternalHandle> {
    let (tx, rx) = channel::<InternalHandle>();
    if let Some(some_state) = state {
        let state_clone = some_state.clone();
        std::thread::spawn(move || {
            span_log.in_scope(move || {
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

pub trait SpellAssociated {
    fn on_call<F>(
        &mut self,
        state: Option<Arc<RwLock<Box<dyn ForeignController>>>>,
        set_callback: Option<F>,
        span_log: tracing::span::Span,
    ) -> Result<(), Box<dyn Error>>
    where
        F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>) + 'static;

    fn get_span(&self) -> span::Span;
}

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
// 1. open_sec_service
//
// TODO implement logiing for SpellLock.
