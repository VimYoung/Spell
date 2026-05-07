//! `vault` contains the necessary utilities/APTs for various common tasks required
//! when creating a custom shell. This includes apps, pipewire, PAM, Mpris etc
//! among other things.
//!
//! <div class="warning">
//! For now, this module doesn't contain much utilities. As, more common methods
//! are added, docs will expand to include examples and panic cases.
//! </div>
//!
//! The whole module is divided into structs representing these utilities and
//! their corresponding traits (if it applies). Why do most utilities have a trait counterpart?
//!
//! Traits are made so as to represent actions when occured from the other server/
//! application side. For example, An [`AppSelector`] is used and initialised for
//! usage by your shell. Some utilities like
//! `AppSelector` are more user intensive and less trait intensive(i.e. there are not
//! many cases when server will ping, hence not much methods in traits). On the
//! other hand implementations like that of notifications (via [`NotificationManager`])
//! are majorly trait intensive.
//! As a general tip, the best way to implement traits is to stores weak reference to
//! your widget windows on slint side in structs and then implement these traits on it.
use crate::vault::application::desktop_entry_extracter;
pub use mpris;
pub use notification_manager::set_notification;
pub use rust_fuzzy_search::fuzzy_search_best_n;
use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};
use zbus::blocking::{Connection, Proxy};

mod application;
mod notification_manager;

/// This public static is only set when a notification server instance is passed in
/// [`cast_spell`](crate::cast_spell).
/// It is created to maintain the compliance with freedesktop's desktop notification
/// [specification](https://specifications.freedesktop.org/notification/1.3/index.html).
/// It's method can be called in specific senarios to notify applications that a notification
/// with an id has been closed. This static holds an instance of
/// [`BlockingNotificaiton`](crate::vault::BlockingNotification)
pub static NOTIFICATION_EVENT: OnceLock<BlockingNotification> = OnceLock::new();

/// Holds blocking methods to notify when a notification has bee closed.
#[derive(Default)]
pub struct BlockingNotification;

impl BlockingNotification {
    /// Method to ask the server to emit a signal for closing a particular notificaiton.
    pub fn call_close(
        &self,
        id: u32,
        reason: CloseReason,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::session()?;
        let proxy = Proxy::new(
            &conn,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
        )?;
        proxy.call_noreply("NotificationClosed", &(id, reason as u32))?;
        Ok(())
    }
}

/// This trait's implementation is necessary for passing spell's generated widget
/// into the notification field of [`cast_spell`](crate::cast_spell) macro. It is important to note that
/// implementation of this trait is not on the spell generated widget/window but on the
/// **slint generated window**. For example, a window with name `TopBar` will have a
/// spell implemetation `TopBarSpell`. This trait will be implemented over `TopBar`.
pub trait NotificationManager {
    /// This method is called when a new notification is sent.
    fn new_notification(&self, notification: Notification) -> Result<(), NotiError>;
    /// This method is called when CloseNotification Server method is invoked.
    /// It requres the implementation of the trait to close the notification with
    /// the provided id if it is open.
    fn close_notification(&self, id: u32) -> Result<(), NotiError>;
}

/// Reason to close a notification in [`NOTIFICATION_EVENT`].
#[derive(Debug)]
pub enum CloseReason {
    /// The notification expired
    Expired = 1,
    /// The notification was dismissed by the user
    Dismissed = 2,
    /// The notification was closed by a call to CloseNotification Server method.
    ByCall = 3,
    /// Undefined/reserved reasons
    Undefined = 4,
}

/// Error type used by [`NotificationManager`].
#[derive(Debug)]
pub enum NotiError {
    /// Returned when a new notification can't be handled by the custom implementation.
    MessageUnprocessed,
    /// Returned when a message close request has been failed and the notification
    /// is not closed.
    MessageCloseFailed,
}

/// Object representing a notification.
#[derive(Debug, Clone)]
pub struct Notification {
    /// id of the notification.
    pub id: u32,
    /// Name of app invoking the notification.
    pub appname: String,
    /// Summary (generally main title) of the notification.
    pub summary: String,
    /// Optionaly sub-title of the notification.
    pub subtitle: Option<String>,
    /// Body of the notificaiton.
    pub body: String,
    /// Icon path of the notification.
    pub icon: String,
    /// Hints of the notification. Refer [here](https://specifications.freedesktop.org/notification/1.3/hints.html)
    ///  for more details.
    pub hints: HashMap<String, Hint>,
    /// Specified actions by the notification. Currently partially implemented.
    pub actions: Vec<String>,
    /// Specified timeout in which the notification expects to expire itself.
    pub timeout: Timeout,
}

/// Hints provided by a notification. Refer [here](https://specifications.freedesktop.org/notification/1.3/hints.html)
/// for more details. Currently "image-data" and "image_data" hints are not supported.
#[derive(Debug, Clone)]
pub enum Hint {
    /// When set, a server that has the "action-icons" capability will attempt to
    /// interpret any action identifier as a named icon. The localized display name
    ///  will be used to annotate the icon for accessibility purposes. The icon name
    ///  should be compliant with the Freedesktop.org Icon Naming Specification.
    ActionIcons(bool),
    /// The type of notification this is.
    Category(String),
    /// This specifies the name of the desktop filename representing the  calling
    /// program. This should be the same as the prefix used for the application's
    /// .desktop file. An example would be "rhythmbox" from "rhythmbox.desktop".
    ///  This can be used by the daemon to retrieve the correct icon for the application,
    ///  for logging purposes, etc.
    DesktopEntry(String),
    /// Alternative way to define the notification image. See [Icons and Images](https://specifications.freedesktop.org/notification/1.3/icons-and-images.html).
    ImagePath(String),
    /// When set the server will not automatically remove the notification when
    ///  an action has been invoked. The notification will remain resident in the
    ///  server until it is explicitly removed by the user or by the sender. This
    ///  hint is likely only useful when the server has the "persistence" capability.
    Resident(bool),
    /// The path to a sound file to play when the notification pops up.
    SoundFile(String),
    /// A themeable named sound from the freedesktop.org [sound naming specification](https://0pointer.de/public/sound-naming-spec.html)
    /// to play when the notification pops up. Similar to icon-name, only for sounds. An example would be "message-new-instant".
    SoundName(String),
    /// Causes the server to suppress playing any sounds, if it has that ability.
    /// This is usually set when the client itself is going to play its own sound.
    SuppressSound(bool),
    /// When set the server will treat the notification as transient and by-pass
    ///  the server's persistence capability, if it should exist.
    Transient(bool),
    /// Specifies the X location on the screen that the notification should point to. The "y" hint must also be specified.
    X(i32),
    /// Specifies the Y location on the screen that the notification should point to. The "x" hint must also be specified.
    Y(i32),
    /// The urgency level.
    Urgency(Urgency),
    // Custom(String, String),
    // CustomInt(String, i32),
    // Invalid,
}

/// The proposed urgency level by the notification, implementations of trait [`NotificationManager`]
/// can mark the accent color of their notifications based on this.
#[derive(Debug, Clone)]
pub enum Urgency {
    /// The urgency of the notification is low. Like completion of some unimportant task
    /// by some application.
    Low = 0,
    /// The urgency of the notification is normal. Used by most notifications.
    Normal = 1,
    /// The urgency of the notification is critical. This urgency level is used by
    /// low battery, shutdown related etc notification types.
    Critical = 2,
}

/// Timeout duration for a notification.
#[derive(Debug, Clone)]
pub enum Timeout {
    /// Use server's default duration to close a notification.
    Default,
    /// Don't close the notification until closed by the end user.
    Never,
    /// Close the notification after specified milliseconds.
    Milliseconds(i32),
}

/// AppSelector stores the data for each application with possible actions. Known bugs
/// include failing to open flatpak apps in certain cases and failing to find icons
/// of apps in certain cases both of which will be fixed in coming releases.
#[derive(Debug, Clone)]
pub struct AppSelector {
    /// Storing [`AppData`] in a vector.
    pub app_list: Vec<AppData>,
}

impl Default for AppSelector {
    fn default() -> Self {
        let data_dirs: String =
            env::var("XDG_DATA_DIRS").expect("XDG_DATA_DIRS couldn't be fetched");
        let mut app_line_data: Vec<AppData> = Vec::new();
        let mut data_dirs_vec = data_dirs.split(':').collect::<Vec<_>>();
        // Adding some other directories.
        data_dirs_vec.push("/home/ramayen/.local/share/");
        for dir in data_dirs_vec.iter() {
            // To check if the directory mentioned in var actually exists.
            if Path::new(dir).is_dir() {
                for inner_dir in Path::new(dir)
                    .read_dir()
                    .expect("Couldn't read the directory")
                    .flatten()
                {
                    // if let Ok(inner_dir_present) = inner_dir {
                    if *inner_dir
                        .path()
                        .components()
                        .collect::<Vec<_>>()
                        .last()
                        .unwrap()
                        == Component::Normal(OsStr::new("applications"))
                    {
                        let app_dir: PathBuf = inner_dir.path();
                        for entry_or_dir in
                            app_dir.read_dir().expect("Couldn't read app dir").flatten()
                        {
                            if entry_or_dir.path().is_dir() {
                                println!("Encountered a directory");
                            } else if entry_or_dir.path().extension() == Some(OsStr::new("desktop"))
                            {
                                let new_data: Vec<Option<AppData>> =
                                    desktop_entry_extracter(entry_or_dir.path());
                                let filtered_data: Vec<AppData> = new_data
                                    .iter()
                                    .filter_map(|val| val.to_owned())
                                    .filter(|new| {
                                        !app_line_data.iter().any(|existing| {
                                            existing.desktop_file_id == new.desktop_file_id
                                        })
                                    })
                                    .collect();
                                app_line_data.extend(filtered_data);
                            } else if entry_or_dir.path().is_symlink() {
                                println!("GOt the symlink");
                            } else {
                                // println!("Found something else");
                            }
                        }
                    }
                }
            }
        }

        AppSelector {
            app_list: app_line_data,
        }
    }
}

impl AppSelector {
    /// Returns an iterator over primary enteries of applications.
    pub fn get_primary(&self) -> impl Iterator<Item = &AppData> {
        self.app_list.iter().filter(|val| val.is_primary)
    }

    /// Returns an iterator of all enteries of all applications.
    pub fn get_all(&self) -> impl Iterator<Item = &AppData> {
        self.app_list.iter()
    }

    /// Returns an iterator over the most relevent result of applications' primary enteries
    /// for a given string query. `size` determines the number of enteries to
    /// yield.
    pub fn query_primary(&self, query_val: &str, size: usize) -> Vec<&AppData> {
        let query_val = query_val.to_lowercase();
        let query_list = self
            .app_list
            .iter()
            .filter(|val| val.is_primary)
            .map(|val| val.name.to_lowercase())
            .collect::<Vec<String>>();
        let query_list: Vec<&str> = query_list.iter().map(|v| v.as_str()).collect();
        let best_match_names: Vec<&str> =
            fuzzy_search_best_n(query_val.as_str(), &query_list, size)
                .iter()
                .map(|val| val.0)
                .collect();
        best_match_names
            .iter()
            .map(|app_name| {
                self.app_list
                    .iter()
                    .find(|val| val.name.to_lowercase().as_str() == *app_name)
                    .unwrap()
            })
            .collect::<Vec<&AppData>>()
    }

    /// Returns an iterator over the most relevent result of all applications' enteries
    /// for a given string query. `size` determines the number of enteries to
    /// yield.
    pub fn query_all(&self, query_val: &str, size: usize) -> Vec<&AppData> {
        let query_val = query_val.to_lowercase();
        let query_list = self
            .app_list
            .iter()
            .map(|val| val.name.to_lowercase())
            .collect::<Vec<String>>();
        let query_list: Vec<&str> = query_list.iter().map(|v| v.as_ref()).collect();
        let best_match_names: Vec<&str> =
            fuzzy_search_best_n(query_val.as_str(), &query_list, size)
                .iter()
                .map(|val| val.0)
                .collect();

        best_match_names
            .iter()
            .map(|app_name| {
                self.app_list
                    .iter()
                    .find(|val| val.name.to_lowercase().as_str() == *app_name)
                    .unwrap()
            })
            .collect::<Vec<&AppData>>()
    }
}

// TODO add representation for GenericName and comments for better searching
/// Stores the relevent data for an application. Used internally by [`AppSelector`].
#[derive(Debug, Clone)]
pub struct AppData {
    /// Unique ID of an application desktop file according to
    /// [spec](https://specifications.freedesktop.org/desktop-entry-spec/latest/file-naming.html#desktop-file-id).
    pub desktop_file_id: String,
    /// Determines if the entry is primary or an action of an application.
    pub is_primary: bool,
    /// Image path of the application if could be fetched.
    pub image_path: Option<String>,
    /// Name of application
    pub name: String,
    /// Execute command which runs in an spaned thread when an application is asked to run.
    pub exec_comm: Option<String>,
}

// TODO have to replace fuzzy search with a custom implementation to avoid dependency.
// There needs to be performance improvements in AppSelector's default implementation
// TODO add an example section in this module with pseudocode for trait implementations.
