use crate::{State, dbus_window_state::InternalHandle, layer_properties::DataType};
use core::panic;
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use tracing::{info, trace, warn};
use zbus::{Connection as BusConn, fdo::Error as BusError, interface};
pub async fn open_sec_service(
    state: State,
    state_updater: Sender<InternalHandle>,
    layer_name: String,
) -> zbus::Result<()> {
    info!("Secondary client for {} created", layer_name);
    let conn = BusConn::session().await?;

    let path = "org.VimYoung.".to_string() + layer_name.as_str();
    conn.object_server()
        .at(
            "/org/VimYoung/VarHandler",
            WidgetHandler {
                state,
                state_updater,
            },
        )
        .await?;
    trace!("Object path for secondary service created");

    if conn.request_name(path.as_str()).await.is_err() {
        panic!(
            "A Service for this widget name already exists, please change the name or don't restart an already running widget"
        );
    }
    info!("Secondary clinet execution is complete.");
    std::future::pending::<()>().await;
    Ok(())
}

pub(crate) struct WidgetHandler {
    state: State,
    state_updater: Sender<InternalHandle>,
}

#[interface(name = "org.VimYoung.Widget", proxy(gen_blocking = false))]
impl WidgetHandler {
    pub(crate) async fn set_value(&mut self, key: &str, val: &str) -> Result<(), BusError> {
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
    }

    pub(crate) async fn hide_window(&self) -> Result<(), BusError> {
        self.state_updater.send(InternalHandle::HideWindow).unwrap();
        Ok(())
    }

    async fn show_window_back(&self) -> Result<(), BusError> {
        self.state_updater
            .send(InternalHandle::ShowWinAgain)
            .unwrap();
        Ok(())
    }

    async fn find_value(&self, key: &str) -> Result<String, BusError> {
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
    }
}
