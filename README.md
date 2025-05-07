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
> The crate is under active development and is not read for use. The development will
> be halted for next month or so for academic reasons but I will try to push a stable release
> as soon as possible.

Since, the crate has not yet been published, you can only use it from the github link in
your `Cargo.toml` file.

## Why Slint?

Slint because it is a simple yet powerful declarative lang that is extremely
easy to learn (you can even get a sense in just few mins [here](https://docs.slint.dev/latest/docs/slint/guide/language/concepts/slint-language/)). Secondly, unlike
other good UI kits, it just has awesome integration for rust. A compatibility that
is hard to find.

## Minimal Example

I am creating my own shell from spell, which is currently private and will soon be shown
on display as spell becomes more mature.

## Batteries

No batteries, but common functionalities like system tray, temp etc, will be added later for
convinience.

## Docs
There are no docs now but some docs will be added before a stable release.

## Contributing

I should say that at this point, the crate is not ready for contributions but people can open
issues for suggestions. Bugs and feature-requests will be ignored for now. As soon as a stable
release happens, I will restructure my workflow for issues and PR.
