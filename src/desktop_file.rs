use crate::spawn;
use color_eyre::{Help, Report, Result};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, error};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone)]
enum DesktopFileRef {
    Unloaded(PathBuf),
    Loaded(DesktopFile),
}

impl DesktopFileRef {
    async fn get(&mut self) -> Result<DesktopFile> {
        match self {
            DesktopFileRef::Unloaded(path) => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                let path = path.clone();

                spawn(async move { tx.send(Self::load(&path).await) });

                let file = rx.await??;
                *self = DesktopFileRef::Loaded(file.clone());

                Ok(file)
            }
            DesktopFileRef::Loaded(file) => Ok(file.clone()),
        }
    }

    async fn load(file_path: &Path) -> Result<DesktopFile> {
        debug!("loading applications file: {}", file_path.display());

        let file = tokio::fs::File::open(file_path).await?;

        let mut desktop_file = DesktopFile::new(
            file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );

        let mut lines = BufReader::new(file).lines();

        let mut has_name = false;
        let mut has_type = false;
        let mut has_wm_class = false;
        let mut has_exec = false;
        let mut has_icon = false;
        let mut has_categories = false;
        let mut has_no_display = false;

        while let Ok(Some(line)) = lines.next_line().await {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            match key {
                "Name" if !has_name => {
                    desktop_file.name = Some(value.to_string());
                    has_name = true;
                }
                "Type" if !has_type => {
                    desktop_file.app_type = Some(value.to_string());
                    has_type = true;
                }
                "StartupWMClass" if !has_wm_class => {
                    desktop_file.startup_wm_class = Some(value.to_string());
                    has_wm_class = true;
                }
                "Exec" if !has_exec => {
                    desktop_file.exec = Some(value.to_string());
                    has_exec = true;
                }
                "Icon" if !has_icon => {
                    desktop_file.icon = Some(value.to_string());
                    has_icon = true;
                }
                "Categories" if !has_categories => {
                    desktop_file.categories = value.split(';').map(|s| s.to_string()).collect();
                    has_categories = true;
                }
                "NoDisplay" if !has_no_display => {
                    desktop_file.no_display = Some(value.parse()?);
                    has_no_display = true;
                }
                _ => {}
            }

            // parsing complete - don't bother with the rest of the lines
            if has_name
                && has_type
                && has_wm_class
                && has_exec
                && has_icon
                && has_categories
                && has_no_display
            {
                break;
            }
        }

        Ok(desktop_file)
    }
}

#[derive(Debug, Clone)]
pub struct DesktopFile {
    pub file_name: String,
    pub name: Option<String>,
    pub app_type: Option<String>,
    pub startup_wm_class: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub no_display: Option<bool>,
}

impl DesktopFile {
    fn new(file_name: String) -> Self {
        Self {
            file_name,
            name: None,
            app_type: None,
            startup_wm_class: None,
            exec: None,
            icon: None,
            categories: vec![],
            no_display: None,
        }
    }
}

type FileMap = HashMap<Box<str>, DesktopFileRef>;

/// Desktop file cache and resolver.
///
/// Files are lazy-loaded as required on resolution.
#[derive(Debug, Clone)]
pub struct DesktopFiles {
    files: Arc<Mutex<FileMap>>,
}

impl Default for DesktopFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopFiles {
    /// Creates a new instance,
    /// scanning disk to generate a list of (unloaded) file refs in the process.
    pub fn new() -> Self {
        let desktop_files: FileMap = dirs()
            .iter()
            .flat_map(|path| files(path))
            .map(|file| {
                (
                    file.file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string()
                        .into(),
                    DesktopFileRef::Unloaded(file),
                )
            })
            .collect();

        debug!("resolved {} files", desktop_files.len());

        Self {
            files: Arc::new(Mutex::new(desktop_files)),
        }
    }

    pub async fn get_all(&self) -> Result<Vec<DesktopFile>> {
        let mut files = self.files.lock().await;

        let mut res = Vec::with_capacity(files.len());
        for file in files.values_mut() {
            let file = file.get().await?;
            res.push(file);
        }

        Ok(res)
    }

    /// Attempts to locate a applications file by file name or contents.
    ///
    /// Input should typically be the app id, app name or icon.
    pub async fn find(&self, input: &str) -> Result<Option<DesktopFile>> {
        let mut res = self.find_by_file_name(input).await?;
        if res.is_none() {
            res = self.find_by_file_contents(input).await?;
        }

        debug!("found match for app_id {input}: {}", res.is_some());

        Ok(res)
    }

    /// Checks file names for an exact or partial match of the provided input.
    async fn find_by_file_name(&self, input: &str) -> Result<Option<DesktopFile>> {
        let mut files = self.files.lock().await;

        let mut file_ref = files
            .iter_mut()
            .find(|&(name, _)| name.eq_ignore_ascii_case(input));

        if file_ref.is_none() {
            file_ref = files.iter_mut().find(
                |&(name, _)| // this will attempt to find flatpak apps that are in the format
                    // `com.company.app` or `com.app.something`
                    input
                        .split(&[' ', ':', '@', '.', '_'][..])
                        .any(|part| name.eq_ignore_ascii_case(part)),
            );
        }

        let file_ref = file_ref.map(|(_, file)| file);

        if let Some(file_ref) = file_ref {
            let file = file_ref.get().await?;
            Ok(Some(file))
        } else {
            Ok(None)
        }
    }

    /// Checks file contents for an exact or partial match of the provided input.
    async fn find_by_file_contents(&self, app_id: &str) -> Result<Option<DesktopFile>> {
        let mut files = self.files.lock().await;

        // first pass - check name for exact match
        for (_, file_ref) in files.iter_mut() {
            let file = file_ref.get().await?;
            if let Some(name) = &file.name {
                if name.eq_ignore_ascii_case(app_id) {
                    return Ok(Some(file));
                }
            }
        }

        // second pass - check name for partial match
        for (_, file_ref) in files.iter_mut() {
            let file = file_ref.get().await?;
            if let Some(name) = &file.name {
                if name.to_lowercase().contains(app_id) {
                    return Ok(Some(file));
                }
            }
        }

        // third pass - check remaining fields for partial match
        for (_, file_ref) in files.iter_mut() {
            let file = file_ref.get().await?;

            if let Some(name) = &file.exec {
                if name.to_lowercase().contains(app_id) {
                    return Ok(Some(file));
                }
            }

            if let Some(name) = &file.startup_wm_class {
                if name.to_lowercase().contains(app_id) {
                    return Ok(Some(file));
                }
            }

            if let Some(name) = &file.icon {
                if name.to_lowercase().contains(app_id) {
                    return Ok(Some(file));
                }
            }
        }

        Ok(None)
    }
}

/// Gets a list of paths to all directories
/// containing `.applications` files.
fn dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"), // system installed apps
        PathBuf::from("/var/lib/flatpak/exports/share/applications"), // flatpak apps
    ];

    let xdg_dirs = env::var("XDG_DATA_DIRS");
    if let Ok(xdg_dirs) = xdg_dirs {
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

    dirs.into_iter().filter(|dir| dir.exists()).rev().collect()
}

/// Gets a list of all `.applications` files in the provided directory.
///
/// The directory is recursed to a maximum depth of 5.
fn files(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .max_depth(5)
        .into_iter()
        .filter_map(Result::ok)
        .map(DirEntry::into_path)
        .filter(|file| file.is_file() && file.extension().unwrap_or_default() == "desktop")
        .collect()
}

/// Starts a `.desktop` file with the provided formatted command.
pub async fn open_program(file_name: &str, launch_command: &str) {
    let expanded = launch_command.replace("{app_name}", file_name);
    let launch_command_parts: Vec<&str> = expanded.split_whitespace().collect();

    debug!("running {launch_command_parts:?}");
    if let Err(err) = Command::new(launch_command_parts[0])
        .args(&launch_command_parts[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
    {
        error!(
            "{:?}",
            Report::new(err)
                .wrap_err("Failed to run launch command.")
                .suggestion("Perhaps the desktop file is invalid or orphaned?")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        unsafe {
            let pwd = env::current_dir().unwrap();
            env::set_var("XDG_DATA_DIRS", format!("{}/test-configs", pwd.display()));
        }
    }

    #[tokio::test]
    async fn find_by_filename() {
        setup();

        let desktop_files = DesktopFiles::new();
        let file = desktop_files.find_by_file_name("firefox").await.unwrap();

        assert!(file.is_some());
        assert_eq!(file.unwrap().file_name, "firefox.desktop");
    }

    #[tokio::test]
    async fn find_by_file_contents() {
        setup();

        let desktop_files = DesktopFiles::new();

        let file = desktop_files.find_by_file_contents("427520").await.unwrap();

        assert!(file.is_some());
        assert_eq!(file.unwrap().file_name, "Factorio.desktop");
    }

    #[tokio::test]
    async fn parser() {
        let mut file_ref =
            DesktopFileRef::Unloaded(PathBuf::from("test-configs/applications/firefox.desktop"));
        let file = file_ref.get().await.unwrap();

        assert_eq!(file.name, Some("Firefox".to_string()));
        assert_eq!(file.icon, Some("firefox".to_string()));
        assert_eq!(file.exec, Some("/usr/lib/firefox/firefox %u".to_string()));
        assert_eq!(file.startup_wm_class, Some("firefox".to_string()));
        assert_eq!(file.app_type, Some("Application".to_string()));
    }
}
