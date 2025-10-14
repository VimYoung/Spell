use crate::{State, dbus_window_state::second_client::open_sec_service};
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use std::result::Result;
use tracing::{info, trace, warn};
use zbus::{
    Connection as BusConn,
    fdo::{Error as BusError, RequestNameFlags},
    interface,
};

mod second_client;
// TODO Currently doesn't support brush, this enum needs to be updated to incorporate
// every type in which slint can convert its values to.
// TODO, I can support a vector type which someone might use for using external
// command outputs to be stored inside.
#[derive(Debug)]
pub enum DataType {
    Int(i32),
    Float(f32),
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
    state: State,
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
                        self.state_updater
                            .send(InternalHandle::StateValChange((
                                key.to_string(),
                                DataType::Boolean(con_var),
                            )))
                            .unwrap_or_else(|err| {
                                warn!("{:?}", err);
                            });
                        Ok(())
                    } else {
                        Err(BusError::NotSupported("Value is not supported".to_string()))
                    }
                }
                DataType::Int(_) => {
                    if let Ok(con_var) = val.trim().parse::<i32>() {
                        self.state_updater
                            .send(InternalHandle::StateValChange((
                                key.to_string(),
                                DataType::Int(con_var),
                            )))
                            .unwrap_or_else(|err| {
                                warn!("{:?}", err);
                            });
                        Ok(())
                    } else {
                        Err(BusError::NotSupported("Value is not supported".to_string()))
                    }
                }
                DataType::Float(_) => {
                    if let Ok(con_var) = val.trim().parse::<f32>() {
                        self.state_updater
                            .send(InternalHandle::StateValChange((
                                key.to_string(),
                                DataType::Float(con_var),
                            )))
                            .unwrap_or_else(|err| {
                                warn!("{:?}", err);
                            });
                        Ok(())
                    } else {
                        Err(BusError::NotSupported("Value is not supported".to_string()))
                    }
                }
                DataType::String(_) => {
                    self.state_updater
                        .send(InternalHandle::StateValChange((
                            key.to_string(),
                            DataType::String(val.to_string()),
                        )))
                        .unwrap_or_else(|err| {
                            warn!("{:?}", err);
                        });
                    Ok(())
                }
                DataType::Panic => Err(BusError::Failed("Error from Panic".to_string())),
            }
        } else {
            let conn = BusConn::session().await?;
            let path = "org.VimYoung.".to_string() + layer_name;
            let _ = conn
                .call_method(
                    Some(path.as_str()),
                    "/org/VimYoung/VarHandler",
                    Some("org.VimYoung.Widget"),
                    "SetValue",
                    &(key, val),
                )
                .await?;
            Ok(())
        }
    }

    async fn find_value(&self, layer_name: &str, key: &str) -> Result<String, BusError> {
        if self.layer_name == layer_name {
            let value: DataType = self.state.read().unwrap().get_type(key);
            match value {
                DataType::Int(int_value) => Ok(int_value.to_string()),
                DataType::Boolean(bool_val) => Ok(bool_val.to_string().clone()),
                DataType::Float(float_val) => Ok(float_val.to_string()),
                DataType::String(val) => Ok(val.clone()),
                DataType::Panic => Err(BusError::Failed(
                    "Panic value could not be found".to_string(),
                )),
            }
        } else {
            let conn = BusConn::session().await?;
            let path = "org.VimYoung.".to_string() + layer_name;
            let return_val = conn
                .call_method(
                    Some(path.as_str()),
                    "/org/VimYoung/VarHandler",
                    Some("org.VimYoung.Widget"),
                    "FindValue",
                    &(key),
                )
                .await?;
            // TODO this unwrap needs to be better handleed.
            Ok(return_val.body().deserialize().unwrap())
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
                    &(),
                )
                .await;
            Ok(())
        }
    }

    async fn hide_window(&self, layer_name: &str) -> Result<(), BusError> {
        if self.layer_name == layer_name {
            if self.state_updater.send(InternalHandle::HideWindow).is_err() {};
            Ok(())
        } else {
            let conn = BusConn::session().await?;
            let path = "org.VimYoung.".to_string() + layer_name;
            let _ = conn
                .call_method(
                    Some(path.as_str()),
                    "/org/VimYoung/VarHandler",
                    Some("org.VimYoung.Widget"),
                    "HideWindow",
                    &(),
                )
                .await?;
            Ok(())
        }
    }
}

pub async fn deploy_zbus_service(
    state: State,
    state_updater: Sender<InternalHandle>,
    layer_name: String,
) -> zbus::Result<()> {
    let connection = BusConn::session().await.unwrap();
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
    trace!("Object server set up");
    // connection.request_name("org.VimYoung.Spell").await?;
    // open_sec_service(state, state_updater, layer_name).await?;
    if let Err(err) = connection
        .request_name_with_flags("org.VimYoung.Spell", RequestNameFlags::DoNotQueue.into())
        .await
    {
        open_sec_service(state, state_updater, layer_name).await?;
        info!("Successfully created secondary service, Error: {}", err);
    } else {
        info!("Successfully created main service");
    }
    std::future::pending::<()>().await;
    Ok(())
}
