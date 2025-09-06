pub const MAIN_HELP: &str = "
Usage: spell-cli [<argument>] [sub-command] [options] ...

spell-cli provides a convenient way to remotely handle windows made by spell-framework.

Sub-commands:
    [-l] update KEY VALUE: Updates the value of given key with the provided value
                            for the specifed layer window. Final implementation
                            depends on your ForeignController trait implementation.
    [-l] look KEY: Fetched the key's value for the specifed layer.
    [-l] show: Shows the specified window if hidden.
    [-l] hide: Hides the specified window.
    enable: Enable services like lockscreen, notification etc if there is a
            provided implementation for it. Run `spell-cli enable --help` for
            more details.
    logs: Display the specified logs. If -l is defined defaults to the logs
          of that layer only. Run `spell-cli [-l LAYER_NAME] logs --help` for
          more details.
    list: Lists the running instances of windows created by spell-framework.

Arguments:
    --layer | -l :  Specifies the name of layer (aka window) to be used for specific commands. Use
                    unique names of layers to avoid undefined behaviour. Required by
                    update, look, show and hide sub commands.
    --help | -h :   Shows this help message.
    --version | -v: Displays the version of spell-cli.
";

pub const LOGS_HELP: &str = "
Usage: spell-cli log [<argument>] [sub-command] ...

`log` subcommand is used to retrieve logging information from currently running widget instances.

Sub-commands:
    slint_debug: links slint's debug!{} method output. You can view your debug
                 statements from it.
    debug: General errors and warning that might be helpful in debugging.
    dump: Performance metric information of spell, generally not needed by
          end user. It's output can be used to point issues.
    dev: Development related logs not intended for end users.

Arguments:
    --layer | -l :  Specifies the name of layer (aka window) whose logs to show. Use
                    unique names of layers to avoid undefined behaviour. You can also
                    define the layer before like with `update` subcommand.
    --help | -h :   Shows this help message.
";
pub const ENABLE_HELP: &str = "";
