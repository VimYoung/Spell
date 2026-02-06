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

**Don't forget to star the project if you liked it 🌟🌟**

<https://github.com/user-attachments/assets/7e1c6beb-17ad-492c-b7d2-06688cfcbc77>

> This preview is part of a WIP shell I made using Spell called [Young Shell](https://github.com/VimYoung/Young-Shell).

Spell is a framework that provides necessary tooling to create highly customisable,
shells for your wayland compositors (like hyprland) using Slint UI.

> [Here](https://ramayen.netlify.app/#/page/make%20your%20first%20widget%20with%20spell) is a tutorial for new comers to get a hang of spell.

Rather then leveraging Gtk for widget creation, Slint declarative language provides
a very easy but comprehensive way to make aesthetic interfaces. It, supports rust
as backend, so as though there are not many batteries (for now) included
in the framework itself, everything can be brought to life from the dark arts of
rust.

> [!IMPORTANT]
> Please provide your inputs to improve Spell. Don't use 1.0.2. It is unstable and contains
> experimental changes, either wait for 1.0.3 or use 1.0.1 till the new APIs become stable.

## Features 🖊️

1. **Simple frontend with fast backend:** Spell leverages slint for creating widgets,
   which is extremely customisable while being easy to use. Backed by rust, the code remains
   lightweight, memory safe and predictable.
1. **Hot Reload:** Once the size of widget is set. Changes in slint code is reflected
   as is in the widget. Leading to faster iterations on code.
1. **Remote Accessibility:** Spell also ships a CLI to which state of widget can be made accessible,
   enabling integration in compositor settings.
1. **Prebuilt Material Components:** Spell's CLI can port slint's [material components](https://material.slint.dev/)
   to your project, Just add `--material` when creating a starter project with `sp`.
1. **Services:** (WIP) Spell also provides a vault with common functionalities like
   app launcher backend, notification backend, MPRIS etc.

## Minimal Example ✨

> [!NOTE]
> If you don't want to go through the hassle and simply want to jump over to
> analyse the code, a ready-made starter spell project can be made
> with command. Make sure spell-cli is installed.

```
# To install CLI
cargo install spell-cli

# To create a starter project
sp new project-name
```

Create a new project with `cargo new project_name`. Let's start by adding Slint and Spell as dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
slint = { version = "1.13.1", features = ["renderer-software"] }
spell = "1.0.0"

[build-dependencies]
slint-build = "1.13.1"

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
use slint::ComponentHandle;
use spell::{
    cast_spell,
    layer_properties::{ForeignController, LayerAnchor, LayerType, WindowConf, BoardType},
    wayland_adapter::SpellWin,
    Handle,
};
slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Necessary configurations for the widget like dimensions, layer etc.
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
    let waywindow = SpellWin::invoke_spell("counter-widget", window_conf);

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
    cast_spell(waywindow, None, None)
}
```

Running this code with cargo will display a widget in your wayland compositor. It is important to
mention that if you have defined width and height in both your window and in the rust
code,then the renderer will manage the more or less dimensions accordingly, which may lead to undefined behaviour. For details of arguments and use of [`layer_properties::WindowConf`] and [`cast_spell`], head to their respective attached docs.
The same frontend code for this example can also be found in [slint-rust-template](https://github.com/slint-ui/slint-rust-template)

## When can we expect a stable release?

I remember adding this section a few months ago, now I can say that the first stable version is out!!.
Create a spell project and give it a shot.

> [!WARNING]
> There will be some heavy breaking changes for making management of CLI access to
> variables easier. `ForeignController` will essentially be replaced by a macro.
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
