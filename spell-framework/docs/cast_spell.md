It is a macro which builds upon the internal functions of spell to create and start
an event loop. This macro is responsible for the creation of necessary bindings before
running the code.

`cast_spell` uses a combination of internal functions that are not to be used by
the end user. This macro also generates the code for IPC control. This macro is created so
that addition of optional arguments can take place easily. Hence, one could
optionally define these adapters and listeners (unlike previously when everything
was passed to a function wrapped in `Option`s).
This page defines the various ways in which the macro is capable of taking in windows.

It is important to note that passing values that are not documented here can lead to
unexpected issues and errors.

## Single window

```rust
// window here is generated from generate_widgets macro.
cast_spell!(winodw)
cast_spell!((window, ipc))
```

If you want to specify that window also supports remote IPC, then it should be wrapped
in parenthesis like this `(_, ipc)`. In that case, it should also implement
[`IpcController`](crate::IpcController). The most basic way in which spell
can be used is passing a single widget to it. This method is common if you use
spell for a specific widget or you have compressed whole shell in a single widget
(It is possible and rather efficient).

## Multiple windows

```rust
cast_spell!(windows: [win1, win2, (win3, ipc)])
```

This method is used if there are multiple widgets and their number is known during
compilation time. This requires that windows be generated from the `generate_widgets`
macro. You can pass them directly or within parenthesis as written if they implement
IPC trait.
Previously, when vectors were passed in event loops with widgets. It was possible
to dynamically create and add widgets and call them all at once.

You can't do that in this macro as the structs are generated dynamically by [`generate_widgets`](crate::generate_widgets)
and vectors can hold data of same types only. Hence, separate vectors of windows
implementing IPC and not can be passed to maintain backwards compatibility.

```rust
cast_spell!(windows: windows_vector, windows_ipc: windows_ipc_vector)
```

`windows_ipc_vector` here is of type `Vec<Box<dyn IpcController + OtherTrais>>`.
If there is no window implementing [`IpcController`](crate::IpcController),
`windows_ipc` can be ignored but `windows` need to be passed with a vector containing
at least a single widget.

```rust
cast_spell!(windows: windows_vector)
```

## Optional Arguments

This is the biggest reason for using a macro instead of a function. It gives the
flexibility for defining values optionally. Currently, there is only following values
that can be specified. The key and requirement over input values is defined below.

1. `notification`: value should implement [`NotificationManager`](`crate::vault::NotificationManager`).

It is important to note that optional values can only be defined when passing single
and multiple widgets. Example for all the optional values is defined below.

```rust
cast_spell!(window,
  notification: noti_window // Optional
)
```

## Lock screen

Lock screen can also be defined by the macro. Since, the event loop of a lock screen
terminates after teh lock is unlocked. The lock can never be used and initialised
with other optional values and windows. As a result, it is needed to be defined
and used in a separate binary. More on this can be found in documentation of
[`SpellLock`](crate::wayland_adapter::SpellLock). Soon, functionality of lock will
all be include in [`generate_widgets`](crate::generate_widgets) and `SpellLock`
will become internal.

Example code snippet.

```rust
cast_spell!(lock: lock_window)
```
