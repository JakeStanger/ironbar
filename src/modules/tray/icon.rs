use glib::ffi::g_strfreev;
use glib::translate::ToGlibPtr;
use gtk::ffi::gtk_icon_theme_get_search_path;
use gtk::gdk_pixbuf::{Colorspace, InterpType};
use gtk::prelude::IconThemeExt;
use gtk::{gdk_pixbuf, IconLookupFlags, IconTheme, Image};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use system_tray::message::tray::StatusNotifierItem;

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

/// Attempts to get a GTK `Image` component
/// for the status notifier item's icon.
pub(crate) fn get_image_from_icon_name(
    item: &StatusNotifierItem,
    icon_theme: &IconTheme,
) -> Option<Image> {
    if let Some(path) = item.icon_theme_path.as_ref() {
        if !path.is_empty() && !get_icon_theme_search_paths(icon_theme).contains(path) {
            icon_theme.append_search_path(path);
        }
    }

    item.icon_name.as_ref().and_then(|icon_name| {
        let icon_info = icon_theme.lookup_icon(icon_name, 16, IconLookupFlags::empty());
        icon_info.map(|icon_info| Image::from_pixbuf(icon_info.load_icon().ok().as_ref()))
    })
}

/// Attempts to get an image from the item pixmap.
///
/// The pixmap is supplied in ARGB32 format,
/// which has 8 bits per sample and a bit stride of `4*width`.
pub(crate) fn get_image_from_pixmap(item: &StatusNotifierItem) -> Option<Image> {
    const BITS_PER_SAMPLE: i32 = 8;

    let pixmap = item
        .icon_pixmap
        .as_ref()
        .and_then(|pixmap| pixmap.first())?;

    let bytes = glib::Bytes::from(&pixmap.pixels);
    let row_stride = pixmap.width * 4; //

    let pixbuf = gdk_pixbuf::Pixbuf::from_bytes(
        &bytes,
        Colorspace::Rgb,
        true,
        BITS_PER_SAMPLE,
        pixmap.width,
        pixmap.height,
        row_stride,
    );

    let pixbuf = pixbuf
        .scale_simple(16, 16, InterpType::Bilinear)
        .unwrap_or(pixbuf);
    Some(Image::from_pixbuf(Some(&pixbuf)))
}
