# 1.0.2 & 1.0.2 CLI (25-02-02)

Changes:

- WindowConf creation is replaced by a builder method for cleaner creation of configuration.
  Following new configurations are added.
  - `monitor_name`: specify the monitor to display the widget.
  - `natural_scroll`: Whether to enable natural scrolling. (Default: false)
- Added fingerprint support in SpellLock. Now, You can make your lockscreens open by fingerprint.
- Experimental macro addition (not ready for usage) to combine wayland and slint
  UIs into a single variable. This removes the need to manage two separate variables
  (for wayland and slint separately) for same widget.

Changes(spell-cli):

- Addition of `fprint` subcommand to add enroll, verify and list fingerprints for the
  purpose of being used in SpellLock.

# 1.0.1 & 1.0.1 CLI (25-12-27)

Changes:

- Ported live preview from slint. This feature now enables live preview of widgets
  as they are modified without the need for recompilation every time.
- Added touch input event in Lockscreens.
- Bridged `debug!` messages in slint to show in logs directly in lockscreen like for `SpellWin`.
- Fractional scaling for spell windows in now available making spell experience better for
  large display monitors(probably working but still broken for input events).
- Better error management by replacing `unwrap` with `unwrap_or_else` on necessary places.

Changes(spell-cli):

- The cli has been renamed to `sp`, still the crate is called `spell-cli`(therefore the
  command to download it remains same, i.e. `cargo install spell-cli`). Renaming the cli
  was necessary for a shorter call sign (which is also cooler that writing spell-cli).
- Added a subcommand to create spell project directly with dependencies added and a minimal
  example present.

# 1.0.0 & 1.0.0 CLI (25-10-10)

Changes:

- Multiple dbus for interaction between windows enabled.
  - This resulted in fixing the working of CLI in multi-widget system.
- New method added to set exclusive zone other than window's dimensions.
- `forge` and repeated backspace temporarily disabled in search of better
  solutions and in pipeline macro.
- `debug` statements in your slint code are ported to logs of spell.
- `DataType` matches extended internally with inclusion of new branch for floats.
- CLI: Logs can now be managed from CLI. More information in CLI section.

Changes(spell-cli):

- Logs are implemented to be accessible from CLI.
  - This included creation of space for spell in `$XDG_RUNTIME_DIR`. The socket to which
    messages are sent.

# 0.1.6 & 0.1.2 CLI (25-09-16)

Changes:

- `Handle` is removed in favor of `WinHandle` and `LockHandle` for `SpellWin` and `SpellLock` respectively.This
  significantly reduces necessary call from objects for use and streamlines the process.
- `cast_spell` is made universal for both `SpellWin` and `SpellLock` (`run_lock` is removed).
- Rather than directly handling the events, event_queue is converted to a calloop based event loop. This
  eliminates the need for using mpsc channels in the same thread (which was previously the case).
  - This also involves removing of channels from internal handles which were called through the cli.
- `Forge` is added to enable timed events.
- Lockscreen leading to compositor(Hyprland) black screen error is resolved.
- Print statements are removed in favour of proper log statements for both `SpellWin` and `SpellLock`. Upcoming releases will have a port in CLI to handle the logs and addition of logs in services of vault.
- Repeated key issue and exclusive zone issue is delayed for next release.

Changes(spell-cli):

- Help statements for upcoming log hooks added.

  # 0.1.5 & 0.1.1 CLI (25-08-19)

Changes:

- LOGO in docs and Readme. Also update the readme to include badges for docs and rust.
- Massive performance improvements in rendering per frame for lock, multi-window setup single window setup.
- Added skeletal structure for future implementations of services.
- Added keyboard implementation for non word chars like TAB, ENTER etc.
  - Next release will implement repeated key for backspace.
- Better docs for methods of SpellWin.
- Added a lockscreen implementation with PAM backend APIs authenticate lockscreens.
  - PAM bindings to rust have a known issue with newer versions of clang .I have opened
    a PR for it, patch the package with git path from my profile till the PR is merged. Look
    patches in README (in example) to copy the contents for necessary slint and pam-sys patches.

Changes(spell-cli):

- Added version info in CLI.
- Help statement for main CLI added.

  # 0.1.3 and 0.1.4 (25-08-10)

Changes:

- Added app selector in docs.
  - Now you can fetch your apps and create a app launcher form slint.
- Optimised user-end APIs for less boilerplate.
- Resize functionality is removed in favour of methods to specify input and opaque regions.

  # 0.1.1 and 0.1.2 (25-07-29)

Changes:

- Added Docs for 50% of library approximately.
- A failed attempt to fix docs build breaking.

  # 0.1.0 (2025-07-28)

This is the first publication of spell-framework and spell-cli in crates. They can be inspected on their respective docs pages.
