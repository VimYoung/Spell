# Spell

## Introduction

This crate provides the necessary abstractions for [wlr_layer_shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) protocols with an implementation of slint platform backend for creating any and every kind of widget in slint.
So, by the dark arts of spell, one can program their widgets creation in every kind of way suitable to their specific needs. Internally, spell provides a slint [Platform](https://docs.rs/slint/latest/slint/platform/trait.Platform.html) implementation combined with necessary wayland counterparts using [Smithay client toolkit's](https://smithay.github.io/client-toolkit/smithay_client_toolkit/index.html) Wayland bindings in rust.
Apart from that, spell also provides convenience functions and method implementations for various common tasks (like Apps search backend, mpris backend etc) according to [freedesktop specs](https://specifications.freedesktop.org/) standards. Though a lot of them are still partially complete, incomplete or not even started.

<div class="warning">
The crate is under active development and is not ready for full fledged end use. Base functionalities are complete but more goddies needs to be added. I am now receiving PRs and Issues now so feel free to fix something or report something that needs to be fixed.
</div>

## Why use Spell and Slint?

It is a necessary question to answer. Spell was created as a personal project to fill a gap, i.e. absence of proper widget making toolkits and frameworks in rust. Moreover, I didn't want to make yet another abstraction over gtk_layer_shell and call it a day. [Slint](https://slint.dev/) is simple yet powerful declarative language which provides excellent support for rust as backend. Spell fills this gap, it implements slint's backend for usage in making widgets. This was rather than bending a verbose language(like rust) to write widgets, people can use slint which was made for it, and still get the excellent support of rust in backend. Another reason was to inspect the inner workings of wayland and how display is managed in linux systems in modern days, don't talk about Xorg :).

## Panics and Errors

Before starting further, it is important to note that due to nature of rust, some sections become more complicated than they need to be, especially in case of UI related components and libraries. Thus, it is important to read the panic sections (if there is any) of spell objects which you are using, to get a sense of cases in which the code will panic.

## Examples and Basic Usage

Create a new project with `cargo new project_name`. Let's start by adding Slint and Spell as dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
slint = { version = "1.8.0", features = ["renderer-software"] }
spell = { git = "https://github.com/VimYoung/Spell"}

[build-dependencies]
slint-build = "1.8.0"

[patch.crates-io]
slint = { git = "https://github.com/slint-ui/slint" }
slint-build = { git = "https://github.com/slint-ui/slint" }
i-slint-core = { git = "https://github.com/slint-ui/slint" }
i-slint-renderer-skia = { git = "https://github.com/slint-ui/slint" }
```

Since, spell uses some of the private APIs of Slint, it is necessary to provide the above mentioned patches. Build deps are required by slint during compilation process. Moving on, add the `ui` directory (which will store your `.slint` files) in your project root (via command `mkdir ui`). Also add `build.rs` in project root with the following contents for building slint files.

```rust
fn main() {
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
}
```

Now the main juice, let's create a counter widget with a button to increment a count which starts from, say 42.

```slint
// In path and file name `ui/app-window.slint`
export component AppWindow inherits Window {
    in-out property <int> counter: 42;
    callback request-increase-value();
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
```

Now, to increment the data and specify the dimensions of widget add the following to your `src/main.rs` file.

```rust
use std::{
    error::Error,
    sync::mpsc,
    sync::{Arc, RwLock},
};

use slint::ComponentHandle;
use spell::{
    cast_spell,
    layer_properties::{ForeignController, LayerAnchor, LayerType, WindowConf, BoardType},
    wayland_adapter::SpellWin,
    Handle,
};
slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // An rx is necessary to define the remote handle from which the window will
    // be handled from.
    let (_tx, rx) = mpsc::channel::<Handle>();

    // Necessary configurations for the widget like dimensions. layer etc.
    let window_conf = WindowConf::new(
        376,
        576,
        (Some(LayerAnchor::TOP), Some(LayerAnchor::LEFT)),
        (5, 0, 0, 10),
        LayerType::Top,
        BoardType::None,
        false,
    );

    // Getting the window and its event_queue given the properties and a window name.
    let (waywindow, event_queue) = SpellWin::invoke_spell("counter-widget", window_conf);

    // Slint specific code. Like initialising the window.
    let ui = AppWindow::new().unwrap();

    // Setting the callback closure value which will be called on when the button is clicked.
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });

    // Calling the event loop function for running the window
    cast_spell::<Box<dyn FnMut(Arc<RwLock<Box<dyn ForeignController>>>)>>(
        waywindow,
        event_queue,
        rx,
        None,
        None,
    )
}
```

Running this code with cargo will display a widget in your wayland compositor. For details of arguments and use of  [`layer_properties::WindowConf`] and [`cast_spell`], head to their respective attached docs.
For code examples , tips and common functionalities like timed re-running of code, callbacks etc, head over to the book on Spell which contain these guides.
The same frontend code for this example can also be found in [slint-rust-template](https://github.com/slint-ui/slint-rust-template)

## Spell CLI

Spell CLI is a utility for managing and handling remotely running windows/widgets. It provides various features like hiding/opening the widget, toggling the widget, remotely setting variables with new/changed values, seeing logs etc. CLI's commands can be combined with your wayland compositor's configuration file to make keybind for your windows. Read more about it in its documentation.

## Inspiration

The inspiration for making spell came from various weird places. First and foremost was my inability to understand the workings of GTK (I didn't spend much time on it), which drifted me away to better alternative,slint. Another reason was to answer the question, "how does wayland actually works?". Moreover, outfoxxed made `quickshell` around the same time, which does a similar thing with Qt and C++, this gave me confidence that a non-gtk framework for widget creation is possible.

## Slint part of Spell

This is future work and here is a note to it. As some more base functionalities will be complete I will also create a slint counterpart of "components" which are most commonly used in the process of widget creation.

## Rough Content

Spell github link maybe wrong and needs to be fixed.
Add Spell CLI link and instructions to download after publishing the cli.

[positional parameters](std::fmt#formatting-parameters)

This is a [`Handle`]
<div class="warning">
  This is a warning.
</div>
