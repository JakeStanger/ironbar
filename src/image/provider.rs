use crate::channels::{AsyncSenderExt, MpscReceiverExt};
use crate::desktop_file::get_desktop_icon_name;
#[cfg(feature = "http")]
use crate::spawn;
use cfg_if::cfg_if;
use color_eyre::{Help, Report, Result};
use gtk::cairo::Surface;
use gtk::gdk::ffi::gdk_cairo_surface_create_from_pixbuf;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme};
use std::path::{Path, PathBuf};
#[cfg(feature = "http")]
use tokio::sync::mpsc;
use tracing::{debug, warn};

cfg_if!(
    if #[cfg(feature = "http")] {
        use gtk::gio::{Cancellable, MemoryInputStream};
        use tracing::error;
    }
);

#[derive(Debug)]
enum ImageLocation<'a> {
    Icon {
        name: String,
        theme: &'a IconTheme,
    },
    Local(PathBuf),
    Steam(String),
    #[cfg(feature = "http")]
    Remote(reqwest::Url),
}

pub struct ImageProvider<'a> {
    location: ImageLocation<'a>,
    size: i32,
}

impl<'a> ImageProvider<'a> {
    /// Attempts to parse the image input to find its location.
    /// Errors if no valid location type can be found.
    ///
    /// Note this checks that icons exist in theme, or files exist on disk
    /// but no other check is performed.
    pub fn parse(input: &str, theme: &'a IconTheme, use_fallback: bool, size: i32) -> Option<Self> {
        let location = Self::get_location(input, theme, size, use_fallback, 0)?;
        debug!("Resolved {input} --> {location:?} (size: {size})");

        Some(Self { location, size })
    }

    /// Returns true if the input starts with a prefix
    /// that is supported by the parser
    /// (ie the parser would not fallback to checking the input).
    pub fn is_definitely_image_input(input: &str) -> bool {
        input.starts_with("icon:")
            || input.starts_with("file://")
            || input.starts_with("http://")
            || input.starts_with("https://")
            || input.starts_with('/')
    }

    fn get_location(
        input: &str,
        theme: &'a IconTheme,
        size: i32,
        use_fallback: bool,
        recurse_depth: usize,
    ) -> Option<ImageLocation<'a>> {
        macro_rules! fallback {
            () => {
                if use_fallback {
                    Some(Self::get_fallback_icon(theme))
                } else {
                    None
                }
            };
        }

        const MAX_RECURSE_DEPTH: usize = 2;

        let should_parse_desktop_file = !Self::is_definitely_image_input(input);

        let (input_type, input_name) = input
            .split_once(':')
            .map_or((None, input), |(t, n)| (Some(t), n));

        match input_type {
            Some(input_type) if input_type == "icon" => Some(ImageLocation::Icon {
                name: input_name.to_string(),
                theme,
            }),
            Some(input_type) if input_type == "file" => Some(ImageLocation::Local(PathBuf::from(
                input_name[2..].to_string(),
            ))),
            #[cfg(feature = "http")]
            Some(input_type) if input_type == "http" || input_type == "https" => {
                input.parse().ok().map(ImageLocation::Remote)
            }
            None if input.starts_with("steam_app_") => Some(ImageLocation::Steam(
                input_name.chars().skip("steam_app_".len()).collect(),
            )),
            None if theme
                .lookup_icon(input, size, IconLookupFlags::empty())
                .is_some() =>
            {
                Some(ImageLocation::Icon {
                    name: input_name.to_string(),
                    theme,
                })
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
            None if recurse_depth == MAX_RECURSE_DEPTH => fallback!(),
            None if should_parse_desktop_file => {
                if let Some(location) = get_desktop_icon_name(input_name).map(|input| {
                    Self::get_location(&input, theme, size, use_fallback, recurse_depth + 1)
                }) {
                    location
                } else {
                    warn!("Failed to find image: {input}");
                    fallback!()
                }
            }
            None => {
                warn!("Failed to find image: {input}");
                fallback!()
            }
        }
    }

    /// Attempts to fetch the image from the location
    /// and load it into the provided `GTK::Image` widget.
    pub fn load_into_image(&self, image: &gtk::Image) -> Result<()> {
        // handle remote locations async to avoid blocking UI thread while downloading
        #[cfg(feature = "http")]
        if let ImageLocation::Remote(url) = &self.location {
            let url = url.clone();
            let (tx, rx) = mpsc::channel(64);

            spawn(async move {
                let bytes = Self::get_bytes_from_http(url).await;
                if let Ok(bytes) = bytes {
                    tx.send_expect(bytes).await;
                }
            });

            {
                let size = self.size;
                let image = image.clone();
                rx.recv_glib(move |bytes| {
                    let stream = MemoryInputStream::from_bytes(&bytes);

                    let scale = image.scale_factor();
                    let scaled_size = size * scale;

                    let pixbuf = Pixbuf::from_stream_at_scale(
                        &stream,
                        scaled_size,
                        scaled_size,
                        true,
                        Some(&Cancellable::new()),
                    );

                    // Different error types makes this a bit awkward
                    match pixbuf.map(|pixbuf| Self::create_and_load_surface(&pixbuf, &image)) {
                        Ok(Err(err)) => error!("{err:?}"),
                        Err(err) => error!("{err:?}"),
                        _ => {}
                    }
                });
            }
        } else {
            self.load_into_image_sync(image)?;
        };

        #[cfg(not(feature = "http"))]
        self.load_into_image_sync(image)?;

        Ok(())
    }

    /// Attempts to synchronously fetch an image from location
    /// and load into into the image.
    fn load_into_image_sync(&self, image: &gtk::Image) -> Result<()> {
        let scale = image.scale_factor();

        let pixbuf = match &self.location {
            ImageLocation::Icon { name, theme } => self.get_from_icon(name, theme, scale),
            ImageLocation::Local(path) => self.get_from_file(path, scale),
            ImageLocation::Steam(steam_id) => self.get_from_steam_id(steam_id, scale),
            #[cfg(feature = "http")]
            _ => unreachable!(), // handled above
        }?;

        Self::create_and_load_surface(&pixbuf, image)
    }

    /// Attempts to create a Cairo surface from the provided `Pixbuf`,
    /// using the provided scaling factor.
    /// The surface is then loaded into the provided image.
    ///
    /// This is necessary for HiDPI since `Pixbuf`s are always treated as scale factor 1.
    pub fn create_and_load_surface(pixbuf: &Pixbuf, image: &gtk::Image) -> Result<()> {
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

    /// Attempts to get a `Pixbuf` from the GTK icon theme.
    fn get_from_icon(&self, name: &str, theme: &IconTheme, scale: i32) -> Result<Pixbuf> {
        let pixbuf =
            match theme.lookup_icon_for_scale(name, self.size, scale, IconLookupFlags::empty()) {
                Some(_) => theme.load_icon(name, self.size * scale, IconLookupFlags::FORCE_SIZE),
                None => Ok(None),
            }?;

        pixbuf.map_or_else(
            || Err(Report::msg("Icon theme does not contain icon '{name}'")),
            Ok,
        )
    }

    /// Attempts to get a `Pixbuf` from a local file.
    fn get_from_file(&self, path: &Path, scale: i32) -> Result<Pixbuf> {
        let scaled_size = self.size * scale;
        let pixbuf = Pixbuf::from_file_at_scale(path, scaled_size, scaled_size, true)?;
        Ok(pixbuf)
    }

    /// Attempts to get a `Pixbuf` from a local file,
    /// using the Steam game ID to look it up.
    fn get_from_steam_id(&self, steam_id: &str, scale: i32) -> Result<Pixbuf> {
        // TODO: Can we load this from icon theme with app id `steam_icon_{}`?
        let path = dirs::data_dir().map_or_else(
            || Err(Report::msg("Missing XDG data dir")),
            |dir| {
                Ok(dir.join(format!(
                    "icons/hicolor/32x32/apps/steam_icon_{steam_id}.png"
                )))
            },
        )?;

        self.get_from_file(&path, scale)
    }

    /// Attempts to get `Bytes` from an HTTP resource asynchronously.
    #[cfg(feature = "http")]
    async fn get_bytes_from_http(url: reqwest::Url) -> Result<glib::Bytes> {
        let res = reqwest::get(url).await?;

        let status = res.status();
        if status.is_success() {
            let bytes = res.bytes().await?;
            Ok(glib::Bytes::from_owned(bytes))
        } else {
            Err(Report::msg(format!(
                "Received non-success HTTP code ({status})"
            )))
        }
    }

    fn get_fallback_icon(theme: &'a IconTheme) -> ImageLocation<'a> {
        ImageLocation::Icon {
            name: "dialog-question-symbolic".to_string(),
            theme,
        }
    }
}
