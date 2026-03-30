use crate::vault::AppData;
use core::panic;
use std::{
    env,
    ffi::OsStr,
    fs,
    io::{BufReader, prelude::*},
    path::{Component, Path, PathBuf},
    process::Command,
};

// ─── Constants ───────────────────────────────────────────────────────────────

/// Preferred icon size. No size parameter is exposed; this is resolved
/// internally. Change here if a different default is needed.
const PREFERRED_SIZE: u32 = 48;

/// Extension preference order: PNG first (fast decode, wide support),
/// then SVG (scalable, common in modern themes), then XPM (legacy fallback).
const ICON_EXTENSIONS: &[&str] = &["png", "svg", "xpm"];

// ─── Public entry point (signature unchanged) ────────────────────────────────

/// Resolves an icon name to an absolute file path using the freedesktop
/// icon theme specification lookup algorithm.
///
/// Search order:
///   1. Active GTK icon theme (from gsettings), following Inherits= chains
///   2. hicolor fallback theme
///   3. /usr/share/pixmaps
///
/// Returns None if no icon is found in any location.
pub fn get_image_path(val: &str) -> Option<String> {
    let theme_name = get_icon_theme_name();
    let search_dirs = build_icon_search_dirs();
    lookup_icon(val, &theme_name, &search_dirs).map(|p| p.to_string_lossy().into_owned())
}

// ─── Theme name ──────────────────────────────────────────────────────────────

/// Queries the active icon theme via gsettings. Returns "hicolor" on failure.
fn get_icon_theme_name() -> String {
    let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
    else {
        return "hicolor".to_owned();
    };

    if !out.status.success() {
        return "hicolor".to_owned();
    }

    let raw = String::from_utf8_lossy(&out.stdout);
    let trimmed = raw.trim();

    // gsettings wraps the value in single quotes: 'Adwaita'
    let name = trimmed
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(trimmed);

    if name.is_empty() {
        "hicolor".to_owned()
    } else {
        name.to_owned()
    }
}

// ─── Search directory list ────────────────────────────────────────────────────

/// Builds the ordered list of base icon directories per the spec:
///   $HOME/.icons
///   $XDG_DATA_HOME/icons  (defaults to $HOME/.local/share/icons)
///   $XDG_DATA_DIRS/icons  (defaults to /usr/local/share:/usr/share)
///
/// /usr/share/pixmaps is handled separately as a last-resort fallback.
fn build_icon_search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    if let Ok(home) = env::var("HOME") {
        // $HOME/.icons — kept for backwards compatibility with the spec
        dirs.push(PathBuf::from(format!("{}/.icons", home)));

        let xdg_data_home =
            env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{}/.local/share", home));
        dirs.push(PathBuf::from(format!("{}/icons", xdg_data_home)));
    }

    let xdg_data_dirs =
        env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    for data_dir in xdg_data_dirs.split(':') {
        dirs.push(PathBuf::from(format!("{}/icons", data_dir)));
    }

    dirs
}

// ─── Top-level lookup with theme inheritance ─────────────────────────────────

/// Performs the full icon lookup: active theme chain → hicolor → pixmaps.
fn lookup_icon(icon_name: &str, theme_name: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let mut visited: Vec<String> = Vec::new();

    // 1. Active theme (recursively follows Inherits=)
    if let Some(path) = lookup_in_theme_chain(icon_name, theme_name, search_dirs, &mut visited) {
        return Some(path);
    }

    // 2. hicolor fallback (skip if already visited through inheritance)
    if !visited.iter().any(|v| v == "hicolor") {
        if let Some(path) = lookup_in_theme_chain(icon_name, "hicolor", search_dirs, &mut visited) {
            return Some(path);
        }
    }

    // 3. /usr/share/pixmaps — spec-mandated last resort
    for ext in ICON_EXTENSIONS {
        let path = PathBuf::from(format!("/usr/share/pixmaps/{}.{}", icon_name, ext));
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Searches `theme_name` across all `search_dirs`, then recurses into each
/// theme listed in its `Inherits=` line. `visited` prevents cycles.
fn lookup_in_theme_chain(
    icon_name: &str,
    theme_name: &str,
    search_dirs: &[PathBuf],
    visited: &mut Vec<String>,
) -> Option<PathBuf> {
    if visited.iter().any(|v| v == theme_name) {
        return None;
    }
    visited.push(theme_name.to_owned());

    // Try every base directory for this theme
    for base_dir in search_dirs {
        let theme_path = base_dir.join(theme_name);
        if !theme_path.is_dir() {
            continue;
        }
        if let Some(path) = find_icon_in_theme_dir(&theme_path, icon_name) {
            return Some(path);
        }
    }

    // Follow Inherits= chain declared in index.theme
    for parent_theme in parse_theme_inherits(theme_name, search_dirs) {
        if let Some(path) = lookup_in_theme_chain(icon_name, &parent_theme, search_dirs, visited) {
            return Some(path);
        }
    }

    None
}

// ─── Single-theme directory search ───────────────────────────────────────────

/// Scans all subdirectories of a theme directory for `icon_name`, returning
/// the path whose size is closest to PREFERRED_SIZE.
///
/// Supports both fixed-size (e.g. "48x48/apps") and scalable ("scalable/apps")
/// subdirectories, and all context folders (apps, mimetypes, places, etc.).
fn find_icon_in_theme_dir(theme_path: &Path, icon_name: &str) -> Option<PathBuf> {
    let subdirs = read_theme_directories(theme_path);
    let mut best: Option<(u32, PathBuf)> = None;

    for subdir in &subdirs {
        let dist = size_distance(subdir);
        let subdir_path = theme_path.join(subdir);
        if !subdir_path.is_dir() {
            continue;
        }
        for ext in ICON_EXTENSIONS {
            let candidate = subdir_path.join(format!("{}.{}", icon_name, ext));
            if candidate.exists() {
                let is_better = best.as_ref().map_or(true, |(d, _)| dist < *d);
                if is_better {
                    best = Some((dist, candidate));
                }
                // Exact match — no need to keep scanning
                if dist == 0 {
                    return best.map(|(_, p)| p);
                }
            }
        }
    }

    best.map(|(_, p)| p)
}

// ─── index.theme parsing ─────────────────────────────────────────────────────

/// Returns the subdirectory list from `Directories=` in index.theme.
/// Falls back to a raw filesystem scan if the file is absent or unparseable.
fn read_theme_directories(theme_path: &Path) -> Vec<String> {
    let index_path = theme_path.join("index.theme");

    if let Ok(content) = fs::read_to_string(&index_path) {
        for line in content.lines() {
            if let Some(rest) = line.trim().strip_prefix("Directories=") {
                return rest
                    .split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    // Fallback: treat every subdirectory as a candidate
    scan_theme_subdirs_recursive(theme_path, theme_path)
}

/// Recursively collects subdirectory paths relative to `theme_path`.
/// Stops after two levels (size_dir/context_dir) to match the typical
/// theme layout and avoid excessive traversal.
fn scan_theme_subdirs_recursive(theme_path: &Path, current: &Path) -> Vec<String> {
    let Ok(entries) = current.read_dir() else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(theme_path)
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        if let Some(rel) = relative {
            let depth = rel.chars().filter(|&c| c == '/').count();
            result.push(rel);
            // Recurse one more level so "48x48/apps" is included when
            // the Directories= line lists "48x48/apps" style paths
            if depth == 0 {
                result.extend(scan_theme_subdirs_recursive(theme_path, &entry.path()));
            }
        }
    }
    result
}

/// Reads `Inherits=` from a theme's index.theme, searching across all base
/// directories (a theme may be split across multiple XDG data dirs).
fn parse_theme_inherits(theme_name: &str, search_dirs: &[PathBuf]) -> Vec<String> {
    for base_dir in search_dirs {
        let index_path = base_dir.join(theme_name).join("index.theme");
        let Ok(content) = fs::read_to_string(&index_path) else {
            continue;
        };
        for line in content.lines() {
            if let Some(rest) = line.trim().strip_prefix("Inherits=") {
                return rest
                    .split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

// ─── Size distance ────────────────────────────────────────────────────────────

/// Computes how far a theme subdirectory's size is from PREFERRED_SIZE.
/// Lower is better; 0 is an exact match.
///
/// Handles formats like "48x48", "48x48@2x", "scalable", "scalable@2x",
/// and paths like "48x48/apps" (spec uses bare subdir names in Directories=).
fn size_distance(subdir: &str) -> u32 {
    // Strip context suffix ("48x48/apps" → "48x48") and HiDPI suffix ("@2x")
    let base = subdir
        .split('/')
        .next()
        .unwrap_or(subdir)
        .split('@')
        .next()
        .unwrap_or(subdir);

    if base.eq_ignore_ascii_case("scalable") {
        // Scalable SVGs work at any size. Give them a moderate distance so
        // a correctly-sized PNG is preferred, but SVG beats a wildly
        // mismatched fixed size.
        return PREFERRED_SIZE / 2;
    }

    // Parse "WxH" — only W is used (icons are square)
    if let Some(w_str) = base.split('x').next() {
        if let Ok(w) = w_str.parse::<u32>() {
            return w.abs_diff(PREFERRED_SIZE);
        }
    }

    // Unrecognised format — sort to the back
    u32::MAX
}

// ─── Unchanged functions ──────────────────────────────────────────────────────

// TODO: Backslash and quoting is not handled properly, needs to be done.
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

    // Removing file inputs from flatpak exec commands
    if output.ends_with('@') {
        output = output.chars().take(output.chars().count() - 7).collect();
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
        if index > app_index as usize
            && let Component::Normal(string_val) = part
        {
            return_string_val.push_str(string_val.to_str().unwrap());
            if !part.as_os_str().to_str().unwrap().ends_with(".desktop") {
                return_string_val.push('-');
            }
        }
    }
    return_string_val
}

// TODO: Have to add binding for other side of application for following 2 tasks
//  1. Updating the list if new apps get added (via traits).
//  2. Providing a way to add category to which the apps belong (games, config, tools, etc.)
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
