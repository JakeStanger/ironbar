#!/usr/bin/env bash

# Migrates CSS selectors from widget names to CSS classes.
# These changed as part of the 0.12 release.

# ⚠ This script will **NOT** check for custom styles and may mangle them!
# ⚠ It is *highly recommended* that you back up your existing styles before running this!

style_path="$HOME/.config/ironbar/style.css"

# general
sed -i 's/#icon/.icon/g' "$style_path"
sed -i 's/#label/.label/g' "$style_path"
sed -i 's/#image/.image/g' "$style_path"

# clipboard
sed -i 's/#clipboard/.clipboard/g' "$style_path"
sed -i 's/#popup-clipboard/.popup-clipboard/g' "$style_path"

# clock
sed -i 's/#clock/.clock/g' "$style_path"
sed -i 's/#popup-clock/.popup-clock/g' "$style_path"
sed -i 's/#calendar-clock/.calendar-clock/g' "$style_path"
sed -i 's/#calendar/.calendar/g' "$style_path"

# custom
sed -i 's/#custom/.custom/g' "$style_path"
sed -i 's/#popup-custom/.popup-custom/g' "$style_path"

# focused
sed -i 's/#focused/.focused/g' "$style_path"

# launcher
sed -i 's/#launcher/.launcher/g' "$style_path"
sed -i 's/#popup-launcher/.popup-launcher/g' "$style_path"
sed -i 's/#launcher-popup/.popup-launcher/g' "$style_path" # was incorrect in docs

# music
sed -i 's/#music/.music/g' "$style_path"
sed -i 's/#contents/.contents/g' "$style_path"
sed -i 's/#popup-music/.popup-music/g' "$style_path"
sed -i 's/#album-art/.album-art/g' "$style_path"
sed -i 's/#title/.title/g' "$style_path"
sed -i 's/#album/.album/g' "$style_path"
sed -i 's/#artist/.artist/g' "$style_path"
sed -i 's/#controls/.controls/g' "$style_path"
sed -i 's/#btn-prev/.btn-prev/g' "$style_path"
sed -i 's/#btn-play/.btn-play/g' "$style_path"
sed -i 's/#btn-pause/.btn-pause/g' "$style_path"
sed -i 's/#btn-next/.btn-next/g' "$style_path"
sed -i 's/#volume/.volume/g' "$style_path"
sed -i 's/#slider/.slider/g' "$style_path"

# script
sed -i 's/#script/.script/g' "$style_path"

# sys_info
sed -i 's/#sysinfo/.sysinfo/g' "$style_path"
sed -i 's/#item/.item/g' "$style_path"

# tray
sed -i 's/#tray/.tray/g' "$style_path"

# upower
sed -i 's/#upower/.upower/g' "$style_path"
sed -i 's/#button/.button/g' "$style_path"
sed -i 's/#popup-upower/.popup-upower/g' "$style_path"
sed -i 's/#upower-details/.upower-details/g' "$style_path"

# workspaces
sed -i 's/#workspaces/.workspaces/g' "$style_path"
sed -i 's/#item/.item/g' "$style_path"