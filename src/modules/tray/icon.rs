use crate::image::ImageProvider;
use crate::modules::tray::interface::TrayMenu;
use color_eyre::{Report, Result};
use glib::ffi::g_strfreev;
use glib::translate::ToGlibPtr;
use gtk::ffi::gtk_icon_theme_get_search_path;
use gtk::gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use gtk::{IconLookupFlags, IconTheme, Image};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

pub fn get_image(
    item: &TrayMenu,
    icon_theme: &IconTheme,
    size: u32,
    prefer_icons: bool,
) -> Result<Image> {
    let icon = item
        .icon_name
        .as_ref()
        .and_then(|icon_name| ImageProvider::parse(&icon_name, &icon_theme, false, size as i32));

    //map(|provider| provider.load_into_image(&icon));
    if let Some(provider) = icon {
        let image = Image::new();
        provider.load_into_image(&image);
        Ok(image)
    } else {
        Err(Report::msg("could not find icon"))
    }
}
