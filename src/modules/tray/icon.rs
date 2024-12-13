use crate::image::ImageProvider;
use crate::modules::tray::interface::TrayMenu;
use cairo::ImageSurface;
use color_eyre::{Report, Result};
use glib::ffi::g_strfreev;
use glib::translate::ToGlibPtr;
use gtk::ffi::gtk_icon_theme_get_search_path;
use gtk::gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::prelude::{GdkContextExt, IconThemeExt};
use gtk::{IconLookupFlags, IconTheme, Image};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use system_tray::item::IconPixmap;

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
    let pixbuf = if !prefer_icons && item.icon_pixmap.is_some() {
        get_image_from_pixmap(item, size)
    } else {
        get_image_from_icon_name(item, icon_theme, size)
            .or_else(|_| get_image_from_pixmap(item, size))
    }?;

    let image = if pixbuf.height() == size as i32 {
        let image = Image::new();
        ImageProvider::create_and_load_surface(&pixbuf, &image)?;
        image
    } else {
        Image::from_surface(Some(&scale_image_to_height(pixbuf, size as i32)))
    };

    Ok(image)
}

fn scale_image_to_height(pixbuf: Pixbuf, size: i32) -> ImageSurface {
    let scale = size as f64 / pixbuf.height() as f64;
    let width = (pixbuf.width() as f64 * scale).ceil() as i32;
    let height = (pixbuf.height() as f64 * scale).ceil() as i32;

    let surf = ImageSurface::create(cairo::Format::ARgb32, width, height)
        .expect("Failed to create image surface");
    let context = cairo::Context::new(&surf).expect("Failed to create cairo context");

    context.scale(scale, scale);
    context.set_source_pixbuf(&pixbuf, 0., 0.);
    context.paint().expect("Failed to paint scaled image");

    surf
}

/// Attempts to get a GTK `Image` component
/// for the status notifier item's icon.
fn get_image_from_icon_name(item: &TrayMenu, icon_theme: &IconTheme, size: u32) -> Result<Pixbuf> {
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
        Ok(pixbuf)
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
fn get_image_from_pixmap(item: &TrayMenu, size: u32) -> Result<Pixbuf> {
    const BITS_PER_SAMPLE: i32 = 8;

    let pixmap = item
        .icon_pixmap
        .as_ref()
        // The vec is sorted(ASC) with size(width==height) most of the time,
        // but we can not be sure that it'll always sorted by `height`
        .and_then(|pixmap| find_approx_size(pixmap, size as i32))
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

    Ok(pixbuf)
}

///  finds the pixmap
///  which is the smallest but bigger than wanted
///  or
///  the biggest of all if no bigger than wanted
///
///  O(n)
fn find_approx_size(v: &[IconPixmap], size: i32) -> Option<&IconPixmap> {
    if v.is_empty() {
        return None;
    }
    if v.len() == 1 {
        return v.first();
    }

    let mut approx = &v[0];

    for p in &v[1..] {
        // p bigger than wanted size
        // and then we check for
        // `approx` is smaller than wanted || p smaller than `approx`
        if (p.width >= size && (approx.width < size || p.width < approx.width))
                // or p smaller than wanted
                // but bigger than `approx`
                || (p.width < size && p.width > approx.width)
        {
            approx = p;
        }
    }

    Some(approx)
}

mod tests {

    #[test]
    fn test_find_approx_height() {
        use super::{find_approx_size, IconPixmap};

        macro_rules! make_list {
            ($heights:expr) => {
                $heights
                    .iter()
                    .map(|width| IconPixmap {
                        width: *width,
                        height: 0,
                        pixels: vec![],
                    })
                    .collect::<Vec<IconPixmap>>()
            };
        }
        macro_rules! assert_correct {
            ($list:expr, $width:expr, $index:expr) => {
                assert_eq!(
                    find_approx_size(&$list, $width).unwrap().width,
                    $list[$index].width
                );
            };
        }

        let list = make_list!([10, 20, 50, 40, 30]);
        assert_correct!(list, 1, 0);
        assert_correct!(list, 10, 0);
        assert_correct!(list, 11, 1);
        assert_correct!(list, 20, 1);
        assert_correct!(list, 21, 4);
        assert_correct!(list, 30, 4);
        assert_correct!(list, 31, 3);
        assert_correct!(list, 40, 3);
        assert_correct!(list, 41, 2);
        assert_correct!(list, 50, 2);
        assert_correct!(list, 51, 2);
    }
}
