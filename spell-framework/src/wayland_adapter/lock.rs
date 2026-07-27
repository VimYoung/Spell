use crate::{
    SpellAssociatedNew,
    configure::{LayerConf, set_up_tracing},
    slint_adapter::{SpellLockShell, SpellMultiWinHandler, SpellSkiaWinAdapter},
    wayland_adapter::{
        common::PointerState,
        lock::{self, wayland::SpellSlintLock},
    },
};
use i_slint_core::items::MouseCursor;
use nonstick::{
    AuthnFlags, ConversationAdapter, Result as PamResult, Transaction, TransactionBuilder,
};
use slint::{
    PhysicalSize,
    platform::{Key, WindowAdapter},
};
use smithay_client_toolkit::{
    compositor::CompositorState,
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_session_lock, delegate_shm, delegate_touch,
    output::{self, OutputState},
    reexports::{
        calloop::{
            self, EventLoop, LoopHandle, RegistrationToken,
            channel::{self, Sender},
            timer::{TimeoutAction, Timer},
        },
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_keyboard::WlKeyboard, wl_shm, wl_touch::WlTouch},
        },
    },
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager},
    session_lock::{SessionLock, SessionLockState, SessionLockSurface},
    shm::{
        Shm,
        slot::{Buffer, Slot, SlotPool},
    },
};
use std::{cell::RefCell, process::Command, rc::Rc};
use tracing::{Level, info, span, warn};

mod nonstick_impl;
mod wayland;

/// SpellLock is a struct which represents a window lock. It can be run and initialised
/// on a custom lockscreen implementation with slint.
/// Know limitations include the abscence to verify from fingerprints and unideal issues on
/// multi-monitor setup. You can add the path of binary of your lock in your compositor config and idle
/// manager config to use the program. It will be linked to spell-cli directly in coming releases.
///
/// ## Example
/// Here is a minimal example of rust side, for complete code of slint, check
/// the codebase of young-shell.
///
/// ```rust
/// use spell_framework::cast_spell;
/// use std::{error::Error, sync::{Arc, RwLock}};
/// use slint::ComponentHandle;
/// use spell_framework::{layer_properties::ForeignController, wayland_adapter::SpellLock};
/// slint::include_modules!();
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let lock = SpellLock::invoke_lock_spell();
///     let lock_ui = LockScreen::new().unwrap();
///     let looop_handle = lock.get_handler();
///     lock_ui.on_check_pass({
///         let lock_handle = lock_ui.as_weak();
///         move |string_val| {
///             let lock_handle_a = lock_handle.clone().unwrap();
///             let lock_handle_b = lock_handle.clone().unwrap();
///             looop_handle.unlock(
///                 None,
///                 string_val.to_string(),
///                 Box::new(move || {
///                     lock_handle_a.set_lock_error(true);
///                 }),
///                 Box::new(move || {
///                     lock_handle_b.set_is_lock_activated(false);
///                 }),
///             );
///         }
///     });
///     lock_ui.set_is_lock_activated(true);
///     cast_spell(
///         lock,
///         None,
///         None::<fn(Arc<RwLock<Box<dyn ForeignController>>>)>,
///     )
/// }
/// ```
pub struct SpellLock {
    pub(crate) loop_handle: LoopHandle<'static, SpellLock>,
    pub(crate) conn: Connection,
    pub(crate) compositor_state: CompositorState,
    pub(crate) registry_state: RegistryState,
    pub(crate) output_state: OutputState,
    pub(crate) keyboard_state: Option<WlKeyboard>,
    pub(crate) pointer_state: PointerState,
    pub(crate) touch_state: Option<WlTouch>,
    pub(crate) seat_state: SeatState,
    pub(crate) shm: Shm,
    pub(crate) session_lock: Option<SessionLock>,
    pub(crate) lock_surfaces: Vec<SessionLockSurface>,
    pub(crate) slint_part: Option<SpellSlintLock>,
    pub(crate) is_locked: bool,
    pub span: span::Span,
    pub(crate) unlock_screen: Sender<bool>,
    // TODO, check if it need internal mutability?
    pub(crate) event_loop: Rc<RefCell<EventLoop<'static, SpellLock>>>,
    pub(crate) backspace: Option<RegistrationToken>,
}

impl std::fmt::Debug for SpellLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpellLock")
            .field("is_locked", &self.is_locked)
            .finish()
    }
}

impl SpellLock {
    /// This function creates an instance of SpellLock which can be combined with
    /// slint windows to create a lockscreen.
    pub fn invoke_lock_spell() -> Self {
        let conn = Connection::connect_to_env().unwrap();
        let _ = set_up_tracing("SpellLock");
        let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
        let qh: QueueHandle<SpellLock> = event_queue.handle();
        let registry_state = RegistryState::new(&globals);
        let shm = Shm::bind(&globals, &qh).unwrap();
        let event_loop: EventLoop<'static, SpellLock> =
            EventLoop::try_new().expect("Failed to initialize the event loop!");
        let output_state = OutputState::new(&globals, &qh);
        let session_lock_state = SessionLockState::new(&globals, &qh);
        let compositor_state =
            CompositorState::bind(&globals, &qh).expect("Faild to create compositor state");
        let cursor_manager =
            CursorShapeManager::bind(&globals, &qh).expect("cursor shape is not available");
        let mut win_handler_vec: Vec<(String, (u32, u32))> = Vec::new();
        let lock_surfaces = Vec::new();

        let pointer_state = PointerState {
            pointer: None,
            pointer_data: None,
            cursor_shape: cursor_manager,
            last_cursor_enter_serial: None,
            current_wayland_cursor: MouseCursor::Default,
        };
        let (sender, rx) = channel::channel::<bool>();
        let mut spell_lock = SpellLock {
            loop_handle: event_loop.handle().clone(),
            conn: conn.clone(),
            compositor_state,
            output_state,
            keyboard_state: None,
            touch_state: None,
            pointer_state,
            registry_state,
            seat_state: SeatState::new(&globals, &qh),
            slint_part: None,
            shm,
            session_lock: None,
            lock_surfaces,
            unlock_screen: sender,
            span: span!(Level::INFO, "lock", name = "lock-screen",),
            is_locked: true,
            event_loop: Rc::new(RefCell::new(event_loop)),
            backspace: None,
        };

        let _ = event_queue.roundtrip(&mut spell_lock);

        let session_lock = Some(
            session_lock_state
                .lock(&qh)
                .expect("ext-session-lock not supported"),
        );

        spell_lock.session_lock = session_lock;
        for output in spell_lock.output_state.outputs() {
            let output_info: output::OutputInfo = spell_lock.output_state.info(&output).unwrap();
            let output_name: String = output_info.name.unwrap_or_else(|| "SomeOutput".to_string());
            let dimensions = (
                output_info.logical_size.unwrap().0 as u32,
                output_info.logical_size.unwrap().1 as u32,
            );
            win_handler_vec.push((output_name, dimensions));

            let session_lock = spell_lock.session_lock.as_ref().unwrap();
            let surface = spell_lock.compositor_state.create_surface(&qh);

            // It's important to keep the `SessionLockSurface` returned here around, as the
            // surface will be destroyed when the `SessionLockSurface` is dropped.
            let lock_surface = session_lock.create_lock_surface(surface, &output, &qh);
            spell_lock.lock_surfaces.push(lock_surface);
        }
        let multi_handler = SpellMultiWinHandler::new_lock(win_handler_vec);
        let sizes: Vec<PhysicalSize> = multi_handler
            .borrow()
            .windows
            .iter()
            .map(|(_, conf)| {
                if let LayerConf::Lock(width, height) = conf {
                    PhysicalSize {
                        width: *width,
                        height: *height,
                    }
                } else {
                    panic!("Shouldn't enter here");
                }
            })
            .collect();

        let mut pool = SlotPool::new(
            (sizes[0].width * sizes[0].height * 4) as usize,
            &spell_lock.shm,
        )
        .expect("Couldn't create pool");
        let mut buffer_slots: Vec<RefCell<Slot>> = Vec::new();
        let buffers: Vec<Buffer> = sizes
            .iter()
            .map(|physical_size| {
                let stride = physical_size.width as i32 * 4;
                let (wayland_buffer, _) = pool
                    .create_buffer(
                        physical_size.width as i32,
                        physical_size.height as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    )
                    .expect("Creating Buffer");
                buffer_slots.push(RefCell::new(wayland_buffer.slot()));
                wayland_buffer
            })
            .collect();
        let (slint_event_sender, slint_event_receiver) =
            calloop::channel::channel::<Box<dyn FnOnce() + Send>>();
        let pool: Rc<RefCell<SlotPool>> = Rc::new(RefCell::new(pool));
        let mut adapters: Vec<Rc<SpellSkiaWinAdapter>> = Vec::new();
        buffer_slots
            .into_iter()
            .enumerate()
            .for_each(|(index, slot)| {
                let adapter = SpellSkiaWinAdapter::new(
                    pool.clone(),
                    slot,
                    sizes[index].width,
                    sizes[index].height,
                );
                adapters.push(adapter);
            });

        multi_handler.borrow_mut().adapter = adapters.clone();
        spell_lock.slint_part = Some(SpellSlintLock {
            adapters,
            size: sizes,
            wayland_buffer: buffers,
        });

        spell_lock
            .loop_handle
            .insert_source(slint_event_receiver, |event, _, data| {
                if let calloop::channel::Event::Msg(callback) = event {
                    callback();

                    if let Some(slint_part) = &data.slint_part {
                        for adapter in &slint_part.adapters {
                            adapter.request_redraw();
                        }
                    }
                }
            })
            .unwrap();

        spell_lock.backspace = Some(
            spell_lock
                .loop_handle
                .insert_source(
                    Timer::from_duration(std::time::Duration::from_millis(1500)),
                    |_, _, data| {
                        data.slint_part.as_ref().unwrap().adapters[0]
                            .try_dispatch_event(slint::platform::WindowEvent::KeyPressed {
                                text: Key::Backspace.into(),
                            })
                            .unwrap();
                        TimeoutAction::ToDuration(std::time::Duration::from_millis(1500))
                    },
                )
                .unwrap(),
        );

        let _ =
            spell_lock
                .loop_handle
                .clone()
                .insert_source(rx, move |event, _, data| match event {
                    channel::Event::Msg(msg) => {
                        if msg {
                            if let Some(locked_val) = data.session_lock.take() {
                                locked_val.unlock();
                            } else {
                                warn!("Authentication verified but couldn't unlock");
                            }
                            data.is_locked = false;
                            data.conn.roundtrip().unwrap();
                        }
                    }
                    channel::Event::Closed => {
                        warn!("Unlock channel to open thread is closed.");
                    }
                });

        spell_lock
            .loop_handle
            .disable(&spell_lock.backspace.unwrap())
            .unwrap();
        let _ = slint::platform::set_platform(Box::new(SpellLockShell::new(
            multi_handler,
            slint_event_sender,
        )));

        WaylandSource::new(spell_lock.conn.clone(), event_queue)
            .insert(spell_lock.loop_handle.clone())
            .unwrap();
        spell_lock
    }

    fn converter_lock(&mut self, qh: &QueueHandle<Self>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.slint_part.as_ref().unwrap().size[0].width;
        let height: u32 = self.slint_part.as_ref().unwrap().size[0].height;
        let window_adapter = self.slint_part.as_ref().unwrap().adapters[0].clone();
        let _redraw_val: bool = window_adapter.draw_if_needed();

        let buffer = &self.slint_part.as_ref().unwrap().wayland_buffer[0];
        self.lock_surfaces[0]
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        self.lock_surfaces[0]
            .wl_surface()
            .frame(qh, self.lock_surfaces[0].wl_surface().clone());
        self.lock_surfaces[0]
            .wl_surface()
            .attach(Some(buffer.wl_buffer()), 0, 0);

        self.lock_surfaces[0].wl_surface().commit();
    }

    fn unlock_finger(&mut self, error_callback: Box<dyn FnOnce() + Send>) {
        let sender = self.unlock_screen.clone();
        let span = self.span.clone();
        std::thread::spawn(move || {
            let _guard = span.enter();
            fn unlock_internal(sender: Sender<bool>) -> PamResult<()> {
                let finger = lock::nonstick_impl::FingerprintInfo;
                let output = Command::new("sh")
                    .arg("-c")
                    .arg("last | awk '{print $1}' | sort | uniq -c | sort -nr")
                    .output()
                    .expect("Couldn't retrive username");

                let val = String::from_utf8_lossy(&output.stdout);
                let val_2 = val.split('\n').collect::<Vec<_>>()[0].trim();
                let user_name = val_2.split(" ").collect::<Vec<_>>()[1].to_string();

                let mut txn = TransactionBuilder::new_with_service("login")
                    .username(user_name)
                    .build(finger.into_conversation())?;
                // If authentication fails, this will return an error.
                // We immediately give up rather than re-prompting the user.
                txn.authenticate(AuthnFlags::empty())?;
                txn.account_management(AuthnFlags::empty())?;
                if let Err(err) = sender.send(true) {
                    warn!("Error sending unlock via sender: {err}");
                }
                Ok(())
            }
            if let Err(err) = unlock_internal(sender) {
                warn!("{:?}", err);
                error_callback();
            } else {
                info!("Password passed");
            }
        });
    }

    fn unlock(
        &mut self,
        username: Option<&str>,
        password: &str,
        on_unlock_callback: Box<dyn FnOnce()>,
    ) -> PamResult<()> {
        let user_name;
        if let Some(username) = username {
            user_name = username.to_string();
        } else {
            let output = Command::new("sh")
                .arg("-c")
                .arg("last | awk '{print $1}' | sort | uniq -c | sort -nr")
                .output()
                .expect("Couldn't retrive username");

            let val = String::from_utf8_lossy(&output.stdout);
            let val_2 = val.split('\n').collect::<Vec<_>>()[0].trim();
            user_name = val_2.split(" ").collect::<Vec<_>>()[1].to_string();
        }

        let user_pass = lock::nonstick_impl::UsernamePassConvo {
            username: user_name.clone(),
            password: password.into(),
        };

        let mut txn = TransactionBuilder::new_with_service("login")
            .username(user_name)
            .build(user_pass.into_conversation())?;
        // If authentication fails, this will return an error.
        // We immediately give up rather than re-prompting the user.
        txn.authenticate(AuthnFlags::empty())?;
        txn.account_management(AuthnFlags::empty())?;

        on_unlock_callback();
        if let Some(locked_val) = self.session_lock.take() {
            locked_val.unlock();
        } else {
            warn!("Authentication verified but couldn't unlock");
        }
        self.is_locked = false;
        self.conn.roundtrip().unwrap();
        Ok(())
    }

    /// Provides a lockscreen handler used to invoke the unlock
    /// callback with the user entered password.For more details
    /// view [`LockHandle`].
    pub fn get_handler(&self) -> LockHandle {
        LockHandle(self.loop_handle.clone())
    }
}

impl SpellAssociatedNew for SpellLock {
    fn on_call(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = self.event_loop.clone();
        event_loop
            .borrow_mut()
            .dispatch(std::time::Duration::from_millis(1), self)?;
        Ok(())
    }

    fn is_locked(&self) -> bool {
        self.is_locked
    }

    fn get_span(&self) -> span::Span {
        self.span.clone()
    }
}

delegate_keyboard!(SpellLock);
delegate_compositor!(SpellLock);
delegate_output!(SpellLock);
delegate_shm!(SpellLock);
delegate_registry!(SpellLock);
delegate_pointer!(SpellLock);
delegate_touch!(SpellLock);
delegate_session_lock!(SpellLock);
delegate_seat!(SpellLock);

/// Struct to handle unlocking of a SpellLock instance. It can be captured from
/// [`SpellLock::get_handler`].
#[derive(Debug, Clone)]
pub struct LockHandle(LoopHandle<'static, SpellLock>);

impl LockHandle {
    /// Call this method to unlock Spelllock. It also takes two callbacks which
    /// are invoked when the password parsed is wrong or right (i.e. resulting
    /// in an screen unlock) respectively. Callbacks can be used to invoke UI
    /// specific changes for your slint frontend.
    pub fn unlock(
        &self,
        username: Option<String>,
        password: String,
        on_err_callback: Box<dyn FnOnce()>,
        on_unlock_callback: Box<dyn FnOnce()>,
    ) {
        self.0.insert_idle(move |app_data| {
            if app_data
                .unlock(username.as_deref(), &password, on_unlock_callback)
                .is_err()
            {
                on_err_callback();
            }
        });
    }

    /// Function which opens fingerprint device for authentication.
    /// error_callback is executed when fingerprint is not registered and fails
    /// to unlock the lockscreen.
    pub fn verify_fingerprint(&self, error_callback: Box<dyn FnOnce() + Send>) {
        self.0.insert_idle(move |app_data| {
            app_data.unlock_finger(error_callback);
        });
    }
}
