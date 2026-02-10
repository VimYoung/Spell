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
    new: Create a new spell project with git initialised dependencies added and a
         minimial example to run given the path/name of project.
    enable: Enable services like lockscreen, notification etc if there is a
            provided implementation for it. Run `spell-cli enable --help` for
            more details.
    log: Display the specified logs. If -l is defined defaults to the logs
          of that layer only. Run `spell-cli [-l LAYER_NAME] logs --help` for
          more details.
    list: Lists the running instances of windows created by spell-framework.

Arguments:
    --layer | -l:   Specifies the name of layer (aka window) to be used for specific
                    commands. Use unique names of layers to avoid undefined behaviour.
                    Required by update, look, show and hide sub commands.
    --help | -h:    Shows this help message.
    --version | -v: Displays the version of spell-cli.
";

pub const LOGS_HELP: &str = "
Usage: spell-cli log [<argument>] [sub-command] ...

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
Usage: spell-cli fprint [<argument>] [sub-command]

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
Usage: spell-cli enable [<argument>] [sub-command] ...

`enable` subcommand could be used in order to trigger events for vault. Complete
implementation willl come in upcoming versions.
";

pub const APP_WINDOW_SLINT: &str = r#"import { VerticalBox, Button } from "std-widgets.slint";

export component AppWindow inherits Window {
    in-out property <int> counter: 42;
    callback request-increase-value();
    width: 276px;
    height: 576px;
    VerticalBox {
        Text {
            text: "Counter: \{root.counter}";
        }

        Button {
            text: "Increase value";
            clicked => {
                root.request-increase-value();
            }
        }
    }
}
"#;

pub const BUILD_FILE: &str = r#"fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("cosmic-dark".into());
    // let config = slint_build::CompilerConfiguration::new().with_style("material-dark".into());
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
"#;

pub const CARGO_TOML: &str = r#"
version = "0.1.0"
edition = "2021"

[dependencies]
slint = { version = "1.14.1", features = ["live-preview", "renderer-software"] }
spell-framework = "1.0.1"

[build-dependencies]
slint-build = "1.14.1"

[patch.crates-io]
slint = { git = "https://github.com/slint-ui/slint" }
slint-build = { git = "https://github.com/slint-ui/slint" }
i-slint-core = { git = "https://github.com/slint-ui/slint" }
i-slint-renderer-skia = { git = "https://github.com/slint-ui/slint" }
"#;
pub const MAIN_FILE: &str = r#"use std::{ env, error::Error};

use slint::ComponentHandle;
use spell_framework::{
    cast_spell,
    layer_properties::{BoardType, LayerAnchor, LayerType, WindowConf},
    wayland_adapter::SpellWin,
};
slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let window_conf = WindowConf::new(
        376,
        576,
        (Some(LayerAnchor::TOP), Some(LayerAnchor::LEFT)),
        (5, 0, 0, 10),
        LayerType::Top,
        BoardType::None,
        None,
    );

    let waywindow = SpellWin::invoke_spell("counter-widget", window_conf);
    let ui = AppWindow::new().unwrap();
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });
    cast_spell(waywindow, None, None)
}
"#;

pub const SPELL_PAM_FPRINT: &str = r#"Make sure that a polkit agent is up and running!!
Also `login` file in `/etc/pam.d/` should have following line in top below comments:
auth      sufficient pam_fprintd.so
"#;
