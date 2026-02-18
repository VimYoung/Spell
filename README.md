# Spell

 <img align="right" width="25%" src="https://raw.githubusercontent.com/VimYoung/Spell/main/spell-framework/assets/spell_trans.png">

<h3 align="left">Make desktop widgets by the mystic arts of Spell  !!</h3>
<hr>

![rust with crates.io](https://img.shields.io/badge/RUST-CRATES.IO-RED?style=for-the-badge&logo=Rust&logoSize=auto&color=%23fdacac&link=https%3A%2F%2Fcrates.io%2Fcrates%2Fspell-framework)
![docs.rs (with version)](https://img.shields.io/docsrs/spell-framework/latest?style=for-the-badge&logo=docsdotrs&logoSize=auto&label=docs.rs&color=CBF3BB&link=https%3A%2F%2Fdocs.rs%2Fspell-framework%2Flatest%2Fspell_framework%2F)
![GitHub Repo stars](https://img.shields.io/github/stars/VimYoung/Spell?style=for-the-badge&logo=Github&logoSize=auto&color=6AECE1&link=github.com%2FVimYoung%2FSpell)

<p align="left">
  <br />
  <a href="https://github.com/VimYoung/Spell/issues">Report Bug</a>
  ·
  <a href="https://github.com/VimYoung/Spell/discussions">Request Feature</a>
  ·
  <a href="https://docs.rs/spell-framework/latest/spell_framework/">Wiki</a>
  <br />
  <br />
</p>

**Don't forget to star the project if you like it 🌟🌟**

<https://github.com/user-attachments/assets/7e1c6beb-17ad-492c-b7d2-06688cfcbc77>

> This preview is part of a WIP shell I made using Spell called [Young Shell](https://github.com/VimYoung/Young-Shell).

Spell is a framework that provides necessary tooling to create highly customisable,
shells for your wayland compositors (like niri, hyprland) using Slint UI.

> [Here](https://ramayen.netlify.app/#/page/make%20your%20first%20widget%20with%20spell) is a tutorial for new comers to get a hang of spell.

Rather then leveraging Gtk for widget creation.Spell leverages Slint, a declarative
language provides a very easy but comprehensive way to make aesthetic interfaces.
It, supports rust as backend, so as though there are not many batteries (for now)
included in the framework itself, everything can be brought to life from the dark
arts of rust.

> [!IMPORTANT]
> Please provide your inputs to improve Spell 1.0.3 is out but remote IPC access remains broken.
> Ability to pass vectors of ipc and non ipc will be added in the next release.

## Features 🖊️

1. **Simple frontend with fast backend:** As Spell uses Slint for creating widgets,
   which is extremely customisable while being easy to use. Backed by rust, the code remains
   lightweight, memory safe and predictable.
2. **Hot Reload:** Once the size of widget is set. Changes in slint code is reflected
   as is in the widget. Leading to faster iterations of code.
3. **Streamline Project Structure:** Spell doesn't change the project structure
   of slint in any way. So, no new paradigm needs to be learned for working with
   spell.
4. **Remote Accessibility:** Spell also ships a CLI through which state of widget can be made accessible,
   enabling integration in compositor settings.
5. **Prebuilt Components(Material, Vivi):** Spell's CLI can port slint's
   [material components](https://material.slint.dev/) to your project, Just add `--material`
   or `--vivi` when creating a starter project with `sp`.
6. **Services:** (WIP) Spell also provides a vault with common functionalities like
   app launcher backend, notification backend, MPRIS etc.

## Minimal Example ✨

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

## When can we expect a stable release?

I remember adding this section a few months ago, now I can say that the first stable version is out!!.
Create a spell project and give it a shot.

> [!WARNING]
> There are some heavy breaking changes for making CLI/IPC/Remote access to
> variables easier. `ForeignController` essentially replaced by `IpcController`.
> Leading to a much lighter window.

## Caution

> [!WARNING]
> The crate is under active development and breaking changes are expected.

1. Multi-widget gets unstable sometimes due to changes in slint.
2. Multiwidget event loops are slow in niri than in hyprland
3. Hide and show features of widgets work flawlessly in niri but hangs in hyprland due to an underlying [bug](https://github.com/hyprwm/Hyprland/discussions/11654).

Efforts are in way to clear out these rough edges. For the time being, you can head over to minimal example
to add appropriate patches and dependencies to use spell with slint.

## Inspiration 💡

The project started as a personal repo for my own use. There is lack of widget
creating tools in rust. Secondly, I had a question:

> How the heck wayland works?

So, to understand wayland and side-by-side create a client gave birth to Spell.
I know a lot more about functioning of wayland than I did. Also, a framework
developed that could be delivered in some time for others to use and create widgets
in rust.

## Why Slint? 🤔

Slint because it is a simple yet powerful declarative lang that is extremely
easy to learn (you can even get a sense in just few mins [here](https://docs.slint.dev/latest/docs/slint/guide/language/concepts/slint-language/)). Secondly, unlike
other UI toolkits, it has awesome integration for rust. A compatibility that
is hard to find.

## Batteries 🔋

Not a lot of batteries included for now, future implementations of common functionalities will occur
in `vault` module of this crate. For now it has a AppSelector, which can be used to retrieve app information
for creating a launcher. Other common functionalities like system tray, temp etc, will be added later for
convenience. I recommend the use of following crates for some basic usage, though you must note
that I haven't used them extensively myself (for now). For roadmap, view [here](https://github.com/VimYoung/Spell/blob/main/ROADMAP.md).

1. [sysinfo](https://crates.io/crates/sysinfo): For System info like uptime, cpu, memory usage etc.
2. [rusty-network-manger](https://crates.io/crates/rusty_network_manager): For network management.
3. [bluer](https://docs.rs/bluer/latest/bluer/): For bluetooth management.

## Contributing 🙌

The library is still in an early stage. Yet I will encourage you to try it out, feel free to open issues and even better, PRs for issues. Feature requests can be posted in the issues section itself, but since a lot of things are planned already, they will take a lower priority.

---

Made with ♥️ and Vim.
