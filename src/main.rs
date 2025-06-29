use std::{
    env::{self, Args},
    result::Result,
};

use zbus::{Connection, Result as BusResult, proxy};

#[proxy(
    default_service = "org.VimYoung.Spell",
    default_path = "/org/VimYoung/VarHandler"
)]
trait SpellClient {
    fn set_value(&mut self, key: &str, val: &str) -> Result<(), SpellError>;
    fn find_value(&self, key: &str) -> BusResult<String>;
}

#[tokio::main]
async fn main() -> Result<(), SpellError> {
    let mut values = env::args();
    values.next();
    if let Some(sub_command) = values.next() {
        match sub_command.as_str() {
            "update" => update_value(values).await,
            _ => Err(SpellError::CLI(Cli::BadSubCommand(format!(
                "This subcommand doesn't exist {}",
                sub_command
            )))),
        };
    }
    Ok(())
}

async fn update_value(values: Args) -> Result<(), SpellError> {
    let remain_arg: Vec<String> = values.collect();
    if remain_arg.len() != 2 {
        println!("Got the errrrrorrr");
        Err(SpellError::CLI(Cli::UndefinedArg(
            "Undefined arguments after update, only provide {{key}} and {{Value}}".to_string(),
        )))
    } else {
        let conn = Connection::session().await?;
        let mut proxy = SpellClientProxy::new(&conn).await?;
        let reply = proxy.set_value(&remain_arg[0], &remain_arg[1]).await?;
        dbg!(reply);
        Ok(())
    }
}

// TODO, properly implement the error type for this platform
// Application of an error type to work across the project is must.
#[derive(Debug)]
pub enum SpellError {
    Buserror(zbus::Error),
    CLI(Cli),
}

// TODO it needs to be more comprehensive for handling all the edge cases.
#[derive(Debug)]
pub enum Cli {
    BadSubCommand(String),
    UndefinedArg(String),
}

impl From<zbus::Error> for SpellError {
    fn from(value: zbus::Error) -> Self {
        SpellError::Buserror(value)
    }
}

// TODO write man docs for the command line tool and its Config if expanded further.
// Have to improve error handling by introduction of custom error type.
