use crate::image::ImageProvider;
use crate::modules::tray::interface::TrayMenu;
use color_eyre::{Report, Result};
use glib::ffi::g_strfreev;
use glib::translate::ToGlibPtr;
use gtk::ffi::gtk_icon_theme_get_search_path;
use gtk::gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gtk::prelude::IconThemeExt;
use gtk::{IconLookupFlags, IconTheme, Image};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

/// Gets the GTK icon theme search paths by calling the FFI function.
/// Conveniently returns the result as a `HashSet`.
fn get_icon_theme_search_paths(icon_theme: &IconTheme) -> HashSet<String> {
    let mut gtk_paths: *mut *mut c_char = ptr::null_mut();
    let mut n_elements: c_int = 0;
    let mut paths = HashSet::new();
    unsafe {
        gtk_icon_theme_get_search_path(
            icon_theme.to_glib_none().0,
            &mut gtk_paths,
            &mut n_elements,
        );
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

pub fn get_image(
    item: &TrayMenu,
    icon_theme: &IconTheme,
    size: u32,
    prefer_icons: bool,
) -> Result<Image> {
    if !prefer_icons && item.icon_pixmap.is_some() {
        get_image_from_pixmap(item, size)
    } else {
        get_image_from_icon_name(item, icon_theme, size)
            .or_else(|_| get_image_from_pixmap(item, size))
    }
}

/// Attempts to get a GTK `Image` component
/// for the status notifier item's icon.
fn get_image_from_icon_name(item: &TrayMenu, icon_theme: &IconTheme, size: u32) -> Result<Image> {
    if let Some(path) = item.icon_theme_path.as_ref() {
        if !path.is_empty() && !get_icon_theme_search_paths(icon_theme).contains(path) {
            icon_theme.append_search_path(path);
        }
    }

    let icon_info = item.icon_name.as_ref().and_then(|icon_name| {
        icon_theme.lookup_icon(icon_name, size as i32, IconLookupFlags::empty())
    });

    if let Some(icon_info) = icon_info {
        let pixbuf = icon_info.load_icon()?;
        let image = Image::new();
        ImageProvider::create_and_load_surface(&pixbuf, &image)?;
        Ok(image)
    } else {
        Err(Report::msg("could not find icon"))
    }
}

/// Attempts to get an image from the item pixmap.
///
/// The pixmap is supplied in ARGB32 format,
/// which has 8 bits per sample and a bit stride of `4*width`.
fn get_image_from_pixmap(item: &TrayMenu, size: u32) -> Result<Image> {
    const BITS_PER_SAMPLE: i32 = 8;

    let pixmap = item
        .icon_pixmap
        .as_ref()
        .and_then(|pixmap| pixmap.first())
        .ok_or_else(|| Report::msg("Failed to get pixmap from tray icon"))?;

    let bytes = glib::Bytes::from(&pixmap.pixels);
    let row_stride = pixmap.width * 4;

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
    ImageProvider::create_and_load_surface(&pixbuf, &image)?;
    Ok(image)
}
