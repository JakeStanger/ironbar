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

        let entry = walker.find(|entry| {
            entry.as_ref().map_or(false, |entry| {
                let file_name = entry.file_name().to_string_lossy().to_lowercase();
                let test_name = format!("{}.desktop", app_id.to_lowercase());
                file_name == test_name
            })
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
pub fn get_desktop_icon_name(app_id: &str) -> Option<String> {
    find_desktop_file(app_id).and_then(|file| {
        let map = parse_desktop_file(file);
        map.map_or(None, |map| {
            map.get("Icon").map(std::string::ToString::to_string)
        })
    })
}
