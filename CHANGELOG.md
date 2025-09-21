
W.I.P. 0.1.7 & 0.1.3 CLI {Due Date}
===========================

Changes:

- Multipl dbus for interaction between windows enabled.
  - This resulted in fixing the working of CLI in multi-widget system.
- New method added to set exclusive zone other than window's dimensions.
- CLI: Logs can now be managed from CLI. More information in CLI section.
- Repeated key implemented for backspace for `SpellWin` and `SpellLock`.
- Keyboard enter issue fixed for all layers in multi-window setup.
- NetworkManger added.
- Notification Manager.
- Bluetooth manager added.
- Process interupt command in Logs.

Changes(spell-cli):

- Logs expanded to include user, verbose and developer versions with documentation
  for each.
  - This included creation of space for spell in `$XDG_RUNTIME_DIR`. The socket to which
    messages are sent.

0.1.6 & 0.1.2 CLI (25-09-16)
===========================

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

0.1.5  & 0.1.1 CLI (25-08-19)
================

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

0.1.3 and 0.1.4 (25-08-10)
===============

Changes:

- Added app selector in docs.
  - Now you can fetch your apps and create a app launcher form slint.
- Optimised user-end APIs for less boilerplate.
- Resize functionality is removed in favour of methods to specify input and opaque regions.

0.1.1 and 0.1.2 (25-07-29)
==========================

Changes:

- Added Docs for 50% of library approximately.
- A failed attempt to fix docs build breaking.

0.1.0 (2025-07-28)
==================

This is the first publication of spell-framework and spell-cli in crates. They can be inspected on their respective docs pages.
