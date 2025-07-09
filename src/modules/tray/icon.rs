use crate::image::ImageProvider;
use crate::modules::tray::interface::TrayMenu;
use color_eyre::{Report, Result};
use glib::ffi::g_strfreev;
use glib::translate::ToGlibPtr;
use gtk::ffi::gtk_icon_theme_get_search_path;
use gtk::gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gtk::prelude::WidgetExt;
use gtk::{IconLookupFlags, IconTheme, Image};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use system_tray::item::IconPixmap;
use tracing::trace;

pub fn get_image(
    item: &TrayMenu,
    icon_theme: &IconTheme,
    size: u32,
    prefer_icons: bool,
) -> Result<Image> {
    if !prefer_icons && item.icon_pixmap.is_some() {
        get_image_from_pixmap(item, size)
    } else {
        let icon = item.icon_name.as_ref().and_then(|icon_name| {
            ImageProvider::parse(&icon_name, &icon_theme, false, size as i32)
        });

        if let Some(provider) = icon {
            let image = Image::new();
            provider.load_into_image(&image);
            Ok(image)
        } else {
            get_image_from_pixmap(item, size)
        }
    }
}

/// Attempts to get an image from the item pixmap.
///
/// The pixmap is supplied in ARGB32 format,
/// which has 8 bits per sample and a bit stride of `4*width`.
/// The Pixbuf expects RGBA32 format, so some channel shuffling
/// is required.
fn get_image_from_pixmap(item: &TrayMenu, size: u32) -> Result<Image> {
    const BITS_PER_SAMPLE: i32 = 8;

    let pixmap = item
        .icon_pixmap
        .as_ref()
        .and_then(|pixmap| pixmap.first())
        .ok_or_else(|| Report::msg("Failed to get pixmap from tray icon"))?;

    if pixmap.width == 0 || pixmap.height == 0 {
        return Err(Report::msg("empty pixmap"));
    }

    let mut pixels = pixmap.pixels.clone();

    for i in (0..pixels.len()).step_by(4) {
        let alpha = pixels[i];
        pixels[i] = pixels[i + 1];
        pixels[i + 1] = pixels[i + 2];
        pixels[i + 2] = pixels[i + 3];
        pixels[i + 3] = alpha;
    }

    let row_stride = pixmap.width * 4;
    let bytes = glib::Bytes::from(&pixels);

    let pixbuf = Pixbuf::from_bytes(
        &bytes,
        Colorspace::Rgb,
        true,
        BITS_PER_SAMPLE,
        pixmap.width,
        pixmap.height,
        row_stride,
    );

    let pixbuf = pixbuf
        .scale_simple(size as i32, size as i32, InterpType::Bilinear)
        .unwrap_or(pixbuf);

    let image = Image::new();
    image.set_from_pixbuf(Some(&pixbuf));
    Ok(image)
}
