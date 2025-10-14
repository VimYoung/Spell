use smithay_client_toolkit::{
    reexports::client::protocol::wl_pointer,
    seat::pointer::{PointerData, cursor_shape::CursorShapeManager},
};
use std::{
    any::Any,
    future::pending,
    result::Result,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc::Sender;
use zbus::{Connection as BusConn, Result as BusResult, fdo::Error as BusError, interface};

pub struct PointerState {
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_data: Option<PointerData>,
    pub cursor_shape: CursorShapeManager,
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

struct VarHandler {
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<(String, DataType)>,
}

#[interface(
    name = "org.VimYoung.Spell1",
    proxy(
        gen_blocking = false,
        default_path = "/org/VimYoung/VarHandler/WithProxy",
        default_service = "org.VimYoung.Spell.WithProxy",
    )
)]
impl VarHandler {
    async fn set_value(&mut self, key: &str, val: &str) -> Result<(), BusError> {
        let returned_value: DataType = self.state.read().unwrap().get_type(key);
        match returned_value {
            DataType::Boolean(_) => {
                if let Ok(con_var) = val.trim().parse::<bool>() {
                    self.state_updater
                        .send((key.to_string(), DataType::Boolean(con_var)))
                        .await;
                    return Ok(());
                } else {
                    return Err(BusError::NotSupported("Value is not supported".to_string()));
                }
            }
            DataType::Int(_) => return Ok(()),
            DataType::String(_) => return Ok(()),
            DataType::Panic => return Err(BusError::Failed("Error from Panic".to_string())),
        }
        Ok(())
    }

    async fn find_value(&self, key: &str) -> String {
        let value: DataType = self.state.read().unwrap().get_type(key);
        match value {
            DataType::Int(int_value) => int_value.to_string(),
            DataType::Boolean(bool_val) => bool_val.to_string().clone(),
            // TODO this implementation needs to be improved after changing DATATYPE
            _ => "hello".to_string(),
        }
    }

    // A signal; the implementation is provided by the macro.
    // #[zbus(signal)]
    // async fn greeted_everyone(emitter: &SignalEmitter<'_>) -> FResult<()>;
}

pub async fn deploy_zbus_service(
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<(String, DataType)>,
) -> BusResult<()> {
    println!("deplied zbus serive");
    let connection = BusConn::session().await?;

    //Setting up object server.
    connection
        .object_server()
        .at(
            "/org/VimYoung/VarHandler",
            VarHandler {
                state,
                state_updater,
            },
        )
        .await?;
    connection.request_name("org.VimYoung.Spell").await?;

    pending::<()>().await;

    Ok(())
}
