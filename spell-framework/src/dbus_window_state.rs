use crate::{dbus_window_state::second_client::open_internal_clinet, wayland_adapter::SpellWin};
use smithay_client_toolkit::{
    reexports::client::protocol::{wl_keyboard, wl_pointer},
    seat::{
        keyboard::KeyboardData,
        pointer::{PointerData, cursor_shape::CursorShapeManager},
    },
};
use std::{
    any::Any,
    future::pending,
    result::Result,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc::Sender;
use zbus::{
    Connection as BusConn, fdo::Error as BusError, interface, object_server::SignalEmitter,
};

mod second_client;
#[derive(Debug)]
pub struct PointerState {
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_data: Option<PointerData>,
    pub cursor_shape: CursorShapeManager,
}

#[derive(Debug)]
pub struct KeyboardState {
    pub board: Option<wl_keyboard::WlKeyboard>,
    pub board_data: Option<KeyboardData<SpellWin>>,
}

/// This a boilerplate trait for connection with CLI, it will be replaced by a procedural
/// macro in the future.
pub trait ForeignController: Send + Sync {
    fn get_type(&self, key: &str) -> DataType;
    fn change_val(&mut self, key: &str, val: DataType);
    fn as_any(&self) -> &dyn Any;
}

// TODO Currently doesn't support brush, this enum needs to be updated to incorporate
// every type in which slint can convert its values to.
// TODO, I can support a vector type which someone might use for using external
// command outputs to be stored inside.
#[derive(Debug)]
pub enum DataType {
    Int(i32),
    String(String),
    Boolean(bool),
    Panic,
}

pub enum InternalHandle {
    StateValChange((String, DataType)),
    ShowWinAgain,
    HideWindow,
}

struct VarHandler {
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<InternalHandle>,
    layer_name: String,
}

#[interface(
    name = "org.VimYoung.Spell1",
    proxy(
        gen_blocking = false,
        // default_service = "org.VimYoung.Spell",
        // default_path = "/org/VimYoung/VarHandler",
    )
)]
impl VarHandler {
    async fn set_value(
        &mut self,
        layer_name: &str,
        key: &str,
        val: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), BusError> {
        if layer_name == self.layer_name {
            let returned_value: DataType = self.state.read().unwrap().get_type(key);
            match returned_value {
                DataType::Boolean(_) => {
                    if let Ok(con_var) = val.trim().parse::<bool>() {
                        //TODO this needs to be handled once graceful shutdown is implemented.
                        let _ = self
                            .state_updater
                            .send(InternalHandle::StateValChange((
                                key.to_string(),
                                DataType::Boolean(con_var),
                            )))
                            .await;
                        Ok(())
                    } else {
                        Err(BusError::NotSupported("Value is not supported".to_string()))
                    }
                }
                DataType::Int(_) => Ok(()),
                DataType::String(_) => Ok(()),
                DataType::Panic => Err(BusError::Failed("Error from Panic".to_string())),
            }
        } else {
            emitter
                .layer_var_value_changed(layer_name, key, val)
                .await?;
            Ok(())
        }
    }

    async fn find_value(&self, layer_name: &str, key: &str) -> String {
        let value: DataType = self.state.read().unwrap().get_type(key);
        match value {
            DataType::Int(int_value) => int_value.to_string(),
            DataType::Boolean(bool_val) => bool_val.to_string().clone(),
            // TODO this implementation needs to be improved after changing DATATYPE
            _ => "hello".to_string(),
        }
    }

    async fn show_window_back(&self, layer_name: &str) -> Result<(), BusError> {
        self.state_updater
            .send(InternalHandle::ShowWinAgain)
            .await
            .unwrap();
        Ok(())
    }

    async fn hide_window(&self, layer_name: &str) -> Result<(), BusError> {
        self.state_updater
            .send(InternalHandle::HideWindow)
            .await
            .unwrap();
        Ok(())
    }

    // // A signal; the implementation is provided by the macro.
    // This emitter will be called externally by my CLI to notify the listeners of the
    // New changed state.
    #[zbus(signal)]
    async fn layer_var_value_changed(
        emitter: &SignalEmitter<'_>,
        layer_name: &str,
        var_name: &str,
        value: &str,
    ) -> zbus::Result<()>;
}

pub async fn deploy_zbus_service(
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<InternalHandle>,
    layer_name: String,
) -> zbus::Result<()> {
    println!("deplied zbus serive");
    let connection = BusConn::session().await?;

    //Setting up object server.
    // TODO This clone might be avoided.
    connection
        .object_server()
        .at(
            "/org/VimYoung/VarHandler",
            VarHandler {
                state: state.clone(),
                state_updater: state_updater.clone(),
                layer_name: layer_name.clone(),
            },
        )
        .await?;
    // let _ = connection::Builder::session()?
    //     .name("org.VimYoung.Spell")?
    //     .serve_at(
    //         "/org/VimYoung/VarHandler",
    //         VarHandler {
    //             state,
    //             state_updater,
    //             layer_name,
    //         },
    //     )?
    //     .build();
    if connection.request_name("org.VimYoung.Spell").await.is_err() {
        // An instance of VimYoung Dbus session already exists.
        open_internal_clinet(state, state_updater, layer_name).await?;
    }

    pending::<()>().await;

    Ok(())
}

// #[proxy(
//     default_path = "/org/VimYoung/VarHandler",
//     default_service = "org.VimYoung.Spell",
//     interface = "org.VimYoung.Spell1"
// )]
// trait SeconadryClient {
//     #[zbus(signal)]
//     fn layer_var_value_changed(layer_name: &str, var_name: &str, value: &str) -> zbus::Result<()>;
// }
