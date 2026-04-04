#[doc = include_str!("../docs/generate_widgets.md")]
#[macro_export]
macro_rules! generate_widgets {
    ($($slint_win:ty),+) => {
        use $crate::wayland_adapter::{WinHandle, SpellWin};
        #[allow(unused_imports)]
        use std::io::Write;
        $crate::macro_internal::paste! {
            $(
                struct [<$slint_win Spell>] {
                    ui: $slint_win ,
                    way: SpellWin,
                }

                impl std::fmt::Debug for [<$slint_win Spell>] {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.debug_struct("Spell")
                        .field("wayland_side:", &self.way) // Add fields by name
                        .finish() // Finalize the struct formatting
                    }
                }

                impl [<$slint_win Spell>] {
                    pub fn invoke_spell(name: &str, window_conf: WindowConf) -> Self {
                        let way_win = SpellWin::invoke_spell(name, window_conf);
                        [<$slint_win Spell>] {
                            ui: $slint_win::new().unwrap(),
                            way: way_win
                        }
                    }
                    /// Internally calls [`crate::wayland_adapter::SpellWin::hide`]
                    pub fn hide(&self) {
                        self.way.hide();
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::show_again`]
                    pub fn show_again(&mut self) {
                        self.way.show_again();
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::toggle`]
                    pub fn toggle(&mut self) {
                        self.way.toggle();
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::grab_focus`]
                    pub fn grab_focus(&self) {
                        self.way.grab_focus();
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::remove_focus`]
                    pub fn remove_focus(&self) {
                        self.way.remove_focus();
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::add_input_region`]
                    pub fn add_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
                        self.way.add_input_region(x, y, width, height);
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::subtract_input_region`]
                    pub fn subtract_input_region(&self, x: i32, y: i32, width: i32, height: i32) {
                        self.way.subtract_input_region(x, y, width, height);
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::add_opaque_region`]
                    pub fn add_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
                        self.way.add_opaque_region(x, y, width, height);
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::subtract_opaque_region`]
                    pub fn subtract_opaque_region(&self, x: i32, y: i32, width: i32, height: i32) {
                        self.way.subtract_opaque_region(x, y, width, height);
                    }

                    /// Internally calls [`crate::wayland_adapter::SpellWin::set_exclusive_zone`]
                    pub fn set_exclusive_zone(&mut self, val: i32) {
                        self.way.set_exclusive_zone(val);
                    }
                    /// Returns a handle of [`crate::wayland_adapter::WinHandle`] to invoke wayland specific features.
                    pub fn get_handler(&self) -> WinHandle {
                        WinHandle(self.way.loop_handle.clone())
                    }

                    pub fn parts(self) -> ($slint_win, SpellWin) {
                        let [<$slint_win Spell>] { ui, way } = self;
                        (ui, way)
                    }
                }

                impl $crate::SpellAssociatedNew for [<$slint_win Spell>] {
                    fn on_call(
                        &mut self,
                    ) -> Result<(), Box<dyn std::error::Error>> {
                        let event_loop = self.way.event_loop.clone();
                        event_loop
                            .borrow_mut()
                            .dispatch(std::time::Duration::from_millis(1), &mut self.way)
                            .unwrap();
                        Ok(())
                    }

                    fn get_span(&self) -> $crate::macro_internal::Span {
                        self.way.span.clone()
                    }
                }

                impl std::ops::Deref for [<$slint_win Spell>] {
                    type Target = [<$slint_win>];
                    fn deref(&self) -> &Self::Target {
                        &self.ui
                    }
                }
            )+
        }
    };
}

#[doc = include_str!("../docs/cast_spell.md")]
#[macro_export]
macro_rules! cast_spell {
    // Single window (non-IPC)
    (
        $win:expr
        $(, notification: $noti:expr)?
        $(,)?
    ) => {{
        let (ui, mut way) = $win.parts();
        let mut windows = Vec::new();
        $(
            let (ui_noti, mut way_noti) = $noti.parts();
            $crate::cast_spell!(@notification way_noti);
            windows.push(Box::new(way_noti) as Box<dyn $crate::SpellAssociatedNew>);
        )?
        $crate::cast_spell!(@expand entry: way, ui);
        windows.push(Box::new(way) as Box<dyn $crate::SpellAssociatedNew>);
        $crate::cast_spells_new(windows)
        // $crate::cast_spell!(@run x)
    }};
    // Single window (IPC)
    (
        ($win:expr, ipc)
        $(, notification: $noti:expr)?
        $(,)?
    ) => {{
        let (ui, mut way) = $win.parts();
        let mut windows = Vec::new();
        $(
            let (ui_noti, mut way_noti) = $noti.parts();
            $crate::cast_spell!(@notification way_noti);
            windows.push(Box::new(way_noti) as Box<dyn $crate::SpellAssociatedNew>);
        )?
        $crate::cast_spell!(@expand entry: (way, ipc), ui: ui );
        windows.push(Box::new(way) as Box<dyn $crate::SpellAssociatedNew>);
        $crate::cast_spells_new(windows)
        // $crate::cast_spell!(@run x)
    }};

    // Multiple windows (mixed IPC / non-IPC) (Defined individually)
    (
        windows: [ $($entry:tt),+ $(,)? ]
        $(, notification: $noti:expr)?
        $(,)?
    ) => {{
        let mut windows = Vec::new();
        let mut _ui_handles: Vec<Box<dyn std::any::Any>> = Vec::new();
        $(
            let (ui_noti, mut way_noti) = $noti.parts();
            $crate::cast_spell!(@notification way_noti);
            windows.push(Box::new(way_noti) as Box<dyn $crate::SpellAssociatedNew>);
        )?
        $(
            let (ui, mut way) = $crate::cast_spell!(@handle_entry $entry);
            _ui_handles.push(Box::new(ui));
            windows.push(Box::new(way) as Box<dyn $crate::SpellAssociatedNew>);
        )+
        $crate::cast_spells_new(windows)
    }};

    // Moved to next release, only for non -ipc scenarios
    // // Multiple windows (mixed IPC / non-IPC) (Defined as non-ipc vector)
    // Older Implementation needs updation
    (
        windows: $windows:expr
        $(, windows_ipc: $windows_ipc:expr)?
        $(, Notification: $noti:expr)?
        $(,)?
    ) => {{
        $(
            $crate::cast_spell!(@notification $noti);
        )?
        $crate::cast_spells_new(windows)
    }};

    // INTERNAL EXPANSION RULES
    // ==================================================

    // IPC-enabled window
    (
        @expand
        entry: ($way:expr, ipc),
        ui: $ui: expr
    ) => {
        let socket_path = format!("/tmp/{}_ipc.sock", $way.layer_name);
        let _ = std::fs::remove_file(&socket_path); // Cleanup old socket
        let listener = std::os::unix::net::UnixListener::bind(&socket_path)?;
        let listener_clone = listener.try_clone().unwrap();
        listener.set_nonblocking(true)?;
        $way.ipc_handler = Some(listener_clone);
        let _ = $way.loop_handle.clone().insert_source(
            $crate::macro_internal::Generic::new(listener, $crate::macro_internal::Interest::READ, $crate::macro_internal::Mode::Level),
            move |_, meta, data| {
                loop {
                    match data.ipc_handler.as_ref().unwrap().accept() {
                        Ok((mut stream, _addr)) => {
                            let mut request = String::new();
                            // tracing::info!("new connection");
                            if let Err(err) = std::io::Read::read_to_string(&mut stream, &mut request) {
                                $crate::macro_internal::warn!("Couldn't read CLI stream");
                            }
                            let (operation, command_args) = request.split_once(" ").unwrap_or((request.trim(), ""));
                            let (command, args) = command_args.split_once(" ").unwrap_or((command_args.trim(), ""));
                            match operation {
                                "hide" => data.hide(),
                                "show" => data.show_again(),
                                "update" => {
                                    IpcController::change_val(& $ui, command, args);
                                }
                                "look"=> {
                                    let returned_type = IpcController::get_type(& $ui,command);
                                    if let Err(_) = stream.write_all(returned_type.as_bytes()) {
                                        // warn!("Couldn't send back return type");
                                    }
                                }
                                // TODO provide mechanism for custom calls from the below
                                // matching.
                                comm => {
                                    IpcController::custom_command(& $ui, comm);
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            break; // drained all pending connections
                        }
                        Err(e) => {
                            // tracing::warn!("Error Reading Socket: {e}");
                            break;
                        }
                    }
                }
                Ok($crate::macro_internal::PostAction::Continue)
            },
        );
    };
    // Non-IPC window
    (
        @expand
        entry: $way:expr,
        $_ui: expr
    ) => {
        // use std::{os::unix::{net::UnixListener, io::AsRawFd}, io::prelude::*};
        let socket_path = format!("/tmp/{}_ipc.sock", $way.layer_name);
        let _ = std::fs::remove_file(&socket_path); // Cleanup old socket
        let listener = std::os::unix::net::UnixListener::bind(&socket_path)?;
        let listener_clone = listener.try_clone().unwrap();
        listener.set_nonblocking(true)?;
        $way.ipc_handler = Some(listener_clone);
        let _ = $way.loop_handle.clone().insert_source(
            $crate::macro_internal::Generic::new(listener, $crate::macro_internal::Interest::READ, $crate::macro_internal::Mode::Level),
            move |_, _, data| {
                loop {
                    match data.ipc_handler.as_ref().unwrap().accept() {
                        Ok((mut stream, _addr)) => {
                            let mut request = String::new();
                            // tracing::info!("new connection");
                            if let Err(err) = std::io::Read::read_to_string(&mut stream, &mut request) {
                                $crate::macro_internal::warn!("Couldn't read CLI stream");
                            }
                            let (operation, command_args) = request.split_once(" ").unwrap_or((request.trim(), ""));
                            // These info statements doesn't seem to be working due to them running in the wrong space.
                            $crate::macro_internal::info!("Operation:{}, command_args:{}", operation, command_args);
                            let (command, args) = command_args.split_once(" ").unwrap_or((command_args.trim(), ""));
                            $crate::macro_internal::info!("Operation:{}, Command {}, args: {}",operation, command, args);
                            match operation {
                                "hide" => data.hide(),
                                "show" => data.show_again(),
                                "update" => {}
                                "look"=> {}
                                _=> {}
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            break; // drained all pending connections;
                        }
                        Err(e) => {
                            panic!("Following error occured.{}",e);
                        }
                    }
                }
                Ok($crate::macro_internal::PostAction::Continue)
            },
        );
    };

    // Compile-time IPC dispatch for the multi-window arm.
    // Both arms return (ui, way) so the caller can keep ui alive in the outer scope.
    // For IPC: ui is moved into the event-loop closure, so clone it first and
    // return the original handle to the caller.
    (@handle_entry ($combowin:expr, ipc)) => {{
        let (ui, mut way) = $combowin.parts();
        // let _ui_for_closure = ui.clone();
        $crate::cast_spell!(@expand entry: (way, ipc), ui: ui);
        (String::from(""), way)
    }};
    (@handle_entry $combowin:expr) => {{
        let (ui, mut way) = $combowin.parts();
        $crate::cast_spell!(@expand entry: way, ui);
        (ui, way)
    }};

    (@parts win: ($combowin:expr , ipc)) => {{
        ($combowin.parts(), true)
    }};
    (@parts win: $combowin:expr) => {{
        ($combowin.parts(), false)
    }};
    // Notification Logic
    (@notification $noti:expr) => {
        // runs ONCE
        let _notification = &$noti;
    };

    (@run $way:expr) => {
        $crate::cast_spell_inner($way)
    };

    // SpellLock Locking
    (lock: $lock:expr) => {
        $crate::cast_spell!(@run $lock)
    };
}
