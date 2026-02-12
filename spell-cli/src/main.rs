pub mod constantvals;
use constantvals::{
    APP_WINDOW_SLINT, BUILD_FILE, CARGO_TOML, ENABLE_HELP, FPRINT_HELP, LOGS_HELP, MAIN_FILE,
    MAIN_HELP, SPELL_PAM_FPRINT,
};
use core::panic;
use futures_util::stream::StreamExt;
use std::{
    env::{self, Args},
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::unix::net::{UnixDatagram, UnixStream},
    path::Path,
    process::Command,
};
use zbus::{Connection, proxy, zvariant::OwnedObjectPath};

#[proxy(
    default_path = "/net/reactivated/Fprint/Manager",
    default_service = "net.reactivated.Fprint",
    interface = "net.reactivated.Manager"
)]
trait FprintdManagerClient {
    fn get_default_device(&self) -> Result<OwnedObjectPath, SpellError>;
    fn get_devices(&self) -> Result<Vec<OwnedObjectPath>, SpellError>;
}

#[proxy(
    default_path = "/net/reactivated/Fprint/Device/0",
    default_service = "net.reactivated.Fprint",
    interface = "net.reactivated.Fprint.Device"
)]
trait FprintdClient {
    #[zbus(signal)]
    fn enroll_status(&self, result: &str, done: bool) -> zbus::Result<()>;
    #[zbus(signal)]
    fn verify_status(&self, result: &str, done: bool) -> zbus::Result<()>;
    #[zbus(signal)]
    fn verify_finger_selected(&self, result: &str, done: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;
    fn claim(&self, username: &str) -> Result<(), SpellError>;
    fn enroll_start(&self, finger_name: &str) -> Result<(), SpellError>;
    fn enroll_stop(&self) -> Result<(), SpellError>;
    fn verify_start(&self, finger_name: &str) -> Result<(), SpellError>;
    fn verify_stop(&self) -> Result<(), SpellError>;
    fn list_enrolled_fingers(&self, username: &str) -> Result<Vec<String>, SpellError>;
}

#[tokio::main]
async fn main() -> Result<(), SpellError> {
    let mut values = env::args();
    values.next();
    if let Some(sub_command) = values.next() {
        let return_value = match sub_command.trim() {
            "--version" | "-v" => {
                println!("spell-cli version 1.0.2");
            Ok(())
            },
            "update" | "look" | "show" | "hide" => Err(SpellError::CLI(Cli::BadSubCommand("`-l` is not defined. Call these sub commands after specifying name with spell-cli -l|--layer `name` sub command".to_string()))),
            "-l" | "--layer" => match values.next() {
                Some(layer_value) => match values.next() {
                    Some(sub_command_after_layer) => match sub_command_after_layer.trim() {
                        "update" => update_value(layer_value, values).await,
                        "look" => look_value(layer_value, values).await,
                        "show" => show_window_back(layer_value).await,
                        "hide" => hide_window(layer_value).await,
                        "log" => get_logs(Some(layer_value), values).await,
                        _ => Err(SpellError::CLI(Cli::BadSubCommand(format!("The subcommand \"{sub_command_after_layer}\" doesn't exist, use `spell --help` to view available commands"))))
                    },
                    None => Err(SpellError::CLI(Cli::UndefinedArg(
                        "provide a subcommand like 'update', 'look' etc".to_string(),
                    ))),
                },
                None => Err(SpellError::CLI(Cli::UndefinedArg(
                    "Provide the value of layer name".to_string(),
                ))),
            },
            // Used for enabling notifications, clients, lockscreen etc.
            "enable" => Ok(()),
            "new" => match values.next() {
                Some(dest_dir) => {
                    create_spell_project(dest_dir)
                }
                None => Err(SpellError::CLI(Cli::UndefinedArg("Provide a destination for creating the project.".to_string())))
            }
            // TODO tracing subscriber logs here plus debug logs of slint here in sub commands.
            "log" => match values.next() {
                Some(log_type) =>  match log_type.trim() {
                "-l" | "--layer" => match values.next() {
                        Some(layer_name) => get_logs(Some(layer_name), values).await,
                        None => Err(SpellError::CLI(Cli::UndefinedArg(
                            "Provide the value of layer name".to_string(),
                        ))),
                    },
                "--help" | "-h" => show_help(Some("log")),
                _ => get_logs(None, values).await,
                },
                None => Err(SpellError::CLI(Cli::UndefinedArg("define a layer name to display user logs".to_string())))
            } ,
            // A later on added trait which can be configured and then running this command
            // Will display all the existing features of your shell as configured by the user.
            // So, when showcasing, he would only need to run this command once.
            "test" => Ok(()),
            // List the running instancs of windows and subwindows.
            "list" => Ok(()),
            "--help" | "-h" => show_help(None),
            "fprint" => {
                match values.next() {
                Some(fprint_sub) => match fprint_sub.trim() {
                    "--help" => show_help(Some("fprint")),
                    "list" => fingerprint(Fprint::List).await,
                    "enroll" => fingerprint(Fprint::Enroll).await,
                    "verify" =>fingerprint(Fprint::Verify).await,
                    sub => Err(SpellError::CLI(Cli::BadSubCommand(format!("The subcommand \"{sub}\" doesn't exist, use `spell --help` to view available commands"))))
                }
                None => fingerprint(Fprint::List).await
                }
            },
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
                    Cli::BadSubCommand(err) => eprintln!("[Bad Sub-command]: {err}"),
                    Cli::UndefinedArg(err) => eprintln!("[Undefined Arg]: {err}"),
                    Cli::UnknownVal(err) => eprintln!("[Unknown Value] {err}"),
                    Cli::UndeclaredVal(err) => eprintln!("[Undeclared Content]: {err}"),
                },
                SpellError::Buserror(bus_error) => match bus_error {
                    zbus::Error::MethodError(rare_err_1, err_val, rare_err_2) => {
                        if let Some(value) = err_val {
                            match value.as_str() {
                                "Value is not supported" => eprintln!(
                                    "[Parse Error]: Given Value for key couldn't be parsed."
                                ),
                                "Sender is not authorized to send message" => eprintln!(
                                    "List couldn't currently display devices and fingerprints. Use `fprintd-list` instead if installed"
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
                SpellError::IO(err_val) => {
                    eprintln!(
                        "[IO Error]: IO error while running the commands internally. \n Error: {}",
                        err_val
                    )
                }
            }
        }
    } else {
        let _ = show_help(None);
    }
    Ok(())
}

async fn fingerprint(fprint: Fprint) -> Result<(), SpellError> {
    let conn_system = Connection::system().await?;
    let proxy = FprintdClientProxy::new(&conn_system).await?;
    match fprint {
        Fprint::Enroll => enroll_fingerprint(&proxy).await?,
        Fprint::Verify => verify_fingerprint(&proxy).await?,
        Fprint::List => {
            // TODO this functionality doesn't work currently, fix it.
            let manager_proxy = FprintdManagerClientProxy::new(&conn_system).await?;
            list_devices_and_fingerprints(&conn_system, &manager_proxy).await?;
        }
    }
    Ok(())
}

pub(crate) async fn list_devices_and_fingerprints(
    conn_system: &Connection,
    manager_proxy: &FprintdManagerClientProxy<'_>,
) -> Result<(), SpellError> {
    let devices = manager_proxy.get_devices().await?;
    for (manager_index, device) in devices.iter().enumerate() {
        println!("{:?}", device);
        let proxy = FprintdClientProxy::builder(conn_system)
            .path(device)?
            .build()
            .await?;
        println!("Device {}: {}", manager_index, proxy.name().await?);
        let fingerprints = proxy.list_enrolled_fingers("").await?;
        for (index, fingerprint) in fingerprints.iter().enumerate() {
            println!("    Fingerprint {}: {}", index, fingerprint);
        }
    }
    Ok(())
}

pub(crate) async fn verify_fingerprint(proxy: &FprintdClientProxy<'_>) -> Result<(), SpellError> {
    proxy.claim("").await?;
    let fingerprints = proxy.list_enrolled_fingers("").await?;
    for finger in fingerprints {
        proxy.verify_start(&finger).await?;
        while let Some(msg) = proxy.receive_verify_status().await?.next().await {
            // struct `JobNewArgs` is generated from `job_new` signal function arguments
            let args: VerifyStatusArgs<'_> = msg.args().expect("Error parsing message");
            println!("Result={}", args.result);
            if args.done {
                break;
            }
        }
        proxy.verify_stop().await?;
    }
    Ok(())
}

pub(crate) async fn enroll_fingerprint(proxy: &FprintdClientProxy<'_>) -> Result<(), SpellError> {
    println!("{}", SPELL_PAM_FPRINT);
    let mut finger_str = String::from("");
    print!(
        r#"Enter Fingerprint type (Available options):
            Value               : Meaning
            left-thumb          : Left thumb
            left-index-finger   : Left index finger
            left-middle-finger  : Left middle finger
            left-ring-finger    : Left ring finger
            left-little-finger  : Left little finger
            right-thumb         : Right thumb
            right-index-finger  : Right index finger
            right-middle-finger : Right middle finger
            right-ring-finger   : Right ring finger
            right-little-finger : Right little finger 
            :"#
    );
    io::stdin()
        .read_line(&mut finger_str)
        .expect("unable to read finger type");
    proxy.claim("").await?;
    proxy.enroll_start(finger_str.trim()).await?;
    while let Some(msg) = proxy.receive_enroll_status().await?.next().await {
        // struct `JobNewArgs` is generated from `job_new` signal function arguments
        let args: EnrollStatusArgs<'_> = msg.args().expect("Error parsing message");

        println!("Result={}", args.result);
        if args.done {
            break;
        }
    }
    proxy.enroll_stop().await?;
    Ok(())
}

fn create_spell_project(path: String) -> Result<(), SpellError> {
    if let Ok(output) = Command::new("cargo").args(["new", &path]).output() {
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
    } else {
        panic!("Error running cargo command, is it installed?");
    }
    let path_ui: String;
    let slint_file: String;
    let build_file: String;
    let main_file: String;
    let cargo_file: String;

    if path.ends_with('/') {
        path_ui = path.clone() + "ui";
        slint_file = path.clone() + "ui/app-window.slint";
        build_file = path.clone() + "build.rs";
        main_file = path.clone() + "src/main.rs";
        cargo_file = path.clone() + "Cargo.toml";
    } else {
        path_ui = path.clone() + "/ui";
        slint_file = path.clone() + "/ui/app-window.slint";
        build_file = path.clone() + "/build.rs";
        main_file = path.clone() + "/src/main.rs";
        cargo_file = path.clone() + "/Cargo.toml";
    }

    fs::create_dir(Path::new(&path_ui))?;
    let mut app_window_slint = fs::File::create(Path::new(&slint_file))?;
    let mut build_rs = fs::File::create(Path::new(&build_file))?;
    let mut main_rs = fs::File::create(Path::new(&main_file))?;
    let mut cargo_toml = fs::File::create(Path::new(&cargo_file))?;

    app_window_slint.write_all(APP_WINDOW_SLINT.as_bytes())?;
    println!("Writing slint file...");
    build_rs.write_all(BUILD_FILE.as_bytes())?;
    println!("Writing build file...");
    main_rs.write_all(MAIN_FILE.as_bytes())?;
    println!("Writing main file...");
    let file_name = Path::new(&path).file_name().unwrap().to_str().unwrap();
    cargo_toml
        .write_all(format!("[package] \nname = \"{file_name}\" \n {CARGO_TOML}").as_bytes())?;
    println!("Spell enchanted!!");
    Ok(())
}

async fn get_logs(layer_name: Option<String>, mut values: Args) -> Result<(), SpellError> {
    if let Some(layer_name) = layer_name {
        match values.next() {
            Some(debug_type) => match debug_type.trim() {
                "slint_debug" => get_tracing_debug(LogType::Slint, layer_name),
                "debug" => get_tracing_debug(LogType::Debug, layer_name),
                "dump" => get_tracing_debug(LogType::Dump, layer_name),
                "dev" => get_tracing_debug(LogType::Dev, layer_name),
                x => Err(SpellError::CLI(Cli::UnknownVal(format!(
                    "{x} is not a debug type.",
                )))),
            },
            None => get_tracing_debug(LogType::Debug, layer_name),
        }
    } else {
        Err(SpellError::CLI(Cli::UndefinedArg(
            "define a layer name to display user logs".to_string(),
        )))
    }
}

fn get_tracing_debug(log_type: LogType, _layer_name: String) -> Result<(), SpellError> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("runtime dir is not set");
    let logging_dir = runtime_dir + "/spell/";
    let socket_cli_dir = logging_dir.clone() + "/spell_cli";
    // let _ = fs::remove_file(&socket_cli_dir);
    // stream.connect(&socket_dir).unwrap();
    // TODO have to return error here in the match sttement to avoid unwrap.
    let mut file = OpenOptions::new()
        .create(true) // create the file if it doesn’t exist
        .write(true)
        .truncate(true)
        .open(&socket_cli_dir)
        .unwrap();
    match log_type {
        LogType::Slint => file.write_all(b"slint_log").unwrap(),
        LogType::Debug => file.write_all(b"debug").unwrap(),
        LogType::Dump => file.write_all(b"dump").unwrap(),
        LogType::Dev => file.write_all(b"dev").unwrap(),
    };

    let socket_path = "/run/user/1000/spell/spell.sock";
    std::fs::remove_file(socket_path).ok();
    let sock = UnixDatagram::bind(socket_path).unwrap();
    println!("Listening for logs on {}", socket_path);

    let mut buf = [0u8; 2048];
    loop {
        if let Ok((n, _)) = sock.recv_from(&mut buf) {
            print!("{}", String::from_utf8_lossy(&buf[..n]));
        };
    }
}

fn show_help(sub_command: Option<&str>) -> Result<(), SpellError> {
    match sub_command {
        // TODO Add help commands messages for sub-commands.
        Some(sub_comm) => {
            match sub_comm {
                "log" => println!("{LOGS_HELP}"),
                "enable" => println!("{ENABLE_HELP}"),
                "fprint" => println!("{FPRINT_HELP}"),
                _ => {}
            }
            Ok(())
        }
        None => {
            println!("{MAIN_HELP}");
            Ok(())
        }
    }
}

async fn hide_window(layer_name: String) -> Result<(), SpellError> {
    let request = String::from("hide");
    let path = format!("/tmp/{}_ipc.sock", layer_name.trim());
    let mut stream = UnixStream::connect(path)?;
    stream.write_all(request.as_bytes())?;
    Ok(())
}

async fn show_window_back(layer_name: String) -> Result<(), SpellError> {
    let request = String::from("show");
    let path = format!("/tmp/{}_ipc.sock", layer_name.trim());
    let mut stream = UnixStream::connect(path)?;
    stream.write_all(request.as_bytes())?;
    Ok(())
}

async fn look_value(layer_name: String, mut values: Args) -> Result<(), SpellError> {
    let remain_arg: String = values
        .next()
        .ok_or_else(|| SpellError::CLI(Cli::UndefinedArg("No variable name provided".to_string())))?
        .clone();

    let request = format!("look {}", remain_arg);
    let path = format!("/tmp/{}_ipc.sock", layer_name.trim());
    let mut stream = UnixStream::connect(path)?;
    stream.write_all(request.as_bytes())?;
    let mut value = String::new();
    stream.read_to_string(&mut value)?;
    println!("{value}");
    Ok(())
}

async fn update_value(layer_name: String, values: Args) -> Result<(), SpellError> {
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
        let request = format!("update {}", remain_arg.join(" "));
        let path = format!("/tmp/{}_ipc.sock", layer_name.trim());
        let mut stream = UnixStream::connect(path)?;
        stream.write_all(request.as_bytes())?;
        // proxy
        //     .set_value(&layer_name, &remain_arg[0], &remain_arg[1])
        //     .await?;
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
    IO(std::io::Error),
}

// TODO it needs to be more comprehensive for handling all the edge cases.
#[derive(Debug)]
pub enum Cli {
    BadSubCommand(String),
    UndefinedArg(String),
    UnknownVal(String),
    UndeclaredVal(String),
}

impl From<std::io::Error> for SpellError {
    fn from(value: std::io::Error) -> Self {
        SpellError::IO(value)
    }
}

impl From<zbus::Error> for SpellError {
    fn from(value: zbus::Error) -> Self {
        SpellError::Buserror(value)
    }
}

enum Fprint {
    Enroll,
    List,
    Verify,
}

enum LogType {
    Slint,
    Debug,
    Dump,
    Dev,
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
