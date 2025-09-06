use crate::dbus_window_state::second_client::{open_internal_clinet, open_sec_service};
use std::{
    any::Any,
    future::pending,
    result::Result,
    sync::{Arc, RwLock},
};
// use tokio::sync::mpsc::Sender;
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use zbus::{
    Connection as BusConn, fdo::Error as BusError, interface, object_server::SignalEmitter,
};

mod second_client;
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

#[interface(name = "org.VimYoung.Spell1", proxy(gen_blocking = false,))]
impl VarHandler {
    async fn set_value(&mut self, layer_name: &str, key: &str, val: &str) -> Result<(), BusError> {
        if layer_name == self.layer_name {
            let returned_value: DataType = self.state.read().unwrap().get_type(key);
            match returned_value {
                DataType::Boolean(_) => {
                    if let Ok(con_var) = val.trim().parse::<bool>() {
                        //TODO this needs to be handled once graceful shutdown is implemented.
                        let _ = self.state_updater.send(InternalHandle::StateValChange((
                            key.to_string(),
                            DataType::Boolean(con_var),
                        )));
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
            todo!();
            // emitter
            //     .layer_var_value_changed(layer_name, key, val)
            //     .await?;
        }
    }

    async fn find_value(&self, layer_name: &str, key: &str) -> String {
        if self.layer_name == layer_name {
            let value: DataType = self.state.read().unwrap().get_type(key);
            match value {
                DataType::Int(int_value) => int_value.to_string(),
                DataType::Boolean(bool_val) => bool_val.to_string().clone(),
                // TODO this implementation needs to be improved after changing DATATYPE
                _ => "".to_string(),
            }
        } else
        /*if let Err(err_val) = emitter.layer_find_var(layer_name, key).await
        && let zbus::Error::Address(val) = err_val */
        {
            todo!()
            //     val
            // } else {
            //     "".to_string()
        }
    }

    async fn show_window_back(&self, layer_name: &str) -> Result<(), BusError> {
        if self.layer_name == layer_name {
            self.state_updater
                .send(InternalHandle::ShowWinAgain)
                .unwrap();
            Ok(())
        } else {
            let path = "org.VimYoung.".to_string() + layer_name;
            let conn = BusConn::session().await?;
            let _ = conn
                .call_method(
                    Some(path.as_str()),
                    "/org/VimYoung/VarHandler",
                    Some("org.VimYoung.Widget"),
                    "ShowWindowBack",
                    &(layer_name),
                )
                .await;
            Ok(())
        }
    }

    async fn hide_window(&self, layer_name: &str) -> Result<(), BusError> {
        println!("Hide command coming, layer name: {}", layer_name);
        println!("{}", self.layer_name);
        if self.layer_name == layer_name {
            println!("In layer same{}", layer_name);
            if self.state_updater.send(InternalHandle::HideWindow).is_err() {
                println!("Some error occured");
            };
            Ok(())
        } else {
            println!("In layer {}", layer_name);
            println!("Called hide");
            let conn = BusConn::session().await?;
            let path = "org.VimYoung.".to_string() + layer_name;
            let _ = conn
                .call_method(
                    Some(path.as_str()),
                    "/org/VimYoung/VarHandler",
                    Some("org.VimYoung.Widget"),
                    "ShowWindowBack",
                    &(layer_name),
                )
                .await?;
            Ok(())
        }
    }
    //
    // #[zbus(signal)]
    // async fn layer_var_value_changed(
    //     emitter: &SignalEmitter<'_>,
    //     layer_name: &str,
    //     var_name: &str,
    //     value: &str,
    // ) -> zbus::Result<()>;
    //
    // #[zbus(signal)]
    // async fn layer_find_var(
    //     emitter: &SignalEmitter<'_>,
    //     layer_name: &str,
    //     var_name: &str,
    // ) -> zbus::Result<()>;
}

pub async fn deploy_zbus_service(
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<InternalHandle>,
    layer_name: String,
) -> zbus::Result<()> {
    println!("deploied zbus serive");
    let connection = BusConn::session().await.unwrap();

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
    println!("Object server set up");
    connection.request_name("org.VimYoung.Spell").await?;

    open_sec_service(state, state_updater, layer_name).await?;
    // if connection.request_name("org.VimYoung.Spell").await.is_err() {
    //     // An instance of VimYoung Dbus session already exists.
    //     // open_internal_clinet(state, state_updater, layer_name).await?;
    //     println!("creating widget");
    //     open_sec_service(state, state_updater, layer_name).await?;
    // }

    pending::<()>().await;

    Ok(())
}
