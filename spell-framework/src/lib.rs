#![doc(
    html_logo_url = "https://raw.githubusercontent.com/VimYoung/Spell/main/spell-framework/assets/spell_trans.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/VimYoung/Spell/main/spell-framework/assets/spell_trans.ico"
)]
// #![doc(html_favicon_url = "https://github.com/VimYoung/Spell/blob/bb01ae94a365d237ebb0db1df1b6eb37aea25367/spell-framework/assets/Spell.png")]
#![doc = include_str!("../docs/entry.md")]
mod configure;
#[warn(missing_docs)]
mod event_macros;
pub mod forge;
#[cfg(feature = "i-slint-renderer-skia")]
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
    pub use crate::configure::{WindowConf, WindowConfBuilder};
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::KeyboardInteractivity as BoardType;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
}
pub use paste;
pub use smithay_client_toolkit;
use std::error::Error;
pub use tracing;
use tracing::{Level, span, trace};

pub trait IpcController {
    /// On calling `spell-cli -l layer_name look
    /// var_name`, the cli calls `get_type` method of the trait with `var_name` as input.
    fn get_type(&self, key: &str) -> String;
    /// It is called on `spell-cli -l layer_name update key value`. `as_any` is for syncing the changes
    /// internally for now and need not be implemented by the end user.
    fn change_val(&mut self, key: &str, val: &str);
}

pub fn cast_spell_inner<S: SpellAssociatedNew + std::fmt::Debug>(
    mut waywindow: S,
) -> Result<(), Box<dyn Error>> {
    let span = waywindow.get_span();
    span.in_scope(|| {
        trace!("{:?}", &waywindow);
        waywindow.on_call()
    })
}

pub trait SpellAssociatedNew {
    fn on_call(&mut self) -> Result<(), Box<dyn Error>>;

    fn get_span(&self) -> span::Span {
        span!(Level::INFO, "unnamed-widget")
    }
}

pub fn cast_spells_new(
    mut windows: Vec<Box<dyn SpellAssociatedNew>>,
) -> Result<(), Box<dyn Error>> {
    loop {
        for win in windows.iter_mut() {
            let span = win.get_span().clone();
            let _gaurd = span.enter();
            win.on_call()?;
        }
    }
}

// Code to launch a Zbus service
// <BS>
// pub async fn deploy_zbus_service(
//     state: State,
//     state_updater: Sender<InternalHandle>,
//     layer_name: String,
// ) -> zbus::Result<()> {
//     let connection = BusConn::session().await.unwrap();
//     connection
//         .object_server()
//         .at(
//             "/org/VimYoung/VarHandler",
//             VarHandler {
//                 state: state.clone(),
//                 state_updater: state_updater.clone(),
//                 layer_name: layer_name.clone(),
//             },
//         )
//         .await?;
//     trace!("Object server set up");
//     // connection.request_name("org.VimYoung.Spell").await?;
//     // open_sec_service(state, state_updater, layer_name).await?;
//     if let Err(err) = connection
//         .request_name_with_flags("org.VimYoung.Spell", RequestNameFlags::DoNotQueue.into())
//         .await
//     {
//         open_sec_service(state, state_updater, layer_name).await?;
//         info!("Successfully created secondary service, Error: {}", err);
//     } else {
//         info!("Successfully created main service");
//     }
//     std::future::pending::<()>().await;
//     Ok(())
// }
// Macro on top of VarHandler impl.
// #[interface(name = "org.VimYoung.Spell1", proxy(gen_blocking = false,))]
// TODO it is necessary to call join unwrap on spawned threads to ensure
// that they are closed when main thread closes.
// TODO linux's DNF Buffers needs to be used to improve rendering and avoid conversions
// from CPU to GPU and vice versa.
// TO REMEMBER I removed dirty region from spellskiawinadapter but it can be added
// if I want to make use of the dirty region information to strengthen my rendering.
// TODO lock screen behaviour in a multi-monitor setup needs to be tested.
// TODO implement logging for SpellLock.
// Provide a method in the macro to disable tracing_subsriber completely for some project
// which want's to do it themselves.
// cast spell macro should be having following values.
// 1. Disable log: should disable setting subscriber, generally for the project to use or for
// someone to set their own.
// 2. forge: provide a forge instance to run independently.
// 3. exclusive_zone: true or false or with specified value.
// 4. it should have the option to take a window_conf or directly the window configurations
// into the macro, removing the need to define it previously.
// 5. monitor: Specify the monitor to show the widget in.
// Also, a procedural macro to mimic the functionalities of ForeignController.
// Build a consistent error type to deal with CLI, dbus and window creation errors
// (including window conf) more gracefully.
