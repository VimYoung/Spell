# Spell

<img align="right" width="25%" src="https://raw.githubusercontent.com/VimYoung/Spell/main/spell-framework/assets/spell_trans.png">

<h3 align="left">Make desktop widgets by the mystic arts of Spell  !!</h3>
<hr>

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
> Please provide your inputs to improve Spell.

## When can we expect a stable release?

I remember adding this section a few months age, now I can say that the first stable version is out!!.
Add `spell-framework` in your project and give it a shot.

## Inspiration :bulb:

The project started as a personal repo for my own use. There is lack of widget
creating tools in rust. Secondly, I had a question:

> How the heck wayland works?

So, to understand wayland and side-by-side create a client gave birth to Spell.
I know a lot more about functioning of wayland than I did. Also, a framework
developed that could be delivered in some time for others to use and create widgets
in rust.

## Installation and Usage :computer:

> [!WARNING]
> The crate is under active development and breaking changes are expected. Though both single widget
> and multiple widgets event loops works fine, multi-widget gets unstable due to some changes in slint.
> Multiwidget event loops are slow in niri than in hyprland, hide and show features of widgets work
> flawlessly in niri but hangs in hyprland due to an underlying bug.

Efforts are in way to clear out these rough edges. For the time being, you can head over to minimal example
to add appropriate patches and dependencies to use spell with slint.

## Why Slint? :thinking:

Slint because it is a simple yet powerful declarative lang that is extremely
easy to learn (you can even get a sense in just few mins [here](https://docs.slint.dev/latest/docs/slint/guide/language/concepts/slint-language/)). Secondly, unlike
other good UI kits, it just has awesome integration for rust. A compatibility that
is hard to find.

## Minimal Example :sparkles:

I am creating my own shell from spell, which is currently private and will soon be shown
on display as spell becomes more mature. As for producing a minimal example, you can clone
[slint-rust-template](https://github.com/slint-ui/slint-rust-template/blob/main/src/main.rs) and change the name to your preferred name ( don't forget to modify `Cargo.toml`).Then add spell in dependencies
from this github link along with the following patches in the bottom.

```toml
[patch.crates-io]
slint = { git = "https://github.com/slint-ui/slint" }
slint-build = { git = "https://github.com/slint-ui/slint" }
i-slint-core = { git = "https://github.com/slint-ui/slint" }
i-slint-renderer-skia = { git = "https://github.com/slint-ui/slint" }
```

and then replace `main.rs` with following contents:

```rust
use std::{ env, error::Error};
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
```

## Batteries :battery:

Not a lot of batteries included for now, future implementations of common functionalities will occur
in `vault` module of this crate. For now it has a AppSelector, which can be used to retrieve app information
for creating a launcher. Other common functionalities like system tray, temp etc, will be added later for
convenience. I recommend the use of following crates for some basic usage, though you must note
that I haven't used them extensively myself (for now). For roadmap, view [here](https://github.com/VimYoung/Spell/blob/main/ROADMAP.md).

1. [sysinfo](https://crates.io/crates/sysinfo): For System info like uptime, cpu, memory usage etc.
2. [rusty-network-manger](https://crates.io/crates/rusty_network_manager): For network management.
3. [bluer](https://docs.rs/bluer/latest/bluer/): For bluetooth management.

## Contributing :raised_hands:

The library is still in an early stage. Yet I will encourage you try it out, feel free to open issues and even better, PRs for issues. Feature requests can be posted in the issues section itself, but since a lot of things are planned already, they will take a lower priority.

---

Made with ♥️ and Vim.
