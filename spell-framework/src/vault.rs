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
//! their corresponding traits (if it applies). The [`Services`] is used to bind
//! and initialise the traits. Why do most utilities have a trait counterpart?
//!
//! Traits are made so as to represent actions when occured from the other server/
//! application side. For example, An [`AppSelector`] is used and initialised for
//! usage by your shell but [`new_app_added`](AppHandler::new_app_added) was called
//! when the coming of a new desktop entry is to be notified. Some utilities like
//! `AppSelector` are more user intensive and less trait intensive(i.e. there are not
//! many cases when server will ping, hence not much methods in traits). On the
//! other hand implementations like that of notifications (via [`NotificationHandler`])
//! are majorly trait intensive. Then, utilities like audio handling (via [`PipeWireHandler`] and
//! [`AudioManager`]) have equal chances of being accessed from anywhere.
//!
//! As a general tip, the best way to implement traits is to stores weak reference to
//! your widget windows on slint side in structs and then implement these traits on it.
use crate::vault::application::desktop_entry_extracter;
use rust_fuzzy_search::fuzzy_search_best_n;
use std::{
    env,
    ffi::OsStr,
    path::{Component, Path, PathBuf},
    sync::Arc,
    thread,
};
mod application;

pub trait NotificationHandler: Send + Sync {}
pub trait AppHandler: Send + Sync {
    fn new_app_added(&mut self, app_data: AppData);
}
pub trait MprisHandler: Send + Sync {}
pub trait PipeWireHandler: Send + Sync {}

#[derive(Default, Clone)]
pub struct Services {
    notification: Option<Arc<dyn NotificationHandler>>,
    app: Option<Arc<dyn AppHandler>>,
    mpris: Option<Arc<dyn MprisHandler>>,
    pipewire: Option<Arc<dyn PipeWireHandler>>,
}

impl Services {
    pub fn add_notification_handle(
        &mut self,
        noti_handler: Arc<dyn NotificationHandler>,
    ) -> &mut Self {
        self.notification = Some(noti_handler);
        self
    }

    pub fn add_app_handle(&mut self, app_handler: Arc<dyn AppHandler>) -> &mut Self {
        self.app = Some(app_handler);
        self
    }

    pub fn add_mpris_handle(&mut self, mpris_handler: Arc<dyn MprisHandler>) -> &mut Self {
        self.mpris = Some(mpris_handler);
        self
    }

    pub fn add_pipewire_handle(&mut self, pipewire_handler: Arc<dyn PipeWireHandler>) -> &mut Self {
        self.pipewire = Some(pipewire_handler);
        self
    }

    pub fn generate_serices(&mut self) {
        if let Some(app) = &self.app {
            let app_clone = app.clone();
            thread::spawn(move || {
                check_for_new_apps(app_clone);
            });
        }
    }
}

fn check_for_new_apps(_app: Arc<dyn AppHandler>) {
    todo!()
}

pub struct AudioManager;
// TODO needs to fix the below mentioned bugs.
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
                                    .collect::<Vec<AppData>>();
                                app_line_data.extend(filtered_data);
                                // if let Some(data) = new_data {
                                //     // if !app_line_data.iter().any(|pushed_data| {
                                //     //     pushed_data.desktop_file_id == data.desktop_file_id
                                //     // }) {
                                //     app_line_data.push(data);
                                //     //}
                                // }
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
