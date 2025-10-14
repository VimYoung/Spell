# Roadmap for upcoming releases

This is a dump file where every feature is dumped which pops in my mind and I
find interesting enough to implement in the crate in the future. Following is the
list in no particular order.

1. Repeated key implemented for backspace for `SpellWin` and `SpellLock`.
2. Keyboard enter issue fixed for all layers in multi-window setup.
3. NetworkManger added.
4. Notification Manager.
5. Bluetooth manager added.
6. Process interupt command in Logs.
7. Add some Popup implementation for xdg.
8. Implement multi window adapter in multiple threads for better performance in Niri.
9. Call to close widgets by self for widgets like menus.

======
In the Next release, following things will be included:

1. A macro to better configure and run spell and default implementation of ForeignController
   trait.
2. Maybe hot reloading (by trying to integrate subsecond) which I discovered in
   a rustconf presentation.
3. Making multi-window system multi-threaded for better performance on niri.
4. Hot reloading (by integration of `subsecond` if possible, I havn't done research on integration of it, so it is hard to say.)
