0.1.5  & 0.1.1 CLI (25-08-19)
================

Changes:

- LOGO in docs and Readme. Also update the readme to include badges for docs and rust.
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
