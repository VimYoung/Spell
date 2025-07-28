pub mod constantvals;
use constantvals::MAIN_HELP;
use std::env::{self, Args};
use zbus::{proxy, Connection, Result as BusResult};

#[proxy(
    default_path = "/org/VimYoung/VarHandler",
    default_service = "org.VimYoung.Spell",
    interface = "org.VimYoung.Spell1"
)]
trait SpellClient {
    fn set_value(&mut self, layer_name: &str, key: &str, val: &str) -> Result<(), SpellError>;
    fn find_value(&self, layer_name: &str, key: &str) -> BusResult<String>;
    fn show_window_back(&self, layer_name: &str) -> Result<(), SpellError>;
    fn hide_window(&self, layer_name: &str) -> Result<(), SpellError>;
    #[zbus(signal)]
    fn layer_var_value_changed(
        &self,
        layer_name: &str,
        var_name: &str,
        value: &str,
    ) -> zbus::Result<()>;
}

#[tokio::main]
async fn main() -> Result<(), SpellError> {
    let mut values = env::args();
    values.next();
    let conn = Connection::session().await?;
    let proxy = SpellClientProxy::new(&conn).await?;
    if let Some(sub_command) = values.next() {
        let return_value = match sub_command.as_str() {
            "update" | "look" | "show" | "hide" => Err(SpellError::CLI(Cli::BadSubCommand("`-l` is not defined. Call these sub commands after specifying name with spell-cli -l `name` sub command".to_string()))),
            "-l" => match values.next() {
                Some(layer_value) => match values.next() {
                    Some(sub_command_after_layer) => match sub_command_after_layer.as_str() {
                        "update" => update_value(layer_value, values, proxy).await,
                        "look" => look_value(layer_value, values, proxy).await,
                        "show" => proxy.show_window_back(&layer_value).await,
                        "hide" => proxy.hide_window(&layer_value).await,
                        _ => Err(SpellError::CLI(Cli::BadSubCommand(format!("The subcommand \"{sub_command_after_layer}\" doesn't exist, use `spell --help` to view available commands"))))
                    },
                    None => Err(SpellError::CLI(Cli::NoSubCommand(
                        "provide a subcommand like 'update', 'look' etc".to_string(),
                    ))),
                },
                None => Err(SpellError::CLI(Cli::UndefinedArg(
                    "Provide the value of layer name".to_string(),
                ))),
            },
            // Used for enabling notifications, clients, lockscreen etc.
            "enable" => Ok(()),
            // TODO tracing subscriber logs here plus debug logs of slint here in sub commands.
            "logs" => Ok(()),
            // A later on added trait which can be configured and then running this command
            // Will display all the existing features of your shell as configured by the user.
            // So, when showcasing, he would only need to run this command once.
            "test" => Ok(()),
            // Following code will be used for opening clsing respected windows later on once
            // Spell is made multi-window compatable.
            "open" => Ok(()),
            "close" => Ok(()),
            "toggle" => Ok(()),
            // List the running instancs of windows and subwindows.
            "list" => Ok(()),
            "--help" => show_help(None),
            _ => {
                if sub_command.starts_with('-') || sub_command.starts_with("--") {
                    Err(SpellError::CLI(Cli::BadSubCommand(format!(
                "The flag \"{sub_command}\" doesn't exist, use `spell --help` to view available commands"
            ))))
                } else {
                    Err(SpellError::CLI(Cli::BadSubCommand(format!(
                "The subcommand \"{sub_command}\" doesn't exist, use `spell --help` to view available commands"
            ))))
                }
            }
        };
        if let Err(recieved_error) = return_value {
            // TODO Here the SpellError needs to be matched its each arm and proper messages needs
            // to be sent.
            // TODO below code can be avoided by implementing Debug manually for the erum
            match recieved_error {
                SpellError::CLI(cli) => match cli {
                    Cli::BadSubCommand(err) => {
                        eprintln!("[Bad Sub-command]: {err}");
                    }
                    Cli::UndefinedArg(err) => {
                        eprintln!("[Undefined Arg]: {err}");
                    }
                    Cli::NoSubCommand(flag_val) => {
                        eprintln!("[No Sub-command]: The flag {flag_val} should be followed by a subcommand");
                    }
                },
                SpellError::Buserror(bus_error) => match bus_error {
                    zbus::Error::MethodError(rare_err_1, err_val, rare_err_2) => {
                        if let Some(value) = err_val {
                            match value.as_str() {
                                "Value is not supported" => eprintln!(
                                    "[Parse Error]: Given Value for key couldn't be parsed."
                                ),
                                _ => eprintln!(
                                    "[Method Error]: Seems like the service is not running. \n Invoke `cast_spell` before calling for changes. {value}"
                                ),
                            }
                        } else {
                            let rare_err_value = (rare_err_1, rare_err_2);
                            eprintln!(
                                "[Undefined Error]: This error shouldn't be shown, open an issue with following Debug Output: \n {rare_err_value:#?}"
                            );
                        }
                    }
                    zbus::Error::Unsupported => {
                        eprintln!("[Parse Error]: Given Value for key couldn't be parsed.");
                    }
                    _ => eprintln!("[Undocumented Error]: {bus_error}"),
                },
            }
            // eprintln!("[Error] : \n {recieved_error:?}");
        }
    } else {
        let _ = show_help(None);
    }
    Ok(())
}

fn show_help(sub_command: Option<&str>) -> Result<(), SpellError> {
    match sub_command {
        // TODO Add help commands messages for sub-commands.
        Some(_sub_comm) => Ok(()),
        None => {
            println!("{MAIN_HELP}");
            Ok(())
        }
    }
}

async fn look_value(
    layer_name: String,
    mut values: Args,
    proxy: SpellClientProxy<'_>,
) -> Result<(), SpellError> {
    let remain_arg: String = values
        .next()
        .ok_or_else(|| SpellError::CLI(Cli::UndefinedArg("No variable name provided".to_string())))?
        .clone();
    let value: String = proxy.find_value(&layer_name, &remain_arg).await?;
    println!("{value}");
    Ok(())
}

async fn update_value(
    layer_name: String,
    values: Args,
    mut proxy: SpellClientProxy<'_>,
) -> Result<(), SpellError> {
    let remain_arg: Vec<String> = values.collect();
    if remain_arg.len() < 2 {
        Err(SpellError::CLI(Cli::UndefinedArg(
            "Less arguments given, provide {{key}} and {{Value}}".to_string(),
        )))
    } else if remain_arg.len() > 2 {
        Err(SpellError::CLI(Cli::UndefinedArg(
            "More than 2 arg given. Only provide {{key}} and {{Value}}".to_string(),
        )))
    } else {
        proxy
            .set_value(&layer_name, &remain_arg[0], &remain_arg[1])
            .await?;
        Ok(())
    }
}

// TODO, properly implement the error type for this platform
// Application of an error type to work across the project is must.
// Currently Buserror is not being used. THis requires type implementations
// of variant on Cli.
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
    NoSubCommand(String),
}

impl From<zbus::Error> for SpellError {
    fn from(value: zbus::Error) -> Self {
        SpellError::Buserror(value)
    }
}

// TODO write man docs for the command line tool and its Config if expanded further.
// Have to improve error handling by introduction of custom error type.
// TODO I have to set up panic hooks for .expect statements.
// TODO add --verbose argument in base command for directly passing the error
// outputs without matching/mapping/manipulating them.
// TODO, no answer is showing if the currently running ui doesn't possess the state
// in slint.
// Have to connect the logs of tokio with different filters.
// TODO set the Debug trait for my enuma and tell that dbus(DBUS_SESSION_BUS_ADDRSS)
// is not set when the following error comes:
// Error: Buserror(InputOutput(Os { code: 2, kind: NotFound, message: "No such file or directory" }))
//
