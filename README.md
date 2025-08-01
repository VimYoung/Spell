# Spell

Spell is a framework that provides necessary tooling to create highly customisable,
shells for your wayland compositors (like hyprland) using Slint UI.

Rather then leveraging Gtk for widget creation, Slint declarative language provides
a very easy but comprehensive way to make aesthetic interfaces. It, supports rust
as backend, so as though there are not many batteries (for now) included
in the framework itself, everything can be brought to life from the dark arts of
rust.

## When can we expect a stable release?

No promises but I think I can push it to a release in 3-4 months.

## Inspiration

The project started as a personal repo for my own use. There is lack of widget
creating tools in rust. Secondly, I had a question:
> How the heck wayland works?

So, to understand wayland and side-by-side create a client gave birth to Spell.
I know a lot more about functioning of wayland than I did. Also, a framework
developed that could be delivered in some time for others to use and create widgets
in rust.

## Installation and Usage

> [!WARNING]
> The crate is under active development and is not ready for use.

I have released a version in crates.io . Hence, you can simply add spell by command, `cargo add spell-framework`.

## Why Slint?

Slint because it is a simple yet powerful declarative lang that is extremely
easy to learn (you can even get a sense in just few mins [here](https://docs.slint.dev/latest/docs/slint/guide/language/concepts/slint-language/)). Secondly, unlike
other good UI kits, it just has awesome integration for rust. A compatibility that
is hard to find.

## Todos

A lot of things are left to be done, but following core things are not implemented yet
which might cause problem for widget creation.

1. Resize of buffers aren't possible.
2. It is irrelevant to define `Width` and `Height` of Window, as that
should be provided directly in your `main` function. (Though, I must say that recommended way of creating windows with curved borders is to place a `Rectangle` in a transparent window and then define its `border_radius`.)

Having said that,you should try creating static widgets (like showing clock, day etc) at
this point with `spell` and see how that turns out.

## Minimal Example

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

and then replace the `main.rs` with following contents:

```rust
use std::{
    env,
    error::Error,
    sync::mpsc,
    sync::{Arc, RwLock},
};

use slint::ComponentHandle;
use spell::{
    cast_spell,
    layer_properties::{ForeignController, LayerAnchor, LayerType, WindowConf},
    wayland_adapter::SpellWin,
    Handle,
};
slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let (_tx, rx) = mpsc::channel::<Handle>();
    let window_conf = WindowConf::new(
        376,
        576,
        (Some(LayerAnchor::TOP), Some(LayerAnchor::LEFT)),
        (5, 0, 0, 10),
        LayerType::Top,
        false,
    );
    let (waywindow, event_queue) = SpellWin::invoke_spell("counter-widget", window_conf);

    let ui = AppWindow::new().unwrap();
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });
    cast_spell::<Box<dyn FnMut(Arc<RwLock<Box<dyn ForeignController>>>)>>(
        waywindow,
        event_queue,
        rx,
        None,
        None,
    )
}
```

## Batteries

No batteries, but common functionalities like system tray, temp etc, will be added later for
convenience. I recommend the use of following crates for some basic usage, though you must note
that I haven't used them extensively myself (for now).

1. [sysinfo](https://crates.io/crates/sysinfo): For System info like uptime, cpu, memory usage etc.
2. [rusty-network-manger](https://crates.io/crates/rusty_network_manager): For network management.
3. [bluer](https://docs.rs/bluer/latest/bluer/): For bluetooth management.

## Docs

Docs can be found [here](https://docs.rs/crate/spell-framework/).

## Contributing

The library is still in an early stage. Yet I will encourage you try it out, feel free to open issues and even better PRs for issues. Feature requests can be posted in the issues section itself, but since a lot of things are planned already, they will take a lower priority.
