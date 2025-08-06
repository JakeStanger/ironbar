use crate::desktop_file::DesktopFiles;
use crate::{arc_mut, lock};
use cfg_if::cfg_if;
use color_eyre::{Help, Report, Result};
use gtk::cairo::Surface;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme, Image};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
#[cfg(feature = "http")]
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

cfg_if!(
    if #[cfg(feature = "http")] {
        use gtk::gio::{Cancellable, MemoryInputStream};
        use tracing::error;
    }
);

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
struct ImageRef {
    size: i32,
    location: Option<ImageLocation>,
    theme: IconTheme,
}

impl ImageRef {
    fn new(size: i32, location: Option<ImageLocation>, theme: IconTheme) -> Self {
        Self {
            size,
            location,
            theme,
        }
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
enum ImageLocation {
    Icon(String),
    Local(PathBuf),
    Steam(String),
    #[cfg(feature = "http")]
    Remote(reqwest::Url),
}

#[derive(Debug)]
struct Cache {
    location_cache: HashMap<(Box<str>, i32), ImageRef>,
    pixbuf_cache: HashMap<ImageRef, Option<Pixbuf>>,
}

impl Cache {
    fn new() -> Self {
        Self {
            location_cache: HashMap::new(),
            pixbuf_cache: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Provider {
    desktop_files: DesktopFiles,
    icon_theme: RefCell<Option<IconTheme>>,
    overrides: HashMap<String, String>,
    cache: Arc<Mutex<Cache>>,
}

impl Provider {
    pub fn new(desktop_files: DesktopFiles, overrides: &mut HashMap<String, String>) -> Self {
        let mut overrides_map = HashMap::with_capacity(overrides.len());
        overrides_map.extend(overrides.drain());

        Self {
            desktop_files,
            icon_theme: RefCell::new(None),
            overrides: overrides_map,
            cache: arc_mut!(Cache::new()),
        }
    }

    /// Like [`Provider::load_into_image`], but does not return an error if the image could not be found.
    ///
    /// If an image is not resolved, a warning is logged. Errors are also logged.
    pub async fn load_into_image_silent(
        &self,
        input: &str,
        size: i32,
        use_fallback: bool,
        image: &Image,
    ) {
        match self.load_into_image(input, size, use_fallback, image).await {
            Ok(true) => {}
            Ok(false) => warn!("failed to resolve image: {input}"),
            Err(e) => warn!("failed to load image: {input}: {e:?}"),
        }
    }

    /// Returns the `ImageRef` for the provided input.
    ///
    /// This contains the location of the image if it can be resolved.
    /// The ref will be loaded from cache if present.
    async fn get_ref(&self, input: &str, use_fallback: bool, size: i32) -> Result<ImageRef> {
        let key = (input.into(), size);

        if let Some(location) = lock!(self.cache).location_cache.get(&key) {
            Ok(location.clone())
        } else {
            let location = self.resolve_location(input, size, use_fallback, 0).await?;
            let image_ref = ImageRef::new(size, location, self.icon_theme());

            lock!(self.cache)
                .location_cache
                .insert(key, image_ref.clone());
            Ok(image_ref)
        }
    }

    /// Returns true if the input starts with a prefix
    /// that is supported by the parser
    /// (i.e. the parser would not fall back to checking the input).
    pub fn is_explicit_input(input: &str) -> bool {
        input.starts_with("icon:")
            || input.starts_with("file://")
            || input.starts_with("http://")
            || input.starts_with("https://")
            || input.starts_with('/')
    }

    /// Attempts to resolve the provided input into an `ImageLocation`.
    ///
    /// This will resolve all of:
    /// - The current icon theme
    /// - The file on disk
    /// - Steam icons
    /// - Desktop files (`Icon` keys)
    /// - HTTP(S) URLs
    async fn resolve_location(
        &self,
        input: &str,
        size: i32,
        use_fallback: bool,
        recurse_depth: u8,
    ) -> Result<Option<ImageLocation>> {
        macro_rules! fallback {
            () => {
                if use_fallback {
                    Some(Self::get_fallback_icon())
                } else {
                    None
                }
            };
        }

        const MAX_RECURSE_DEPTH: u8 = 2;

        let input = self.overrides.get(input).map_or(input, String::as_str);

        let should_parse_desktop_file = !Self::is_explicit_input(input);

        let (input_type, input_name) = input
            .split_once(':')
            .map_or((None, input), |(t, n)| (Some(t), n));

        let location = match input_type {
            Some(_t @ "icon") => Some(ImageLocation::Icon(input_name.to_string())),
            Some(_t @ "file") => Some(ImageLocation::Local(PathBuf::from(
                input_name[2..].to_string(),
            ))),
            #[cfg(feature = "http")]
            Some(_t @ ("http" | "https")) => input_name.parse().ok().map(ImageLocation::Remote),
            None if input_name.starts_with("steam_app_") => Some(ImageLocation::Steam(
                input_name.chars().skip("steam_app_".len()).collect(),
            )),
            None if self
                .icon_theme
                .borrow()
                .as_ref()
                .map(|t| t.has_icon(input))
                .unwrap_or(false) =>
            {
                Some(ImageLocation::Icon(input_name.to_string()))
            }
            Some(input_type) => {
                warn!(
                    "{:?}",
                    Report::msg(format!("Unsupported image type: {input_type}"))
                        .note("You may need to recompile with support if available")
                );
                fallback!()
            }
            None if PathBuf::from(input_name).is_file() => {
                Some(ImageLocation::Local(PathBuf::from(input_name)))
            }
            None if recurse_depth == MAX_RECURSE_DEPTH => None,
            None if should_parse_desktop_file => {
                let location = self
                    .desktop_files
                    .find(input_name)
                    .await?
                    .and_then(|input| input.icon);

                if let Some(location) = location {
                    if location == input_name {
                        None
                    } else {
                        Box::pin(self.resolve_location(
                            &location,
                            size,
                            use_fallback,
                            recurse_depth + 1,
                        ))
                        .await?
                    }
                } else {
                    warn!("Failed to find image: {input}");
                    fallback!()
                }
            }
            None => {
                warn!("Failed to find image: {input}");
                fallback!()
            }
        };

        Ok(location)
    }

    /// Attempts to fetch the image from the location
    /// and load it into the provided `GTK::Image` widget.
    pub async fn load_into_image(
        &self,
        input: &str,
        size: i32,
        use_fallback: bool,
        image: &Image,
    ) -> Result<bool> {
        let image_ref = self.get_ref(input, use_fallback, size).await?;
        let scale = image.scale_factor();
        // handle remote locations async to avoid blocking UI thread while downloading
        #[cfg(feature = "http")]
        if let Some(ImageLocation::Remote(url)) = &image_ref.location {
            let res = reqwest::get(url.clone()).await?;

            let status = res.status();
            let bytes = if status.is_success() {
                let bytes = res.bytes().await?;
                Ok(glib::Bytes::from_owned(bytes))
            } else {
                Err(Report::msg(format!(
                    "Received non-success HTTP code ({status})"
                )))
            }?;

            let stream = MemoryInputStream::from_bytes(&bytes);
            let scaled_size = image_ref.size * scale;

            let pixbuf = Pixbuf::from_stream_at_scale(
                &stream,
                scaled_size,
                scaled_size,
                true,
                Some(&Cancellable::new()),
            )
            .map(Some)?;

            image.set_from_pixbuf(pixbuf.as_ref());
        } else {
            self.load_into_image_sync(&image_ref, image);
        };

        #[cfg(not(feature = "http"))]
        self.load_into_image_sync(&image_ref, image);

        Ok(true)
    }

    /// Attempts to synchronously fetch an image from location
    /// and load into into the image.
    fn load_into_image_sync(&self, image_ref: &ImageRef, image: &Image) {
        let scale = image.scale_factor();

        if let Some(location) = &image_ref.location {
            match location {
                ImageLocation::Icon(name) => image.set_icon_name(Some(name)),
                ImageLocation::Local(path) => image.set_from_file(Some(path)),
                ImageLocation::Steam(steam_id) => {
                    image.set_from_file(Self::steam_id_to_path(steam_id).ok())
                }
                #[cfg(feature = "http")]
                _ => unreachable!(), // handled above
            };
        }
    }

    fn steam_id_to_path(steam_id: &str) -> Result<PathBuf> {
        dirs::data_dir().map_or_else(
            || Err(Report::msg("Missing XDG data dir")),
            |dir| {
                Ok(dir.join(format!(
                    "icons/hicolor/32x32/apps/steam_icon_{steam_id}.png"
                )))
            },
        )
    }

    pub fn icon_theme(&self) -> IconTheme {
        self.icon_theme
            .borrow()
            .clone()
            .expect("theme should be set at startup")
    }

    /// Sets the custom icon theme name.
    /// If no name is provided, the system default is used.
    pub fn set_icon_theme(&self, theme: Option<&str>) {
        trace!("Setting icon theme to {:?}", theme);

        *self.icon_theme.borrow_mut() = if theme.is_some() {
            let icon_theme = IconTheme::new();
            icon_theme.set_theme_name(theme);
            Some(icon_theme)
        } else {
            Some(IconTheme::default())
        };
    }

    fn get_fallback_icon() -> ImageLocation {
        ImageLocation::Icon("dialog-question-symbolic".to_string())
    }
}
