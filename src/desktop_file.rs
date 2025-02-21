use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tracing::warn;
use walkdir::{DirEntry, WalkDir};

use crate::lock;

type DesktopFile = HashMap<String, Vec<String>>;

fn desktop_files() -> &'static Mutex<HashMap<PathBuf, DesktopFile>> {
    static DESKTOP_FILES: OnceLock<Mutex<HashMap<PathBuf, DesktopFile>>> = OnceLock::new();
    DESKTOP_FILES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn desktop_files_look_out_keys() -> &'static HashSet<&'static str> {
    static DESKTOP_FILES_LOOK_OUT_KEYS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    DESKTOP_FILES_LOOK_OUT_KEYS
        .get_or_init(|| HashSet::from(["Name", "StartupWMClass", "Exec", "Icon"]))
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
        for mut xdg_dir in env::split_paths(&xdg_dirs) {
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

    find_desktop_file_by_filename(app_id, &files)
        .or_else(|| find_desktop_file_by_filedata(app_id, &files))
}

/// Finds the correct desktop file using a simple condition check
fn find_desktop_file_by_filename(app_id: &str, files: &[PathBuf]) -> Option<PathBuf> {
    let with_names = files
        .iter()
        .map(|f| {
            (
                f,
                f.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase(),
            )
        })
        .collect::<Vec<_>>();

    with_names
        .iter()
        // first pass - check for exact match
        .find(|(_, name)| name.eq_ignore_ascii_case(app_id))
        // second pass - check for substring
        .or_else(|| {
            with_names.iter().find(|(_, name)| {
                // this will attempt to find flatpak apps that are in the format
                // `com.company.app` or `com.app.something`
                app_id
                    .split(&[' ', ':', '@', '.', '_'][..])
                    .any(|part| name.eq_ignore_ascii_case(part))
            })
        })
        .map(|(file, _)| file.into())
}

/// Finds the correct desktop file using the keys in `DESKTOP_FILES_LOOK_OUT_KEYS`
fn find_desktop_file_by_filedata(app_id: &str, files: &[PathBuf]) -> Option<PathBuf> {
    let app_id = &app_id.to_lowercase();
    let mut desktop_files_cache = lock!(desktop_files());

    let files = files
        .iter()
        .filter_map(|file| {
            let parsed_desktop_file = parse_desktop_file(file)?;

            desktop_files_cache.insert(file.clone(), parsed_desktop_file.clone());
            Some((file.clone(), parsed_desktop_file))
        })
        .collect::<Vec<_>>();

    let file = files
        .iter()
        // first pass - check name key for exact match
        .find(|(_, desktop_file)| {
            desktop_file
                .get("Name")
                .is_some_and(|names| names.iter().any(|name| name.eq_ignore_ascii_case(app_id)))
        })
        // second pass - check name key for substring
        .or_else(|| {
            files.iter().find(|(_, desktop_file)| {
                desktop_file.get("Name").is_some_and(|names| {
                    names
                        .iter()
                        .any(|name| name.to_lowercase().contains(app_id))
                })
            })
        })
        // third pass - check all keys for substring
        .or_else(|| {
            files.iter().find(|(_, desktop_file)| {
                desktop_file
                    .values()
                    .flatten()
                    .any(|value| value.to_lowercase().contains(app_id))
            })
        });

    file.map(|(path, _)| path).cloned()
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
            let (key, value) = line.split_once('=')?;

            let key = key.trim();
            let value = value.trim();

            if desktop_files_look_out_keys().contains(key) {
                Some((key, value))
            } else {
                None
            }
        })
        .for_each(|(key, value)| {
            desktop_file
                .entry(key.to_string())
                .or_default()
                .push(value.to_string());
        });

    Some(desktop_file)
}

/// Attempts to get the icon name from the app's `.desktop` file.
pub fn get_desktop_icon_name(app_id: &str) -> Option<String> {
    let path = find_desktop_file(app_id)?;

    let mut desktop_files_cache = lock!(desktop_files());

    let desktop_file = match desktop_files_cache.get(&path) {
        Some(desktop_file) => desktop_file,
        _ => desktop_files_cache
            .entry(path.clone())
            .or_insert_with(|| parse_desktop_file(&path).expect("desktop_file")),
    };

    let mut icons = desktop_file.get("Icon").into_iter().flatten();

    icons.next().map(std::string::ToString::to_string)
}
