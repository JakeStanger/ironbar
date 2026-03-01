use crate::gtk_helpers::IronbarPaintableExt;
use crate::image;
use color_eyre::{Report, Result};
use gtk::gdk::Texture;
use gtk::gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::{ContentFit, Picture};
use std::path::Path;
use system_tray::item::IconPixmap;

/// Attempts to get a GTK `Picture` for the tray item's icon.
///
/// If `prefer_icons` is true (or there is no pixmap), tries the icon name first
/// via the image provider (which handles SVGs, theme icons, and local files),
/// then falls back to raw pixmap data.
///
/// If `prefer_icons` is false and a pixmap is available, uses the pixmap directly.
pub async fn get_image(
    icon_name: Option<&str>,
    icon_theme_path: Option<&Path>,
    icon_pixmap: Option<&[IconPixmap]>,
    size: u32,
    prefer_icons: bool,
    image_provider: &image::Provider,
) -> Result<Picture> {
    if !prefer_icons && icon_pixmap.is_some() {
        get_image_from_pixmap(icon_pixmap, size)
    } else {
        get_image_from_icon_name(icon_name, icon_theme_path, size, image_provider)
            .await
            .or_else(|_| get_image_from_pixmap(icon_pixmap, size))
    }
}

/// Attempts to get a GTK `Picture` for the status notifier item's icon
/// using the image provider, which correctly handles SVGs, theme icons,
/// and local files.
async fn get_image_from_icon_name(
    icon_name: Option<&str>,
    icon_theme_path: Option<&Path>,
    size: u32,
    image_provider: &image::Provider,
) -> Result<Picture> {
    // Add custom icon theme search path if the item specifies one.
    // icon_theme() returns a clone of the GObject reference, so
    // add_search_path mutates the shared underlying GTK IconTheme.
    if let Some(path) = icon_theme_path
        && !path.as_os_str().is_empty()
    {
        let icon_theme = image_provider.icon_theme();
        if !icon_theme.search_path().contains(&path.to_path_buf()) {
            icon_theme.add_search_path(path);
        }
    }

    let icon_name = icon_name
        .filter(|i| !i.is_empty())
        .ok_or_else(|| Report::msg("no icon name"))?;

    let picture = Picture::builder()
        .content_fit(ContentFit::ScaleDown)
        .build();

    // use_fallback=false so we get Ok(false) rather than a fallback icon,
    // allowing the caller to try the pixmap path next.
    let found = image_provider
        .load_into_picture(icon_name, size as i32, false, &picture)
        .await?;

    if found {
        Ok(picture)
    } else {
        Err(Report::msg("could not find icon"))
    }
}

/// Attempts to get an image from the item pixmap.
///
/// The pixmap is supplied in ARGB32 format,
/// which has 8 bits per sample and a bit stride of `4*width`.
/// The Pixbuf expects RGBA32 format, so some channel shuffling is required.
pub(super) fn get_image_from_pixmap(item: Option<&[IconPixmap]>, size: u32) -> Result<Picture> {
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

/// Finds the `IconPixmap` which is the smallest but bigger than wanted,
/// or the biggest of all if none are bigger than wanted.
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
