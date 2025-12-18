use crate::gtk_helpers::IronbarPaintableExt;
use crate::modules::tray::interface::TrayMenu;
use color_eyre::{Report, Result};
use gtk::gdk::Texture;
use gtk::gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::prelude::WidgetExt;
use gtk::{ContentFit, IconLookupFlags, IconTheme, Picture, TextDirection};
use system_tray::item::IconPixmap;

pub fn get_image(
    item: &TrayMenu,
    size: u32,
    prefer_icons: bool,
    icon_theme: &IconTheme,
) -> Result<Picture> {
    if !prefer_icons && item.icon_pixmap.is_some() {
        get_image_from_pixmap(item.icon_pixmap.as_deref(), size)
    } else {
        get_image_from_icon_name(item, size, icon_theme)
            .or_else(|_| get_image_from_pixmap(item.icon_pixmap.as_deref(), size))
    }
}

/// Attempts to get a GTK `Image` component
/// for the status notifier item's icon.
fn get_image_from_icon_name(item: &TrayMenu, size: u32, icon_theme: &IconTheme) -> Result<Picture> {
    if let Some(path) = item.icon_theme_path.as_ref()
        && !path.as_os_str().is_empty()
        && !icon_theme.search_path().contains(path)
    {
        icon_theme.add_search_path(path);
    }

    let picture = Picture::new();
    picture.set_content_fit(ContentFit::ScaleDown);

    let paintable = item
        .icon_name
        .as_ref()
        .filter(|i| !i.is_empty())
        .map(|icon_name| {
            icon_theme.lookup_icon(
                icon_name,
                &[],
                size as i32,
                picture.scale_factor(),
                TextDirection::None,
                IconLookupFlags::empty(),
            )
        });

    if let Some(paintable) = paintable {
        picture.set_paintable(Some(&paintable));
        Ok(picture)
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
fn get_image_from_pixmap(item: Option<&[IconPixmap]>, size: u32) -> Result<Picture> {
    const BITS_PER_SAMPLE: i32 = 8;

    let pixmap = item
        .and_then(|pixmap| find_approx_size(pixmap, size))
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

    let texture = Texture::for_pixbuf(&pixbuf).scale(size as f64, size as f64);

    let picture = Picture::new();
    picture.set_content_fit(ContentFit::ScaleDown);
    picture.set_paintable(texture.as_ref());

    Ok(picture)
}

///  Finds the `IconPixmap`
///  which is the smallest but bigger than wanted,
///  or the biggest of all if no bigger than wanted.
fn find_approx_size(v: &[IconPixmap], size: u32) -> Option<&IconPixmap> {
    let size = size as i32;

    if v.is_empty() {
        return None;
    }

    if v.len() == 1 {
        return v.first();
    }

    let mut approx = &v[0];

    for p in &v[1..] {
        if (p.width >= size && (approx.width < size || p.width < approx.width))
            || (p.width < size && p.width > approx.width)
        {
            approx = p;
        }
    }

    Some(approx)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_find_approx_height() {
        use super::{IconPixmap, find_approx_size};

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
