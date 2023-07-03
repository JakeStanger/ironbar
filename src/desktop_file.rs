use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::warn;
use walkdir::{DirEntry, WalkDir};

use crate::lock;

type DesktopFile = HashMap<String, Vec<String>>;

lazy_static! {
    static ref DESKTOP_FILES: Mutex<HashMap<PathBuf, DesktopFile>> =
        Mutex::new(HashMap::new());

    /// These are the keys that in the cache
    static ref DESKTOP_FILES_LOOK_OUT_KEYS: HashSet<&'static str> =
        HashSet::from(["Name", "StartupWMClass", "Exec", "Icon"]);
}

/// Finds directories that should contain `.desktop` files
/// and exist on the filesystem.
fn find_application_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"), // system installed apps
        PathBuf::from("/var/lib/flatpak/exports/share/applications"), // flatpak apps
    ];

    let xdg_dirs = env::var_os("XDG_DATA_DIRS");
    if let Some(xdg_dirs) = xdg_dirs {
        for mut xdg_dir in env::split_paths(&xdg_dirs).map(PathBuf::from) {
            xdg_dir.push("applications");
            dirs.push(xdg_dir);
        }
    }

    let user_dir = dirs::data_local_dir(); // user installed apps
    if let Some(mut user_dir) = user_dir {
        user_dir.push("applications");
        dirs.push(user_dir);
    }

    dirs.into_iter().filter(|dir| dir.exists()).collect()
}

/// Finds all the desktop files
fn find_desktop_files() -> Vec<PathBuf> {
    let dirs = find_application_dirs();
    dirs.into_iter()
        .flat_map(|dir| {
            WalkDir::new(dir)
                .max_depth(5)
                .into_iter()
                .filter_map(Result::ok)
                .map(DirEntry::into_path)
                .filter(|file| file.is_file() && file.extension().unwrap_or_default() == "desktop")
        })
        .collect()
}

/// Attempts to locate a `.desktop` file for an app id
pub fn find_desktop_file(app_id: &str) -> Option<PathBuf> {
    // this is necessary to invalidate the cache
    let files = find_desktop_files();

    if let Some(path) = find_desktop_file_by_filename(app_id, &files) {
        return Some(path);
    }

    find_desktop_file_by_filedata(app_id, &files)
}

/// Finds the correct desktop file using a simple condition check
fn find_desktop_file_by_filename(app_id: &str, files: &[PathBuf]) -> Option<PathBuf> {
    let app_id = app_id.to_lowercase();

    files
        .iter()
        .find(|file| {
            let file_name: String = file
                .file_name()
                .expect("file name doesn't end with ...")
                .to_string_lossy()
                .to_lowercase();

            file_name.contains(&app_id)
                || app_id
                    .split(&[' ', ':', '@', '.', '_'][..])
                    .any(|part| file_name.contains(part)) // this will attempt to find flatpak apps that are like this
                                                          // `com.company.app` or `com.app.something`
        })
        .map(ToOwned::to_owned)
}

/// Finds the correct desktop file using the keys in `DESKTOP_FILES_LOOK_OUT_KEYS`
fn find_desktop_file_by_filedata(app_id: &str, files: &[PathBuf]) -> Option<PathBuf> {
    let app_id = &app_id.to_lowercase();
    let mut desktop_files_cache = lock!(DESKTOP_FILES);

    files
        .iter()
        .filter_map(|file| {
            let Some(parsed_desktop_file) = parse_desktop_file(file) else { return None };

            desktop_files_cache.insert(file.clone(), parsed_desktop_file.clone());
            Some((file.clone(), parsed_desktop_file))
        })
        .find(|(_, desktop_file)| {
            desktop_file
                .values()
                .flatten()
                .any(|value| value.to_lowercase().contains(app_id))
        })
        .map(|(path, _)| path)
}

/// Parses a desktop file into a hashmap of keys/vector(values).
fn parse_desktop_file(path: &Path) -> Option<DesktopFile> {
    let Ok(file) = fs::read_to_string(path) else {
        warn!("Couldn't Open File: {}", path.display());
        return None;
    };

    let mut desktop_file: DesktopFile = DesktopFile::new();

    file.lines()
        .filter_map(|line| {
            let Some((key, value)) = line.split_once('=') else { return None };

            let key = key.trim();
            let value = value.trim();

            if DESKTOP_FILES_LOOK_OUT_KEYS.contains(key) {
                Some((key, value))
            } else {
                None
            }
        })
        .for_each(|(key, value)| {
            desktop_file
                .entry(key.to_string())
                .or_insert_with(Vec::new)
                .push(value.to_string());
        });

    Some(desktop_file)
}

/// Attempts to get the icon name from the app's `.desktop` file.
pub fn get_desktop_icon_name(app_id: &str) -> Option<String> {
    let Some(path) = find_desktop_file(app_id) else { return None };

    let mut desktop_files_cache = lock!(DESKTOP_FILES);

    let desktop_file = match desktop_files_cache.get(&path) {
        Some(desktop_file) => desktop_file,
        _ => desktop_files_cache
            .entry(path.clone())
            .or_insert_with(|| parse_desktop_file(&path).expect("desktop_file")),
    };

    let mut icons = desktop_file.get("Icon").into_iter().flatten();

    icons.next().map(std::string::ToString::to_string)
}
