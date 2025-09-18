use crate::image;
use crate::modules::tray::interface::TrayMenu;
use color_eyre::{Report, Result};
use glib::ffi::g_strfreev;
use glib::translate::ToGlibPtr;
use gtk::ffi::gtk_icon_theme_get_search_path;
use gtk::gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gtk::prelude::WidgetExt;
use gtk::{IconLookupFlags, IconTheme, Image, TextDirection};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use system_tray::item::IconPixmap;
use tracing::trace;

pub fn get_image(
    item: &TrayMenu,
    size: u32,
    prefer_icons: bool,
    icon_theme: &IconTheme,
) -> Result<Image> {
    if !prefer_icons && item.icon_pixmap.is_some() {
        get_image_from_pixmap(item.icon_pixmap.as_deref(), size)
    } else {
        get_image_from_icon_name(item, size, icon_theme)
            .or_else(|_| get_image_from_pixmap(item.icon_pixmap.as_deref(), size))
    }
}

/// Attempts to get a GTK `Image` component
/// for the status notifier item's icon.
fn get_image_from_icon_name(item: &TrayMenu, size: u32, icon_theme: &IconTheme) -> Result<Image> {
    if let Some(path) = item.icon_theme_path.as_ref()
        && !path.is_empty()
        && !get_icon_theme_search_paths(icon_theme).contains(path)
    {
        icon_theme.add_search_path(path);
    }

    let image = Image::new();

    let paintable = item.icon_name.as_ref().map(|icon_name| {
        icon_theme.lookup_icon(
            icon_name,
            &[],
            size as i32,
            image.scale_factor(),
            TextDirection::None,
            IconLookupFlags::empty(),
        )
    });

    if let Some(paintable) = paintable {
        image.set_paintable(Some(&paintable));
        Ok(image)
    } else {
        Err(Report::msg("could not find icon"))
    }
}

/// Attempts to get an image from the item pixmap.
///
/// The pixmap is supplied in ARGB32 format,
/// which has 8 bits per sample and a bit stride of `4*width`.
/// The Pixbuf expects RGBA32 format, so some channel shuffling
/// is required.
fn get_image_from_pixmap(item: Option<&[IconPixmap]>, size: u32) -> Result<Image> {
    const BITS_PER_SAMPLE: i32 = 8;

    let pixmap = item
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

/// Gets the GTK icon theme search paths by calling the FFI function.
/// Conveniently returns the result as a `HashSet`.
fn get_icon_theme_search_paths(icon_theme: &IconTheme) -> HashSet<String> {
    let mut gtk_paths: *mut *mut c_char = ptr::null_mut();
    let mut n_elements: c_int = 0;
    let mut paths = HashSet::new();
    unsafe {
        gtk_icon_theme_get_search_path(icon_theme.to_glib_none().0);
        // n_elements is never negative (that would be weird)
        for i in 0..n_elements as usize {
            let c_str = CStr::from_ptr(*gtk_paths.add(i));
            if let Ok(str) = c_str.to_str() {
                paths.insert(str.to_owned());
            }
        }

        g_strfreev(gtk_paths);
    }

    paths
}
