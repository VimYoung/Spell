This macro is responsible for generating Spell compatible types from Slint compatible
types.

This macro takes in one or more Slint windows. Thus, it is important to place
this macro alongside slint's `include_modules` macro.

Example code snippet.

```rust
spell_framework::generate_widgets[AppWindow, BarWindow];
```
