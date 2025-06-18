use crate::desktop_file::DesktopFiles;
use crate::{arc_mut, lock};
use color_eyre::{Help, Report, Result};
use gtk::cairo::Surface;
use gtk::gdk::ffi::gdk_cairo_surface_create_from_pixbuf;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{Cancellable, MemoryInputStream};
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme, Image};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, trace, warn};

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

    /// Attempts to resolve the provided input into a `Pixbuf`,
    /// and load that `Pixbuf` into the provided `Image` widget.
    ///
    /// If `use_fallback` is `true`, a fallback icon will be used
    /// where an image cannot be found.
    ///
    /// Returns `true` if the image was successfully loaded,
    /// or `false` if the image could not be found.
    /// May also return an error if the resolution or loading process failed.
    pub async fn load_into_image(
        &self,
        input: &str,
        size: i32,
        use_fallback: bool,
        image: &Image,
    ) -> Result<bool> {
        let image_ref = self.get_ref(input, size).await?;
        debug!("image ref for {input}: {:?}", image_ref);

        let pixbuf = if let Some(pixbuf) = lock!(self.cache).pixbuf_cache.get(&image_ref) {
            pixbuf.clone()
        } else {
            let pixbuf = Self::get_pixbuf(&image_ref, image.scale_factor(), use_fallback).await?;

            lock!(self.cache)
                .pixbuf_cache
                .insert(image_ref, pixbuf.clone());

            pixbuf
        };

        if let Some(ref pixbuf) = pixbuf {
            create_and_load_surface(pixbuf, image)?;
        }

        Ok(pixbuf.is_some())
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
            None if self
                .icon_theme()
                .lookup_icon(input_name, size, IconLookupFlags::empty())
                .is_some() =>
            {
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
    async fn get_pixbuf(
        image_ref: &ImageRef,
        scale: i32,
        use_fallback: bool,
    ) -> Result<Option<Pixbuf>> {
        const FALLBACK_ICON_NAME: &str = "dialog-question-symbolic";

        let buf = match &image_ref.location {
            Some(ImageLocation::Icon(name)) => image_ref.theme.load_icon_for_scale(
                name,
                image_ref.size,
                scale,
                IconLookupFlags::FORCE_SIZE,
            ),
            Some(ImageLocation::Local(path)) => {
                let scaled_size = image_ref.size * scale;
                Pixbuf::from_file_at_scale(path, scaled_size, scaled_size, true).map(Some)
            }
            Some(ImageLocation::Steam(app_id)) => {
                let path = dirs::data_dir().map_or_else(
                    || Err(Report::msg("Missing XDG data dir")),
                    |dir| Ok(dir.join(format!("icons/hicolor/32x32/apps/steam_icon_{app_id}.png"))),
                )?;

                let scaled_size = image_ref.size * scale;
                Pixbuf::from_file_at_scale(path, scaled_size, scaled_size, true).map(Some)
            }
            #[cfg(feature = "http")]
            Some(ImageLocation::Remote(uri)) => {
                let res = reqwest::get(uri.clone()).await?;

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

                Pixbuf::from_stream_at_scale(
                    &stream,
                    scaled_size,
                    scaled_size,
                    true,
                    Some(&Cancellable::new()),
                )
                .map(Some)
            }
            None if use_fallback => image_ref.theme.load_icon_for_scale(
                FALLBACK_ICON_NAME,
                image_ref.size,
                scale,
                IconLookupFlags::empty(),
            ),
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
            .expect("theme should be set at startup")
    }

    /// Sets the custom icon theme name.
    /// If no name is provided, the system default is used.
    pub fn set_icon_theme(&self, theme: Option<&str>) {
        trace!("Setting icon theme to {:?}", theme);

        *self.icon_theme.borrow_mut() = if theme.is_some() {
            let icon_theme = IconTheme::new();
            icon_theme.set_custom_theme(theme);
            Some(icon_theme)
        } else {
            IconTheme::default()
        };
    }
}

/// Attempts to create a Cairo `Surface` from the provided `Pixbuf`,
/// using the provided scaling factor.
/// The surface is then loaded into the provided image.
///
/// This is necessary for HiDPI since `Pixbuf`s are always treated as scale factor 1.
pub fn create_and_load_surface(pixbuf: &Pixbuf, image: &Image) -> Result<()> {
    let surface = unsafe {
        let ptr = gdk_cairo_surface_create_from_pixbuf(
            pixbuf.as_ptr(),
            image.scale_factor(),
            std::ptr::null_mut(),
        );

        Surface::from_raw_full(ptr)
    }?;

    image.set_from_surface(Some(&surface));

    Ok(())
}
