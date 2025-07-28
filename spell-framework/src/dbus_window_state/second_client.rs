use crate::{ForeignController, dbus_window_state::InternalHandle, layer_properties::DataType};
use futures_util::StreamExt;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::Sender;
use zbus::{Connection as BusConn, fdo::Error as BusError, proxy};

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
                        let _ = state_updater
                            .send(InternalHandle::StateValChange((
                                args.var_name.to_string(),
                                DataType::Boolean(con_var),
                            )))
                            .await;
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
            // state_updater
            //     .send(InternalHandle::StateValChange((
            //         args.var_name.to_string(),
            //         DataType::String,
            //     )))
            //     .unwrap();
        }
    }
    // let mut value_change = signal_reciever
    Ok(())
}
