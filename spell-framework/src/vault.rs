use core::panic;
use std::{
    env,
    ffi::OsStr,
    fs,
    io::{BufReader, prelude::*},
    path::{Component, Path, PathBuf},
    process::Command,
    str::FromStr,
};
#[derive(Debug, Clone)]
pub struct AppSelector {
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
                                let new_data: Option<AppData> =
                                    desktop_entry_extracter(entry_or_dir.path());
                                if let Some(data) = new_data {
                                    // if !app_line_data.iter().any(|pushed_data| {
                                    //     pushed_data.desktop_file_id == data.desktop_file_id
                                    // }) {
                                    app_line_data.push(data);
                                    //}
                                }
                            } else if entry_or_dir.path().is_symlink() {
                                println!("GOt the symlink");
                            } else {
                                println!("Found something else");
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

// TODO add representation for GenericName and comments for better searching
#[derive(Debug, Clone)]
pub struct AppData {
    pub desktop_file_id: String,
    pub image_path: Option<String>,
    pub name: String,
    pub exec_comm: Option<String>,
}

fn desktop_entry_extracter(file_path: PathBuf) -> Option<AppData> {
    let desktop_file_id: String = get_desktop_id(&file_path);
    let mut image_path: Option<String> = None;
    let mut name = String::new();
    let mut exec_comm = None;
    let file = fs::File::open(&file_path).unwrap();
    let buf = BufReader::new(file);
    let file_contents: Vec<String> = buf
        .lines()
        .map(|l| l.expect("Could not parse line"))
        .collect();
    let mut main_entry_found = false;
    for (_line_index, line) in file_contents.iter().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        } else if line.starts_with("[Desktop Entry]") {
            main_entry_found = true;
        } else if !main_entry_found {
            panic!("Main Entry Not found");
        } else if line.starts_with('[') {
            // TODO Other sections are not yet handled
        } else {
            // Return None if Hidden or NoDisplay
            let (key, val) = line.split_once('=').unwrap(); //unwrap_or_else(|| {
            //     panic!("More than one equals found {file_path:?}, line: {line_index}",)
            // });
            // TODO val can also take extra more than one value seperated by `;`, needs to be
            // handled. Escape sequence and data types needs to be managed too.
            // Localised values of keys also needs to be manged.
            match key {
                "Type" => {
                    match val {
                        "Application" => {}
                        // TODO manage the other types propely
                        _ => return None,
                    }
                }
                "Name" => name = val.to_string(),
                "Icon" => {
                    if val.contains('/') {
                        image_path = Some(val.to_string());
                    } else {
                        image_path = get_image_path(val);
                    }
                }
                "Exec" => exec_comm = Some(get_execute_command(val)),
                "NoDisplay" => {
                    if val == "true" {
                        return None;
                    }
                }
                "Hidden" => {
                    if val == "true" {
                        return None;
                    }
                }
                // TODO This needs to be properly manged in the future.
                "Terminal" => {}
                // TODO Check for other types that can be used.
                _ => {}
            }
        }
    }
    // if name.is_empty() || image_path.is_none() {
    if name.is_empty() {
        panic!("Some necessary enteries are not provided by the entry")
    }
    Some(AppData {
        desktop_file_id,
        image_path,
        name,
        exec_comm,
    })
}

// Image path implementation as mentioned in https://specifications.freedesktop.org/icon-theme-spec/latest/#example
fn get_image_path(val: &str) -> Option<String> {
    let mut theme_name: String = "hicolor".to_string();
    let theme_name_getter = Command::new("gsettings")
        .arg("get")
        .arg("org.gnome.desktop.interface")
        .arg("icon-theme")
        .output()
        .expect("Failed to execute command gsettings.");
    if theme_name_getter.status.success() {
        theme_name = String::from_utf8(theme_name_getter.stdout).expect("Failed to process theme");
        if theme_name.starts_with('\'') {
            theme_name.pop();
            theme_name.pop();
            theme_name.remove(0);
        }
    }
    // $HOME/.icons (for backwards compatibility), in $XDG_DATA_DIRS/icons and in /usr/share/pixmaps
    let mut dir_list = Vec::new();
    if let Ok(home_val) = env::var("HOME") {
        dir_list.push(home_val);
        dir_list.extend(
            env::var("XDG_DATA_DIRS")
                .expect("XDG_DATA_DIRS not set")
                .split(':')
                .map(|val| val.to_owned() + "/icons/"),
        );
        dir_list.push("/usr/share/pixmaps/".to_string());
    }
    let mut icon_path: Option<String> = None;
    println!("Finding Icon of name: {val} in theme : {theme_name}");
    for dir in &dir_list {
        // dir is some_path/icons here, in which the file is presnet.
        icon_path = find_icon_path(dir, &theme_name, val);
        if icon_path.is_some() {
            break;
        }
    }

    if icon_path.is_none() {
        println!("SEARCHING IN HICOLOR DEFAULT THEME");
        for dir in dir_list {
            icon_path = find_icon_path(&dir, "hicolor", val);
            if icon_path.is_some() {
                break;
            }
        }
    }
    icon_path
}

fn find_icon_path(dir: &str, theme_name: &str, icon_name_given: &str) -> Option<String> {
    if Path::new(dir).is_dir() {
        for theme_dir in Path::new(dir)
            .read_dir()
            .expect("Unable to read dir")
            .flatten()
        {
            if theme_dir.path().iter().next_back() == Some(OsStr::new(theme_name)) {
                let pathh = theme_dir.path();
                println!("GIVEN THEME FOUND, path: {pathh:?}");
                let index_file_path_vec = &Path::new(&theme_dir.path())
                    .read_dir()
                    .expect("Error reading dir")
                    .flatten()
                    .filter(|val| {
                        if val.path().file_name() == Some(OsStr::new("index.theme")) {
                            println!("INDEX.THEME found");
                            return true;
                        }
                        false
                    })
                    .collect::<Vec<_>>();
                if !index_file_path_vec.is_empty() {
                    println!("Vector not empty");
                    let index_file_path = &index_file_path_vec[0];
                    let icon_path: Option<String> =
                        get_path_from_index(index_file_path.path(), icon_name_given);
                    if icon_path.is_some() {
                        return icon_path;
                    }
                }
            }
        }
    }
    None
}

fn remove_last(path: &Path) -> PathBuf {
    let val = String::from(path.as_os_str().to_str().unwrap());
    let mut pathh = val.split('/').collect::<Vec<_>>();
    pathh.pop();
    let binding = pathh.join("/");
    PathBuf::from_str(&binding).expect("Couldn't remove entry")
}

// TODO, future implementation will encoporate scale and size gathered from
// user along with use of index.theme file
fn get_path_from_index(theme_index_path: PathBuf, icon_name: &str) -> Option<String> {
    let dir_path = remove_last(&theme_index_path);
    if dir_path.is_dir() {
        println!("Finding icon path in {dir_path:?} for icon name: {icon_name}");
        for scale_size_dir in dir_path.read_dir().expect("Error reading dir").flatten() {
            let pathh = remove_last(&scale_size_dir.path());
            for dir in pathh
                .read_dir()
                .expect("Couldn't read icon scaled file")
                .flatten()
            {
                // let some_path = dir.path();
                // println!("{some_path:?}");
                if dir.path().iter().next_back() == Some(OsStr::new("24x24")) {
                    for app_dir in dir
                        .path()
                        .read_dir()
                        .expect("Error reading scaled value dir")
                        .flatten()
                    {
                        if app_dir.path().iter().next_back() == Some(OsStr::new("apps")) {
                            // let app_dir_path = app_dir.path();
                            // println!("{app_dir_path:?}");
                            let final_app_path_string =
                                &(dir.path().to_str().unwrap().to_owned() + "/apps");
                            let apps_folder = Path::new(final_app_path_string);
                            println!("ankdnvkn{apps_folder:?}");
                            for icon_path in apps_folder
                                .read_dir()
                                .expect("Error reading apps folder")
                                .flatten()
                            {
                                // let app_dir_path = icon_path.path();
                                // println!("{icon_name}");
                                // println!("icon paths: {app_dir_path:?}");
                                if icon_path.path().file_stem().unwrap() == OsStr::new(icon_name) {
                                    return Some(String::from(
                                        icon_path.path().as_os_str().to_str().unwrap(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_execute_command(val: &str) -> String {
    "some_val".to_string()
}

fn get_desktop_id(file_path: &Path) -> String {
    let mut return_string_val = String::new();
    let mut app_index: i32 = -1;
    for (index, part) in file_path.components().enumerate() {
        if part == Component::Normal(OsStr::new("applications")) {
            app_index = index as i32;
        }
        if index > app_index as usize {
            if let Component::Normal(string_val) = part {
                return_string_val.push_str(string_val.to_str().unwrap());
                if !part.as_os_str().to_str().unwrap().ends_with(".desktop") {
                    return_string_val.push('-');
                }
            }
        }
    }
    return_string_val
}

// TODO have to add break statements in for loops of above functions whene the desired
// folder is found.
// TODO fix image path function so that it can find icons if app not present in /apps/
