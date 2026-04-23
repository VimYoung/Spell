pub const MAIN_HELP: &str = "
Usage: sp [<argument>] [sub-command] [options] ...

sp provides a convenient way to remotely handle windows made by spell-framework.

Sub-commands:
    new: Used to create new rust project with spell and slint set up given the paths.
         For more details run `sp new --help`.
    [-l] update KEY VALUE: Updates the value of given key with the provided value
                            for the specifed layer window. Final implementation
                            depends on your ForeignController trait implementation.
    [-l] look KEY: Fetched the key's value for the specifed layer.
    [-l] show: Shows the specified window if hidden.
    [-l] hide: Hides the specified window.
    new: Create a new spell project with git initialised dependencies added and a
         minimial example to run given the path/name of project.
    enable: Enable services like lockscreen, notification etc if there is a
            provided implementation for it. Run `sp enable --help` for
            more details.
    fprint: Used to verify, add and list fingerprints registered to a device. Needs
            fprint-daemnon running to work.
    log: Display the specified logs. If -l is defined defaults to the logs
          of that layer only. Run `sp [-l LAYER_NAME] logs --help` for
          more details.
    list: (WIP) Lists the running instances of windows created by spell-framework.

Arguments:
    --layer | -l:   Specifies the name of layer (aka window) to be used for specific
                    commands. Use unique names of layers to avoid undefined behaviour.
                    Required by update, look, show and hide sub commands.
    --help | -h:    Shows this help message.
    --version | -v: Displays the version of sp.
";

pub const NEW_PROJECT_HELP: &str = "
Usage: sp new [<argument>] [path]

`new` subcommand is used for creating rust projects with spell and slint. Specification
of pre-build component library can also be done during creation.

Arguments:
    --material: Adds official material component library.
    --sleek : Adds sleek (ant design based) component library.
    --vivi : Adds vivi component library.
    --surrealism : Adds SurrealismUI component library.
";

pub const LOGS_HELP: &str = "
Usage: sp log [<argument>] [sub-command] ...

`log` subcommand is used to retrieve logging information from currently running widget instances.

Sub-commands:
    debug: General errors and warning that might be helpful in debugging(Default type).
    slint_debug: links slint's debug!{} method output. You can view your debug
                 statements from it.
    dump: Performance metric information of spell, generally not needed by
          end user. It's output can be used to point issues.
    dev: Development related logs not intended for end users.

Arguments:
    --layer | -l :  Specifies the name of layer (aka window) whose logs to show. Use
                    unique names of layers to avoid undefined behaviour. You can also
                    define the layer before like with `update` subcommand. Currently,
                    logs can't be specified on the basis of layer_name.
    --help | -h :   Shows this help message.
";

pub const FPRINT_HELP: &str = "
Usage: sp fprint [<argument>] [sub-command]

`fprint` subcommand is used to enroll, verify and list fingerprints.

Sub-commands:
    enroll: Specify the finger and register its fingerprint. Make Sure that a polkit
            agent and fprintd service are up and running.
    verify: Opens the sensor for verification for a fingerprint.
    list:   (WIP) Lists available fingerprint sensors along with enrolled fingerprints.
Argument:
    --help | -h :   Shows this help message.
";

pub const ENABLE_HELP: &str = "
Usage: sp enable [<argument>] [sub-command] ...

`enable` subcommand could be used in order to trigger events for vault. Complete
implementation willl come in upcoming versions.
";

pub const SPELL_PAM_FPRINT: &str = r#"Make sure that a polkit agent is up and running!!
Also `login` file in `/etc/pam.d/` should have following line in top below comments:
auth      sufficient pam_fprintd.so
"#;
