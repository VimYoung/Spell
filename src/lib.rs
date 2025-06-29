mod configure;
pub mod shared_context;
pub mod slint_adapter;
pub mod wayland_adapter;
pub mod layer_properties {
    pub use crate::{
        configure::WindowConf,
        wayland_adapter::window_state::{DataType, ForeignController},
    };
    pub use smithay_client_toolkit::shell::wlr_layer::Anchor as LayerAnchor;
    pub use smithay_client_toolkit::shell::wlr_layer::Layer as LayerType;
    pub use zbus::fdo::Error as BusError;
}

use smithay_client_toolkit::reexports::client::EventQueue;
<<<<<<< HEAD
use std::{error::Error};
use wayland_adapter::{
    SpellWin,
    window_state::{ForeignController, deploy_zbus_service},
=======
use std::{
    error::Error,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;
use wayland_adapter::{
    SpellWin,
    window_state::{DataType, ForeignController, deploy_zbus_service},
>>>>>>> 99095e1 (Dbus interface implemented)
};

use zbus::Error as BusError;

pub fn cast_spell<F>(
    mut waywindow: SpellWin,
    mut event_queue: EventQueue<SpellWin>,
<<<<<<< HEAD
    mut state: Box<dyn ForeignController>,
) -> Result<(), Box<dyn Error>> {
    tokio::spawn(async move {
        println!("deplied zbus serive in thread");
        deploy_zbus_service(state).await?;
        Ok::<_, BusError>(())
=======
    state: Box<dyn ForeignController>,
    set_callback: &mut F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(Arc<RwLock<Box<dyn ForeignController>>>),
{
    // TODO I don't know but seems like 5 would be a good size given the low size.
    let (tx, mut rx) = mpsc::channel::<(String, DataType)>(20);
    let state = Arc::new(RwLock::new(state));
    let state_clone = state.clone();
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // --TODO unwrap needs to be handled here.

        // TOTHINK result not handled as value this runnin indefinetly.
        let _ = rt.block_on(async move {
            println!("deplied zbus serive in thread");
            deploy_zbus_service(state_clone, tx).await?;
            Ok::<_, BusError>(())
        });
>>>>>>> 99095e1 (Dbus interface implemented)
    });

    loop {
<<<<<<< HEAD
        // Following line does the updates to the buffer. Now those updates
        // needs to be picked by the compositer/windowing system and then
        // displayed accordingly.
        // println!("Running the loop");
=======
        if let Ok((key, data_type)) = rx.try_recv() {
            println!("received event");
            //Glad I could think of this sub scope for RwLock.
            {
                let mut state_inst = state.write().unwrap();
                state_inst.change_val(&key, data_type);
                println!("This block is run");
            }
            set_callback(state.clone());
        };
>>>>>>> 99095e1 (Dbus interface implemented)

        if waywindow.first_configure {
            event_queue.roundtrip(&mut waywindow).unwrap();
        } else {
            // event_queue.flush().unwrap();
            // event_queue.dispatch_pending(&mut waywindow).unwrap();
            event_queue.blocking_dispatch(&mut waywindow).unwrap();
        }
    }
}
// TODO it is necessary to call join unwrap on spawned threads to ensure
// that they are closed when main thread closes.

pub fn get_spell_ingredients(width: u32, height: u32) -> Box<[u8]> {
    let a: u8 = 0xFF;
    // vec![Rgba8Pixel::new(a, 0, 0, 0); width as usize * height as usize].into_boxed_slice()
    vec![a; width as usize * height as usize * 4].into_boxed_slice()
}
