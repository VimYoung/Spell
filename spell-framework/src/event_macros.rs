#[doc = include_str!("../docs/generate_widgets.md")]
#[macro_export]
macro_rules! generate_widgets {
    ($($slint_win:ty),+) => {
        use $crate::wayland_adapter::{WinHandle, SpellWin};
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
        $(
            $crate::cast_spell!(@notification $noti);
        )?
        let (x,_y) = $crate::cast_spell!(@expand entry: $win);
        println!("{:?}", x);
        $crate::cast_spell!(@run x)
    }};
    // Single window (IPC)
    (
        ($win:expr, ipc)
        $(, notification: $noti:expr)?
        $(,)?
    ) => {{
        $(
            $crate::cast_spell!(@notification $noti);
        )?
        let (x, _y) = $crate::cast_spell!(@expand entry: ($win, ipc));
        println!("{:?}", x);
        $crate::cast_spell!(@run x)
    }};

    // Multiple windows (mixed IPC / non-IPC) (Defined individually)
    (
        windows: [ $($entry:tt),+ $(,)? ]
        $(, notification: $noti:expr)?
        $(,)?
    ) => {{
        $(
            $crate::cast_spell!(@notification $noti);
        )?
        let mut windows = Vec::new();
        $(
            // NOTE that this won't work in case of ipc windows being passed.
            let (way, $crate::cast_spell!(@name $entry)) = $crate::cast_spell!(@expand entry: $entry);
            $crate::cast_spell!(@vector_add windows, way);
        )+
        println!("{:?}", windows);
        $crate::cast_spells_new(windows)
    }};
    //
    // // Multiple windows (mixed IPC / non-IPC) (Defined as non-ipc vector)
    // (
    //     windows: $windows:expr
    //     $(, windows_ipc: $windows_ipc:expr)?
    //     $(, Notification: $noti:expr)?
    //     $(,)?
    // ) => {{
    //     $(
    //         $crate::cast_spell!(@notification $noti);
    //     )?
    //     $crate::cast_spells_new(windows)
    // }};
    //
    // INTERNAL EXPANSION RULES
    // ==================================================

    // IPC-enabled window
    (
        @expand
        entry: ($waywindow:expr, ipc)
    ) => {{
        let socket_path = format!("/tmp/{}_ipc.sock", $waywindow.way.layer_name);
        let _ = std::fs::remove_file(&socket_path); // Cleanup old socket
        let listener = std::os::unix::net::UnixListener::bind(&socket_path)?;
        listener.set_nonblocking(true)?;
        // let handle_weak = $waywindow.ui.as_weak().clone();
        // $waywindow.way.ipc_listener.replace(Some(listener.try_clone().expect("Couldn't clone the listener")));
        let (ui, way) = $waywindow.parts();
        let _ = way.loop_handle.clone().insert_source(
            $crate::macro_internal::Generic::new(listener, $crate::macro_internal::Interest::READ, $crate::macro_internal::Mode::Level),
            move |_, meta, data| {
                println!("{:?}", meta);
                loop {
                    // match data.ipc_listener.borrow().as_ref().unwrap().accept() {
                    match meta.as_ref().accept() {
                        Ok((mut stream, _addr)) => {
                            let mut request = String::new();
                            // tracing::info!("new connection");
                            if let Err(_) = std::io::Read::read_to_string(&mut stream, &mut request) {
                                // tracing::warn!("Couldn't read CLI stream");
                                println!("Biggeest errorrrrr!!!!");
                            }
                            let (operation, command_args) = request.split_once(" ").unwrap_or((request.trim(), ""));
                            let (command, args) = command_args.split_once(" ").unwrap_or((command_args.trim(), ""));
                            match operation {
                                "hide" => data.hide(),
                                "show" => data.show_again(),
                                "update" => {
                                    let returned_type = IpcController::get_type(&ui,command);
                                    if let Err(_) = stream.write_all(returned_type.as_bytes()) {
                                        // warn!("Couldn't send back return type");
                                    }
                                }
                                "look"=> IpcController::change_val(&mut ui, command, args),
                                // TODO provide mechanism for custom calls from the below
                                // matching.
                                _=> {}
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
        (way, ui)
    }};
    // Non-IPC window
    (
        @expand
        entry: $waywindow:expr
    ) => {{
        // use std::{os::unix::{net::UnixListener, io::AsRawFd}, io::prelude::*};
        let socket_path = format!("/tmp/{}_ipc.sock", $waywindow.way.layer_name);
        let _ = std::fs::remove_file(&socket_path); // Cleanup old socket
        let listener = std::os::unix::net::UnixListener::bind(&socket_path)?;
        let listener_clone = listener.try_clone().unwrap();
        listener.set_nonblocking(true)?;
        // let handle_weak = $waywindow.ui.as_weak().clone();
        // $waywindow.way.ipc_listener.replace(Some(listener.try_clone().expect("Couldn't clone the listener")));
        let (ui, mut way) = $waywindow.parts();
        way.ipc_handler = Some(listener_clone);
        let _ = way.loop_handle.clone().insert_source(
            $crate::macro_internal::Generic::new(listener, $crate::macro_internal::Interest::READ, $crate::macro_internal::Mode::Level),
            move |_, _, data| {
                // println!("generic listener {:?}", meta);
                loop {
                    // match data.ipc_listener.borrow().as_ref().unwrap().accept() {
                    match data.ipc_handler.as_ref().unwrap().accept() {
                        Ok((mut stream, _addr)) => {
                            let mut request = String::new();
                            // tracing::info!("new connection");
                            if let Err(err) = std::io::Read::read_to_string(&mut stream, &mut request) {
                                $crate::macro_internal::warn!("Couldn't read CLI stream");
                            }
                                println!("\n\n GIven request {}", request);
                            let (operation, command_args) = request.split_once(" ").unwrap_or((request.trim(), ""));
                            println!("Operation:{}, command_args:{}", operation, command_args);
                            let (command, args) = command_args.split_once(" ").unwrap_or((command_args.trim(), ""));
                            println!("Operation:{}, Command {}, args: {}",operation, command, args);
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

                // $(
                //     // stringify!($name) => handle_weak.unwrap().$name(args.trim().parse::<$ty>().unwrap()),
                //     println!("dcfv {}", stringify!($name));
                // );*
                Ok($crate::macro_internal::PostAction::Continue)
            },
        );
        (way, ui)
    }};
    (@vector_add $wins:expr, ($waywindow:expr, ipc)) => {
        $wins.push(Box::new($waywindow) as Box<dyn $crate::SpellAssociatedNew>)
    };
    (@vector_add $wins:expr, $waywindow:expr) => {
        $wins.push(Box::new($waywindow) as Box<dyn $crate::SpellAssociatedNew>)
    };

    (@name ($win:expr,ipc)) => {
        $crate::macro_internal::paste! {
            [<$win _var>]
        }
    };

    (@name $win:expr) => {
        $crate::macro_internal::paste! {
            [<$win _var>]
        }
    };
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
