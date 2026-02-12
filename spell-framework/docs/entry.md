# Spell

## Introduction

This crate provides the necessary abstractions for [wlr_layer_shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) protocols with an implementation of slint platform backend for creating any and every kind of widget in slint.
So, by the dark arts of spell, one can program their widgets creation in every kind of way suitable to their specific needs. Internally, spell provides a slint [Platform](https://docs.rs/slint/latest/slint/platform/trait.Platform.html) implementation combined with necessary wayland counterparts using [Smithay client toolkit's](https://smithay.github.io/client-toolkit/smithay_client_toolkit/index.html) Wayland bindings in rust.
Apart from that, spell also provides convenience functions and method implementations for various common tasks (like Apps search backend, mpris backend etc) according to [freedesktop specs](https://specifications.freedesktop.org/) standards. Though a lot of them are still partially complete, incomplete or not even started.

<div class="warning">
The crate is under active development and breaking changes are expected. Base functionalities are complete but more goodies need to be added. I am now receiving PRs and Issues now so feel free to fix something or report something that needs to be fixed.
</div>

## Why use Spell and Slint?

It is a necessary question to answer. Spell was created as a personal project to fill a gap, i.e. absence of proper widget making toolkits and frameworks in rust. Moreover, I didn't want to make yet another abstraction over gtk_layer_shell and call it a day. [Slint](https://slint.dev/) is simple yet powerful declarative language which provides excellent support for rust as backend. Spell fills this gap, it implements slint's backend for usage in making widgets. This was rather than bending a verbose language(like rust) to write widgets, people can use slint which was made for it, and still get the excellent support of rust in backend. Another reason was to inspect the inner workings of wayland and how display is managed in linux systems in modern days.

## Panics and Errors

Before starting further, it is important to note that due to nature of rust, some sections become more complicated than they need to be, especially in case of UI related components and libraries. Thus, it is important to read the panic sections (if there is any) of spell objects which you are using, to get a sense of cases in which the code will panic.

## Examples and Basic Usage

Let's first install the CLI of Spell.

```
# To install CLI
cargo install spell-cli

# To create a starter project
sp new project-name
```

`sp` CLI offers this clean way to create a project so you wouldn't have to hustle with
the initial setup. Under the hood, it uses `cargo new` command paired
with filling the files with appropriate content.

Now the main code, following code in you slint file create a counter and initialises it
from default value of 42.

```slint
// In path and file name `ui/app-window.slint`
import { VerticalBox, Button } from "std-widgets.slint";

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
use slint::ComponentHandle;
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
```

Running this code with cargo will display a widget in your wayland compositor. It is important to
mention that if you have defined width and height in both your window and in the rust
code,then the renderer will manage the more or less dimensions accordingly, which may lead to undefined behaviour. For details of arguments and use of [`layer_properties::WindowConf`] and [`cast_spell`], head to their respective attached docs.
The frontend code for this example is adopted from./[slint-rust-template](https://github.com/slint-ui/slint-rust-template)

## Spell CLI

Spell CLI is a utility for managing and handling remotely running windows/widgets. It provides various features like hiding/opening the widget, toggling the widget, remotely setting variables with new/changed values, seeing logs etc. CLI's commands can be combined with your wayland compositor's configuration file to make keybind for your windows. Read more about it in its documentation.

## Inspiration

The inspiration for making spell came from various weird places. First and foremost was my inability to understand the workings of GTK (I didn't spend much time on it), which drifted me away to better alternative,slint. Another reason was to answer the question, "how does wayland actually works?". Moreover, outfoxxed made `quickshell` around the same time, which does a similar thing with Qt and C++, this gave me confidence that a non-gtk framework for widget creation is possible.

## Slint part of Spell

This is future work and here is a note to it. As some more base functionalities will be complete I will also create a slint counterpart of "components" which are most commonly used in the process of widget creation.
