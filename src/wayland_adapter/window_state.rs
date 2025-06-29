use smithay_client_toolkit::{
    reexports::client::protocol::wl_pointer,
    seat::pointer::{PointerData, cursor_shape::CursorShapeManager},
};
use std::{future::pending, result::Result};
use zbus::{Connection as BusConn, Result as BusResult, fdo::Error as BusError, interface};

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
}

<<<<<<< HEAD
// TODO Currently doesn't support brush, this enum needs to be updated to incorporate
// every type in which slint can convert its values to.
=======
// TODO, I can support a vector type which someone might use for using external
// command outputs to be stored inside.
#[derive(Debug)]
>>>>>>> 99095e1 (Dbus interface implemented)
pub enum DataType {
    Int(i32),
    String(String),
    Boolean(bool),
    Panic,
}

struct VarHandler {
    state: Box<dyn ForeignController>,
}

// TODO, properly implement the error type for this platform
// Application of an error type to work across the project is must.
// #[derive(Debug)]
// pub enum SpellError {
//     Buserror(zbus::Error),
//     FdoError(zbus::fdo::Error),
// }
//
// impl From<zbus::Error> for SpellError {
//     fn from(value: zbus::Error) -> Self {
//         SpellError::Buserror(value)
//     }
// }
//
// impl From<zbus::fdo::Error> for SpellError {
//     fn from(value: zbus::fdo::Error) -> Self {
//         SpellError::FdoError(value)
//     }
// }
//
// impl zbus::DBusError for SpellError {}

#[interface(
    name = "org.VimYoung.Spell",
    proxy(
        gen_blocking = false,
        default_path = "/org/VimYoung/VarHandler/WithProxy",
        default_service = "org.VimYoung.Spell.WithProxy",
    )
)]
impl VarHandler {
    async fn set_value(&mut self, key: &str, val: &str) -> Result<(), BusError> {
<<<<<<< HEAD
        let returned_value: DataType = self.state.get_type(val);
        match returned_value {
            DataType::Boolean(_) => {
                if let Ok(con_var) = val.trim().parse::<bool>() {
                    self.state.change_val(key, DataType::Boolean(con_var));
                } else {
                    return Err(BusError::NotSupported("Value is not supported".to_string()));
                }
            }
            _ => panic!("Implement the rest of Types of DataType"),
=======
        let returned_value: DataType = self.state.read().unwrap().get_type(key);
        match returned_value {
            DataType::Boolean(_) => {
                if let Ok(con_var) = val.trim().parse::<bool>() {
                    self.state_updater
                        .send((key.to_string(), DataType::Boolean(con_var)))
                        .await?;
                    Ok(())
                } else {
                    Err(BusError::Failed("Error".to_string()))
                    // Err(BusError::Failed("Value is not a valid boolean".into()))
                    // panic!("Temporary Panic , remove this code and handle the errors");
                    // return Err(BusError::NotSupported("Value is not supported".to_string()));
                }
            }
            DataType::Int(_) => {}
            DataType::String(_) => {}
            DataType::Panic => "Error from Panic".to_string(),
>>>>>>> 99095e1 (Dbus interface implemented)
        }
        Ok(())
    }

    async fn find_value(&self, key: &str) -> String {
        let value: DataType = self.state.get_type(key);
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

pub async fn deploy_zbus_service(state: Box<dyn ForeignController>) -> BusResult<()> {
    println!("deplied zbus serive");
    let connection = BusConn::session().await?;

    //Setting up object server.
    connection
        .object_server()
        .at("/org/VimYoung/VarHandler", VarHandler { state })
        .await?;
    connection.request_name("org.VimYoung.Spell").await?;

    pending::<()>().await;

    Ok(())
}
