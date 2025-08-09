use crate::vault::AppData;
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
pub(super) fn desktop_entry_extracter(file_path: PathBuf) -> Vec<Option<AppData>> {
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
    let mut main_entry_pushed = false;
    let mut return_vector: Vec<Option<AppData>> = Vec::new();
    for line in file_contents.iter() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        } else if line.starts_with("[Desktop Entry]") {
            main_entry_found = true;
        } else if !main_entry_found {
            panic!("Main Entry Not found");
        } else if line.starts_with('[') {
            if !main_entry_pushed {
                main_entry_pushed = true;
                return_vector.push(Some(AppData {
                    desktop_file_id: desktop_file_id.clone(),
                    is_primary: true,
                    image_path: image_path.clone(),
                    name: name.clone(),
                    exec_comm: exec_comm.clone(),
                }));
            } else {
                return_vector.push(Some(AppData {
                    desktop_file_id: desktop_file_id.clone(),
                    is_primary: false,
                    image_path: image_path.clone(),
                    name: format!("{} - {}", return_vector[0].clone().unwrap().name, name),
                    exec_comm: exec_comm.clone(),
                }));
            }
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
                        _ => return vec![None],
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
                "Exec" => exec_comm = Some(get_exec_command(val)),
                "NoDisplay" => {
                    if val == "true" {
                        return vec![None];
                    }
                }
                "Hidden" => {
                    if val == "true" {
                        return vec![None];
                    }
                }
                // TODO This needs to be properly manged in the future.
                "Terminal" => {}
                // TODO Check for other types that can be used.
                _ => {}
            }
        }
    }

    // Pushing the main entry if no sub-sections found.
    if !main_entry_pushed {
        return_vector.push(Some(AppData {
            desktop_file_id: desktop_file_id.clone(),
            is_primary: true,
            image_path: image_path.clone(),
            name: name.clone(),
            exec_comm: exec_comm.clone(),
        }));
    } else {
        // Pushing the last section enteries.
        return_vector.push(Some(AppData {
            desktop_file_id: desktop_file_id.clone(),
            is_primary: false,
            image_path: image_path.clone(),
            name: format!("{} - {}", return_vector[0].clone().unwrap().name, name),
            exec_comm: exec_comm.clone(),
        }));
    }
    // if name.is_empty() || image_path.is_none() {
    if name.is_empty() {
        panic!("Some necessary enteries are not provided by the entry")
    }
    return_vector
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
    // println!("Finding Icon of name: {val} in theme : {theme_name}");
    for dir in &dir_list {
        // dir is some_path/icons here, in which the file is presnet.
        icon_path = find_icon_path(dir, &theme_name, val);
        if icon_path.is_some() {
            break;
        }
    }

    if icon_path.is_none() {
        // println!("SEARCHING IN HICOLOR DEFAULT THEME");
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
                // let pathh = theme_dir.path();
                // println!("GIVEN THEME FOUND, path: {pathh:?}");
                let index_file_path_vec = &Path::new(&theme_dir.path())
                    .read_dir()
                    .expect("Error reading dir")
                    .flatten()
                    .filter(|val| {
                        if val.path().file_name() == Some(OsStr::new("index.theme")) {
                            // println!("INDEX.THEME found");
                            return true;
                        }
                        false
                    })
                    .collect::<Vec<_>>();
                if !index_file_path_vec.is_empty() {
                    // println!("Vector not empty");
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

// TODO Backlash and quoting is not handled properly, needs to be done.
fn get_exec_command(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '%' => {
                if let Some('%') = chars.peek() {
                    // %% -> %
                    output.push('%');
                    chars.next();
                } else {
                    // %X -> remove both
                    chars.next();
                }
            }
            '"' => {
                // Remove double quotes entirely
            }
            _ => {
                output.push(c);
            }
        }
    }

    output.trim().to_owned()
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
