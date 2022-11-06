use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Gets directories that should contain `.desktop` files
/// and exist on the filesystem.
fn find_application_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    let user_dir = dirs::data_local_dir();

    if let Some(mut user_dir) = user_dir {
        user_dir.push("applications");
        dirs.push(user_dir);
    }

    dirs.into_iter().filter(|dir| dir.exists()).collect()
}

/// Attempts to locate a `.desktop` file for an app id
/// (or app class).
///
/// A simple case-insensitive check is performed on filename == `app_id`.
pub fn find_desktop_file(app_id: &str) -> Option<PathBuf> {
    let dirs = find_application_dirs();

    for dir in dirs {
        let mut walker = WalkDir::new(dir).max_depth(5).into_iter();

        let entry = walker.find(|entry| match entry {
            Ok(entry) => {
                let file_name = entry.file_name().to_string_lossy().to_lowercase();
                let test_name = format!("{}.desktop", app_id.to_lowercase());
                file_name == test_name
            }
            _ => false,
        });

        if let Some(Ok(entry)) = entry {
            let path = entry.path().to_owned();
            return Some(path);
        }
    }

    None
}

/// Parses a desktop file into a flat hashmap of keys/values.
fn parse_desktop_file(path: PathBuf) -> io::Result<HashMap<String, String>> {
    let file = File::open(path)?;
    let lines = io::BufReader::new(file).lines();

    let mut map = HashMap::new();

    for line in lines.flatten() {
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.to_string(), value.to_string());
        }
    }

    Ok(map)
}

/// Attempts to get the icon name from the app's `.desktop` file.
fn get_desktop_icon_name(app_id: &str) -> Option<String> {
    find_desktop_file(app_id).and_then(|file| {
        let map = parse_desktop_file(file);
        map.map_or(None, |map| {
            map.get("Icon").map(std::string::ToString::to_string)
        })
    })
}

enum IconLocation {
    Theme(String),
    File(PathBuf),
}

/// Attempts to get the location of an icon.
///
/// Handles icons that are part of a GTK theme, icons specified as path
/// and icons for steam games.
fn get_icon_location(theme: &IconTheme, app_id: &str, size: i32) -> Option<IconLocation> {
    let has_icon = theme
        .lookup_icon(app_id, size, IconLookupFlags::empty())
        .is_some();

    if has_icon {
        return Some(IconLocation::Theme(app_id.to_string()));
    }

    let is_steam_game = app_id.starts_with("steam_app_");
    if is_steam_game {
        let steam_id: String = app_id.chars().skip("steam_app_".len()).collect();

        return match dirs::data_dir() {
            Some(dir) => {
                let path = dir.join(format!(
                    "icons/hicolor/32x32/apps/steam_icon_{}.png",
                    steam_id
                ));

                return Some(IconLocation::File(path));
            }
            None => None,
        };
    }

    let icon_name = get_desktop_icon_name(app_id);
    if let Some(icon_name) = icon_name {
        let is_path = PathBuf::from(&icon_name).exists();

        return if is_path {
            Some(IconLocation::File(PathBuf::from(icon_name)))
        } else {
            return Some(IconLocation::Theme(icon_name));
        };
    }

    None
}

/// Gets the icon associated with an app.
pub fn get_icon(theme: &IconTheme, app_id: &str, size: i32) -> Option<Pixbuf> {
    let icon_location = get_icon_location(theme, app_id, size);

    match icon_location {
        Some(IconLocation::Theme(icon_name)) => {
            let icon = theme.load_icon(&icon_name, size, IconLookupFlags::FORCE_SIZE);
            icon.map_or(None, |icon| icon)
        }
        Some(IconLocation::File(path)) => Pixbuf::from_file_at_scale(path, size, size, true).ok(),
        None => None,
    }
}
