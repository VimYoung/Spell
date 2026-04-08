use crate::{
    vault::{Notification, NotificationManager},
    wayland_adapter::SpellWin,
};
use smithay_client_toolkit::reexports::calloop::{PostAction, channel};
use tokio::runtime::Runtime;

pub fn set_notification(win: SpellWin, ui: Box<dyn NotificationManager>) {
    let runtime = Runtime::new().unwrap();
    let (sender, rx) = channel::channel::<NotifyEvent>();
    runtime.spawn(async move {});
    let _ = win
        .loop_handle
        .clone()
        .insert_source(rx, move |_, _, data| {
            println!("hello");
            // Ok(PostAction::Continue)
        });
}

enum NotifyEvent {
    Noti(Notification),
}
