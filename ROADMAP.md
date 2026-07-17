# Roadmap for upcoming releases

This is a dump file where every feature is dumped which pops in my mind and I
find interesting to implement in the spell in near future. Following is the
list in no particular order.

1. [ ] Repeated key implemented for backspace for `SpellWin` and `SpellLock`.
2. [ ] Keyboard enter issue fixed for all layers in multi-window setup.
3. [x] NetworkManger added. **(closed as not planned)**
4. [x] Notification Manager.
5. [x] Bluetooth manager added. **(closed as not planned)**
6. [ ] Process interupt command in Logs.
7. [x] Add some Popup implementation for xdg.
8. [x] Implement multi window adapter in multiple threads for better performance
       in Niri.
9. [x] Call to close widgets by self for widgets like menus.

---

In the Next release (1.0.0 onwards), following things will be included:

1. [x] A macro to better configure and run spell and default implementation of ForeignController
       trait.
2. [x] (Partially implemented for slint but not for rust side) Maybe hot reloading
3. (by trying to integrate subsecond) which I discovered in a rustconf presentation.
4. [x] Making multi-window system multi-threaded for better performance on niri.
5. [ ] Addition of compatibility with clipboard.

---

This is a non-exhaustive list of improvements I want to implement post 1.0.5 as
a lot of above changes are in pipeline or completed:

1. [ ] Fix minor issues with lock screen.
   - [ ] Implement fractional scaling for it.
   - [ ] Fix parallel working of fingerprint and password (individually they
         work fine).
   - [ ] Extend `generate_widgets` macro to include lock screens configurations
         for better APIs.
2. [ ] Performance improvements in SpellWin and subsequently in SpellLock.
   - [ ] I believe that the renderer renders the information partially but
         there is a bool it takes for complete redraws in case of say scaling that
         can be used.
   - [ ] Rendering happens in parts but that information is not used by wayland
         side. Essentially, the whole buffer is damage and redrawn. This can be fixed
         by taking use of the output changed data.
   - [ ] I very much dislike the continuous loop which runs indefinitely to
         check for new event, this leads to continuous consumption of CPU cycles.
         Rather, these things needs to have a waker call from slint. In most practical
         examples, slint serves as primary eventloop to which other backend calls
         (unlike the other way around in my case).
3. [ ] Necessary project re-ordering and cleaning.
   - [ ] re-evaluate functions to move their scopes from `pub(crate)` to `pub(super)`
         for better organisation.
   - [ ] remove `layer_properties` and directly expose `configure`.
   - [ ] privatise the arguments whenever possible.
4. [ ] As for new feature addition, following things are proposed.
   - [ ] New API for adding and removing widgets from the running eventloop.
         It is important for the change below.
   - Develop a Hot plug system for addition and removal of outputs, widgets
     needs to be created and added dynamically to a running event loop for the
     initialisations to work.
