use crate::{ForeignController, dbus_window_state::InternalHandle, layer_properties::DataType};
use core::panic;
use futures_util::StreamExt;
use std::sync::{Arc, RwLock};
// use tokio::sync::mpsc::Sender;
use smithay_client_toolkit::reexports::calloop::channel::Sender;
use zbus::{Connection as BusConn, fdo::Error as BusError, interface, proxy};

// Here var_name is nothing but the key
#[proxy(
    default_service = "org.VimYoung.Spell",
    default_path = "/org/VimYoung/VarHandler",
    interface = "org.VimYoung.Spell1"
)]
trait SecondClient {
    #[zbus(signal)]
    fn layer_var_value_changed(
        &self,
        layer_name: &str,
        var_name: &str,
        value: &str,
    ) -> Result<(), zbus::Error>;
}

pub async fn open_internal_clinet(
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<InternalHandle>,
    layer_name: String,
) -> Result<(), BusError> {
    let conn = BusConn::session().await?;
    let recv = SecondClientProxy::new(&conn).await?;

    let mut value_change_stream = recv.receive_layer_var_value_changed().await?;
    while let Some(msg) = value_change_stream.next().await {
        let args: LayerVarValueChangedArgs = msg.args().expect("Error parsing");
        if layer_name == args.layer_name {
            let returned_value: DataType = state.read().unwrap().get_type(args.var_name);
            match returned_value {
                DataType::Boolean(_) => {
                    if let Ok(con_var) = args.value.trim().parse::<bool>() {
                        //TODO this needs to be handled once graceful shutdown is implemented.
                        let _ = state_updater.send(InternalHandle::StateValChange((
                            args.var_name.to_string(),
                            DataType::Boolean(con_var),
                        )));
                        return Ok(());
                    } else {
                        return Err(BusError::NotSupported("Value is not supported".to_string()));
                    }
                }
                // TODO to be implemented for other types.
                DataType::Int(_) => return Ok(()),
                DataType::String(_) => return Ok(()),
                DataType::Panic => return Err(BusError::Failed("Error from Panic".to_string())),
            }
        }
    }
    Ok(())
}

pub async fn open_sec_service(
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<InternalHandle>,
    layer_name: String,
) -> zbus::Result<()> {
    println!("Secondary client for {} created", layer_name);
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

    if conn.request_name(path.as_str()).await.is_err() {
        panic!(
            "A Service for this widget name already exists, please change the name or don't restart an already running widget"
        );
    }
    println!("Secondary clinet execution is complete.");
    Ok(())
}

pub(crate) struct WidgetHandler {
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<InternalHandle>,
}

#[interface(name = "org.VimYoung.Widget", proxy(gen_blocking = false))]
impl WidgetHandler {
    pub(crate) async fn set_value(&mut self, key: &str, val: &str) -> Result<(), BusError> {
        let returned_value: DataType = self.state.read().unwrap().get_type(key);
        match returned_value {
            DataType::Boolean(_) => {
                if let Ok(con_var) = val.trim().parse::<bool>() {
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
    }

    pub(crate) async fn hide_window(&self, layer_name: &str) -> Result<(), BusError> {
        self.state_updater
            .send(InternalHandle::ShowWinAgain)
            .unwrap();
        Ok(())
    }
}
