use crate::desktop_file::get_desktop_icon_name;
use crate::send;
use color_eyre::{Report, Result};
use glib::Bytes;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{Cancellable, MemoryInputStream};
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme};
use reqwest::Url;
use std::path::{Path, PathBuf};
use tokio::spawn;
use tracing::error;

#[derive(Debug)]
enum ImageLocation<'a> {
    Icon { name: String, theme: &'a IconTheme },
    Local(PathBuf),
    Steam(String),
    Remote(Url),
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
    pub fn parse(input: &str, theme: &'a IconTheme, size: i32) -> Result<Self> {
        let location = Self::get_location(input, theme, size)?;
        Ok(Self { location, size })
    }

    /// Returns true if the input starts with a prefix
    /// that is supported by the parser
    /// (ie the parser would not fallback to checking the input).
    pub fn is_definitely_image_input(input: &str) -> bool {
        input.starts_with("icon:")
            || input.starts_with("file://")
            || input.starts_with("http://")
            || input.starts_with("https://")
    }

    fn get_location(input: &str, theme: &'a IconTheme, size: i32) -> Result<ImageLocation<'a>> {
        let (input_type, input_name) = input
            .split_once(':')
            .map_or((None, input), |(t, n)| (Some(t), n));

        match input_type {
            Some(input_type) if input_type == "icon" => Ok(ImageLocation::Icon {
                name: input_name.to_string(),
                theme,
            }),
            Some(input_type) if input_type == "file" => Ok(ImageLocation::Local(PathBuf::from(
                input_name[2..].to_string(),
            ))),
            Some(input_type) if input_type == "http" || input_type == "https" => {
                Ok(ImageLocation::Remote(input.parse()?))
            }
            None if input.starts_with("steam_app_") => Ok(ImageLocation::Steam(
                input_name.chars().skip("steam_app_".len()).collect(),
            )),
            None if theme
                .lookup_icon(input, size, IconLookupFlags::empty())
                .is_some() =>
            {
                Ok(ImageLocation::Icon {
                    name: input_name.to_string(),
                    theme,
                })
            }
            Some(input_type) => Err(Report::msg(format!("Unsupported image type: {input_type}"))),
            None if PathBuf::from(input_name).is_file() => {
                Ok(ImageLocation::Local(PathBuf::from(input_name)))
            }
            None => get_desktop_icon_name(input_name).map_or_else(
                || Err(Report::msg("Unknown image type")),
                |input| Self::get_location(&input, theme, size),
            ),
        }
    }

    /// Attempts to fetch the image from the location
    /// and load it into the provided `GTK::Image` widget.
    pub fn load_into_image(&self, image: gtk::Image) -> Result<()> {
        // handle remote locations async to avoid blocking UI thread while downloading
        if let ImageLocation::Remote(url) = &self.location {
            let url = url.clone();
            let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

            spawn(async move {
                let bytes = Self::get_bytes_from_http(url).await;
                if let Ok(bytes) = bytes {
                    send!(tx, bytes);
                }
            });

            {
                let size = self.size;
                rx.attach(None, move |bytes| {
                    let stream = MemoryInputStream::from_bytes(&bytes);
                    let pixbuf = Pixbuf::from_stream_at_scale(
                        &stream,
                        size,
                        size,
                        true,
                        Some(&Cancellable::new()),
                    );

                    match pixbuf {
                        Ok(pixbuf) => image.set_pixbuf(Some(&pixbuf)),
                        Err(err) => error!("{err:?}"),
                    }

                    Continue(false)
                });
            }
        } else {
            let pixbuf = match &self.location {
                ImageLocation::Icon { name, theme } => self.get_from_icon(name, theme),
                ImageLocation::Local(path) => self.get_from_file(path),
                ImageLocation::Steam(steam_id) => self.get_from_steam_id(steam_id),
                ImageLocation::Remote(_) => unreachable!(), // handled above
            }?;

            image.set_pixbuf(Some(&pixbuf));
        };

        Ok(())
    }

    /// Attempts to get a `Pixbuf` from the GTK icon theme.
    fn get_from_icon(&self, name: &str, theme: &IconTheme) -> Result<Pixbuf> {
        let pixbuf = match theme.lookup_icon(name, self.size, IconLookupFlags::empty()) {
            Some(_) => theme.load_icon(name, self.size, IconLookupFlags::FORCE_SIZE),
            None => Ok(None),
        }?;

        pixbuf.map_or_else(
            || Err(Report::msg("Icon theme does not contain icon '{name}'")),
            Ok,
        )
    }

    /// Attempts to get a `Pixbuf` from a local file.
    fn get_from_file(&self, path: &Path) -> Result<Pixbuf> {
        let pixbuf = Pixbuf::from_file_at_scale(path, self.size, self.size, true)?;
        Ok(pixbuf)
    }

    /// Attempts to get a `Pixbuf` from a local file,
    /// using the Steam game ID to look it up.
    fn get_from_steam_id(&self, steam_id: &str) -> Result<Pixbuf> {
        // TODO: Can we load this from icon theme with app id `steam_icon_{}`?
        let path = dirs::data_dir().map_or_else(
            || Err(Report::msg("Missing XDG data dir")),
            |dir| {
                Ok(dir.join(format!(
                    "icons/hicolor/32x32/apps/steam_icon_{steam_id}.png"
                )))
            },
        )?;

        self.get_from_file(&path)
    }

    /// Attempts to get `Bytes` from an HTTP resource asynchronously.
    async fn get_bytes_from_http(url: Url) -> Result<Bytes> {
        let bytes = reqwest::get(url).await?.bytes().await?;
        Ok(Bytes::from_owned(bytes))
    }
}
