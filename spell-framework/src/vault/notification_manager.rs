use crate::{
    vault::{NOTIFICATION_EVENT, Notification, NotificationManager, Timeout},
    wayland_adapter::SpellWin,
};
use smithay_client_toolkit::reexports::calloop::channel::{self, Sender};
use std::{
    collections::{HashMap, HashSet},
    result::Result,
};
use tracing::{info, warn};
use zbus::{fdo::Error as BusError, interface, object_server::SignalEmitter};

pub fn set_notification(win: &SpellWin, ui: Box<dyn NotificationManager>) {
    let (sender, rx) = channel::channel::<NotifyEvent>();
    // let (sender_async, rx_async) = channel::channel::<NotifyEvent>();
    // NOTIFICATION_EVENT.get_or_init(|| sender_async);
    let layer_name = win.layer_name.clone();
    let sender_cl = sender.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            //TODO handle and report the error here
            let _ = notification_service_enter(sender_cl, layer_name).await;
        });
    });
    let _ = win
        .loop_handle
        .clone()
        .insert_source(rx, move |event, _, _| match event {
            channel::Event::Msg(msg) => match msg {
                NotifyEvent::Noti(notification) => {
                    if let Err(err) = ui.new_notification(notification) {
                        warn!("{:?}", err);
                    }
                }
                NotifyEvent::NotificationClosed(id) => {
                    if let Err(err) = ui.notifcation_close(id) {
                        warn!("{:?}", err)
                    }
                }
            },
            channel::Event::Closed => info!("Notification Channel to async thread is closed!"),
        });
}

pub enum NotifyEvent {
    Noti(Notification),
    NotificationClosed(u32),
}

async fn notification_service_enter(
    sender: Sender<NotifyEvent>,
    layer_name: String,
) -> zbus::fdo::Result<()> {
    let conn = zbus::Connection::session().await?;
    conn.object_server()
        .at(
            "/org/freedesktop/Notifications",
            NotificationHandler {
                sender: sender.clone(),
                layer_name,
            },
        )
        .await?;
    info!("Object server is setup");
    if let Err(err) = conn.request_name("org.freedesktop.Notifications").await {
        warn!("Error When creating notification crate {:?}", err);
    }
    info!("Notification service is live with the provided name");
    std::future::pending::<()>().await;
    Ok(())
}

struct NotificationHandler {
    sender: Sender<NotifyEvent>,
    layer_name: String,
}

#[interface(name = "org.freedesktop.Notifications", proxy(gen_blocking = false,))]
impl NotificationHandler {
    async fn get_capabilities(&self) -> Result<Vec<String>, BusError> {
        info!("capabilities called");
        Ok(Vec::new())
    }

    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, zbus::zvariant::Value<'_>>,
        expire_timeout: i32,
    ) -> Result<u32, BusError> {
        info!("Notifcation event received");
        let _ = self.sender.clone().send(NotifyEvent::Noti(Notification {
            appname: app_name,
            summary,
            subtitle: None,
            body,
            icon: app_icon,
            hints: HashSet::new(),
            actions: Vec::new(),
            timeout: Timeout::Default,
        }));
        Ok(replaces_id)
    }

    async fn close_notification(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        id: u32,
    ) -> Result<(), BusError> {
        emitter.notification_closed().await?;
        let _ = self
            .sender
            .clone()
            .send(NotifyEvent::NotificationClosed(id));
        Ok(())
    }

    async fn get_server_information(&self) -> Result<(String, String, String, String), BusError> {
        Ok((
            "SpellNC".to_string(),
            "VimYoung".to_string(),
            "0.0.1".to_string(),
            "1.3".to_string(),
        ))
    }

    #[zbus(signal)]
    async fn notification_closed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}
