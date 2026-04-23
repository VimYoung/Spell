pub const APP_WINDOW_SLINT_SLEEK: &str = r#"import { VerticalBox } from "std-widgets.slint";
import { UButton } from "@sleek/widgets.slint";

export component AppWindow inherits Window {
    in-out property <int> counter: 42;
    callback request-increase-value();
    width: 276px;
    height: 576px;
    VerticalBox {
        Text {
            text: "Counter: \{root.counter}";
        }

        UButton {
            text: "Increase value";
            clicked => {
                root.request-increase-value();
            }
        }
    }
}
"#;

pub const APP_WINDOW_SLINT_MATERIAL: &str = r#"import { VerticalBox } from "std-widgets.slint";
import { TextButton } from "@material/material.slint";

export component AppWindow inherits Window {
    in-out property <int> counter: 42;
    callback request-increase-value();
    width: 276px;
    height: 576px;
    VerticalBox {
        Text {
            text: "Counter: \{root.counter}";
        }

        TextButton {
            text: "Increase value";
            clicked => {
                root.request-increase-value();
            }
        }
    }
}
"#;

pub const APP_WINDOW_SLINT_SUI: &str = r#"import { VerticalBox } from "std-widgets.slint";
import { SButton } from "@sui/index.slint";

export component AppWindow inherits Window {
    in-out property <int> counter: 42;
    callback request-increase-value();
    width: 276px;
    height: 576px;
    VerticalBox {
        Text {
            text: "Counter: \{root.counter}";
        }

        SButton {
            text: "Increase value";
            clicked => {
                root.request-increase-value();
            }
        }
    }
}
"#;

pub const APP_WINDOW_SLINT_VIVI: &str = r#"import { VerticalBox } from "std-widgets.slint";
import { TextButton } from "@vivi/magic.slint";

export component AppWindow inherits Window {
    in-out property <int> counter: 42;
    callback request-increase-value();
    width: 276px;
    height: 576px;
    VerticalBox {
        Text {
            text: "Counter: \{root.counter}";
        }

        TextButton {
            text: "Increase value";
            clicked => {
                root.request-increase-value();
            }
        }
    }
}
"#;

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
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
"#;

pub const BUILD_FILE_SUI: &str = r#"fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let library_paths =
        std::collections::HashMap::from([("sui".to_string(), manifest_dir.join("ui/surrealism-ui"))]);
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(library_paths);
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
"#;

pub const BUILD_FILE_VIVI: &str = r#"fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let library_paths =
        std::collections::HashMap::from([("vivi".to_string(), manifest_dir.join("ui/vivi/"))]);
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(library_paths);
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
"#;

pub const BUILD_FILE_MATERIAL: &str = r#"fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let library_paths =
        std::collections::HashMap::from([("material".to_string(), manifest_dir.join("ui/material-1.0"))]);
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(library_paths);
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
"#;

pub const BUILD_FILE_SLEEK: &str = r#"fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let library_paths =
        std::collections::HashMap::from([("sleek".to_string(), manifest_dir.join("ui/sleek/widgets.slint"))]);
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(library_paths);
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
"#;

pub const CARGO_TOML: &str = r#"
version = "0.1.0"
edition = "2021"

[dependencies]
slint = { version = "1.14.1", features = ["live-preview", "renderer-software"] }
spell-framework = "1.0.3"

[build-dependencies]
slint-build = "1.14.1"

[patch.crates-io]
slint = { git = "https://github.com/slint-ui/slint" }
slint-build = { git = "https://github.com/slint-ui/slint" }
i-slint-core = { git = "https://github.com/slint-ui/slint" }
i-slint-renderer-skia = { git = "https://github.com/slint-ui/slint" }
"#;
pub const MAIN_FILE: &str = r#"use slint::ComponentHandle;
use spell_framework::{
    self, cast_spell,
    layer_properties::{LayerAnchor, LayerType, WindowConf},
};
use std::{env, error::Error};
slint::include_modules!();
// Generating Spell widgets/windows from slint windows.
spell_framework::generate_widgets![AppWindow];

fn main() -> Result<(), Box<dyn Error>> {
    // Setting configurations for the window.
    let window_conf = WindowConf::builder()
        .width(376_u32)
        .height(576_u32)
        .anchor_1(LayerAnchor::TOP)
        .margins(5, 0, 0, 10)
        .layer_type(LayerType::Top)
        .build()
        .unwrap();

    // Initialising Slint Window and corresponding wayland part.
    let ui = AppWindowSpell::invoke_spell("counter-widget", window_conf);

    // Setting the callback closure value which will be called on when the button is clicked.
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });

    // Calling the event loop function for running the window
    cast_spell!(ui)
}
"#;
