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

#[derive(Debug)]
pub struct PointerState {
    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_data: Option<PointerData>,
    pub cursor_shape: CursorShapeManager,
}

// This a boilerplate trait for connection with CLI, it will be replaced by a procedural
// macro in the future.
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
}

#[interface(
    name = "org.VimYoung.Spell1",
    proxy(
        gen_blocking = false,
        // default_path = "/org/VimYoung/VarHandler/WithProxy",
        // default_service = "org.VimYoung.Spell.WithProxy",
    )
)]
impl VarHandler {
    async fn set_value(&mut self, key: &str, val: &str) -> Result<(), BusError> {
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

    async fn show_window_back(&self) -> Result<(), BusError> {
        self.state_updater
            .send(InternalHandle::ShowWinAgain)
            .await
            .unwrap();
        Ok(())
    }

    async fn hide_window(&self) -> Result<(), BusError> {
        self.state_updater
            .send(InternalHandle::HideWindow)
            .await
            .unwrap();
        Ok(())
    }

    // A signal; the implementation is provided by the macro.
    // #[zbus(signal)]
    // async fn greeted_everyone(emitter: &SignalEmitter<'_>) -> FResult<()>;
}

pub async fn deploy_zbus_service(
    state: Arc<RwLock<Box<dyn ForeignController>>>,
    state_updater: Sender<InternalHandle>,
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
