use crate::desktop_file::DesktopFiles;
use crate::{arc_mut, lock};
use color_eyre::{Help, Report, Result};
use glib::Bytes;
use gtk::cairo::Surface;
use gtk::gdk::{Paintable, Texture};
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{Cancellable, MemoryInputStream};
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconPaintable, IconTheme, Image, Picture, Snapshot, TextDirection};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, trace, warn};

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
    paintable_cache: HashMap<ImageRef, Option<Paintable>>,
}

impl Cache {
    fn new() -> Self {
        Self {
            location_cache: HashMap::new(),
            paintable_cache: HashMap::new(),
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

    /// Attempts to resolve the provided input into a `Pixbuf`,
    /// and load that `Pixbuf` into the provided `Picture` widget.
    ///
    /// If `use_fallback` is `true`, a fallback icon will be used
    /// where an image cannot be found.
    ///
    /// Returns `true` if the image was successfully loaded,
    /// or `false` if the image could not be found.
    /// May also return an error if the resolution or loading process failed.
    pub async fn load_into_picture(
        &self,
        input: &str,
        size: i32,
        use_fallback: bool,
        picture: &Picture,
    ) -> Result<bool> {
        let image_ref = self.get_ref(input, size).await?;
        debug!("image ref for {input}: {:?}", image_ref);

        let paintable = if let Some(pixbuf) = lock!(self.cache).paintable_cache.get(&image_ref) {
            pixbuf.clone()
        } else {
            let pixbuf =
                Self::get_paintable(&image_ref, picture.scale_factor(), use_fallback).await?;

            lock!(self.cache)
                .paintable_cache
                .insert(image_ref, pixbuf.clone());

            pixbuf
        };

        let has_match = paintable.is_some();
        picture.set_paintable(paintable.as_ref());

        Ok(has_match)
    }

    /// Like [`Provider::load_into_picture`], but does not return an error if the image could not be found.
    ///
    /// If an image is not resolved, a warning is logged. Errors are also logged.
    pub async fn load_into_picture_silent(
        &self,
        input: &str,
        size: i32,
        use_fallback: bool,
        picture: &Picture,
    ) {
        match self
            .load_into_picture(input, size, use_fallback, picture)
            .await
        {
            Ok(true) => {}
            Ok(false) => warn!("failed to resolve image: {input}"),
            Err(e) => warn!("failed to load image: {input}: {e:?}"),
        }
    }

    /// Returns the `ImageRef` for the provided input.
    ///
    /// This contains the location of the image if it can be resolved.
    /// The ref will be loaded from cache if present.
    async fn get_ref(&self, input: &str, size: i32) -> Result<ImageRef> {
        let key = (input.into(), size);

        if let Some(location) = lock!(self.cache).location_cache.get(&key) {
            Ok(location.clone())
        } else {
            let location = self.resolve_location(input, size, 0).await?;
            let image_ref = ImageRef::new(size, location, self.icon_theme());

            lock!(self.cache)
                .location_cache
                .insert(key, image_ref.clone());
            Ok(image_ref)
        }
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
        recurse_depth: u8,
    ) -> Result<Option<ImageLocation>> {
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
            None if self.icon_theme().has_icon(input_name) => {
                Some(ImageLocation::Icon(input_name.to_string()))
            }
            Some(input_type) => {
                warn!(
                    "{:?}",
                    Report::msg(format!("Unsupported image type: {input_type}"))
                        .note("You may need to recompile with support if available")
                );
                None
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
                        Box::pin(self.resolve_location(&location, size, recurse_depth + 1)).await?
                    }
                } else {
                    None
                }
            }
            None => None,
        };

        Ok(location)
    }

    /// Attempts to load the provided `ImageRef` into a `Pixbuf`.
    ///
    /// If `use_fallback` is `true`, a fallback icon will be used
    /// where an image cannot be found.
    async fn get_paintable(
        image_ref: &ImageRef,
        scale: i32,
        use_fallback: bool,
    ) -> Result<Option<Paintable>> {
        const FALLBACK_ICON_NAME: &str = "dialog-question-symbolic";

        let buf = match &image_ref.location {
            Some(ImageLocation::Icon(name)) => {
                let sizes = image_ref.theme.icon_sizes(name);
                let size = sizes
                    .into_iter()
                    .find(|&s| s >= image_ref.size)
                    .unwrap_or(image_ref.size);

                Ok(Some(
                    image_ref
                        .theme
                        .lookup_icon(
                            name,
                            &[], // setting fallback here causes issue loading some icons
                            image_ref.size,
                            scale,
                            TextDirection::None,
                            IconLookupFlags::empty(),
                        )
                        .upcast::<Paintable>(),
                ))
            }
            Some(ImageLocation::Local(path)) if path.extension().unwrap_or_default() == "svg" => {
                let scaled_size = image_ref.size * scale;

                let pixbuf = Pixbuf::from_file_at_scale(path, scaled_size, scaled_size, true)?;
                let bytes = pixbuf.pixel_bytes();

                let buffer = pixbuf.save_to_bufferv("png", &[])?;
                let bytes = Bytes::from_owned(buffer);

                let texture = Texture::from_bytes(&bytes)?;
                Ok(Some(texture.upcast::<Paintable>()))
            }
            Some(ImageLocation::Local(path)) => {
                Texture::from_filename(path).map(|t| Self::scale_texture(t, image_ref.size))
            }
            Some(ImageLocation::Steam(app_id)) => {
                const SIZES: [i32; 8] = [16, 24, 32, 48, 64, 96, 128, 256];
                let size = SIZES
                    .into_iter()
                    .find(|&s| s < image_ref.size)
                    .unwrap_or(image_ref.size);

                let path = dirs::data_dir().map_or_else(
                    || Err(Report::msg("Missing XDG data dir")),
                    |dir| {
                        Ok(dir.join(format!(
                            "icons/hicolor/{size}x{size}/apps/steam_icon_{app_id}.png"
                        )))
                    },
                )?;

                Texture::from_filename(path).map(|t| Self::scale_texture(t, image_ref.size))
            }
            #[cfg(feature = "http")]
            Some(ImageLocation::Remote(uri)) => {
                let res = reqwest::get(uri.clone()).await?;

                let status = res.status();
                let bytes = if status.is_success() {
                    let bytes = res.bytes().await?;
                    Ok(Bytes::from_owned(bytes))
                } else {
                    Err(Report::msg(format!(
                        "Received non-success HTTP code ({status})"
                    )))
                }?;

                Texture::from_bytes(&bytes).map(|t| Self::scale_texture(t, image_ref.size))
            }
            None if use_fallback => Ok(Some(
                image_ref
                    .theme
                    .lookup_icon(
                        FALLBACK_ICON_NAME,
                        &[],
                        image_ref.size,
                        scale,
                        TextDirection::None,
                        IconLookupFlags::empty(),
                    )
                    .upcast::<Paintable>(),
            )),
            None => Ok(None),
        }?;

        Ok(buf)
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

    pub fn icon_theme(&self) -> IconTheme {
        self.icon_theme
            .borrow()
            .clone()
            .expect("theme should be set on bar init")
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

    fn scale_texture(texture: Texture, size: i32) -> Option<Paintable> {
        let snapshot = Snapshot::new();
        texture.snapshot(&snapshot, size as f64, size as f64);
        snapshot.to_paintable(None)
    }
}
