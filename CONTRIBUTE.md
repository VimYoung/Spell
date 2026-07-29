# CONTRIBUTE

Thanks for deciding to take out time to contribute to Spell. It is always great
to get any minimum help. Before starting make ensure that:

1. The issue is not present in ROADMAP because if it is, t is quite possible I
   might be having some direction as how to go about solving it. So, you can open
   a discussion for discussion and ensuring that no development on it is actively
   going on.
2. The problem you are working on has an related open issue. It is
   always better to have discussed the issue and proposed approach regarding the
   solution. It will save both your and my(maintainer) time from wandering in the
   wild.
3. You have rust and it's related tooling installed and working. (obviously)

Now, below is the project structure of Spell which will give you some sense as
to where the relevant code will be.

## Project structure

The repository is a rust workspace divided into 3 parts.`spell-framework` contains the code
for the library, `spell-cli` is the CLI's code and `spell-demo` contains examples
about the use of library. Let's talk about each of them:

### spell-framework

The library/crate is divided into two parts: Slint UI's backend & renderer side
and wayland side of configurations and management. Hence, the code for slint goes
in `slint_adapter` and `wayland_adapter`. Following files contain other related
content:

1. `event-macros`: It stitches wayland and slint internals with procedural function-like
   macros so as to provide a unified interface for operation. Hence, all important
   objects like `SpellWin`, `SpellLock` etc are never directly dealt with.
2. `configure`: It contains the objects which are used in configuration and initialisation
   of wayland objects. It is re-exported publicly as `layer_properties` module.
3. `sika*-`: It has skia rendered related types and objects used by `slint_adapter`.

`wayland_adapter` in itself doesn't contains any code, it reexports spell wayland
objects defined in inner modules (like `lock`, `window` etc). It is important to
note the following about the structure of modules of spell objects.

1. **The root file :** It contains the code for all the public and constructor
   method. Any repeated code or helper function is moved to `internal` file.
2. **`internal.rs` :** contains helper function and inner functionality code for
   the object.
3. **`wayland.rs` :** This file contains all the wayland specific code and trait
   implementation required for things like getting memory, fractional scaling,
   interaction with compositor etc. The only exception is input related trait
   implementations.
4. **`input.rs`:** It contains user input related trait implementations in order
   of touch, pointer and keyboard.

Apart from this, module specific functionality go into it's own files/modules.
functionality common to more than one spell object is moved to `common.rs` to reduce
code duplication. If adding public methods, make sure to include documentation
and examples wherever is necessary. If you are creating a new object, make sure
this structure is followed.

### spell-cli

All the code for CLI is in `main.rs` as it isn't large amounts of
code that need to be restructured into modules. Documentation strings shown by `--help`
argument is present in `constantvals.rs`. Static code written in files when a project
is created with an inbuilt component lib is sourced from zips in `component_libs`
folder and text from `constant_files.rs`.

## After making the changes

Spell doesn't have a test suite to check for regressions, but make sure that all
the examples in `spell-demo` are working as expected before submitting the code.

## Not Sure

If you didn't find something you were looking for, reach out to suggest fixes in
this doc and ask for any more clarification on anything. I will be happy to help.
