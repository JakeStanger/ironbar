use color_eyre::eyre::Report;
use color_eyre::Result;
use freedesktop_entry_parser::Entry;
use glib::Propagation;
use gtk::{prelude::*, IconTheme};
use gtk::{Align, Button, Label, Orientation};
use indexmap::IndexMap;
use serde::Deserialize;
use std::process::{Command, Stdio};
use tokio::sync::{broadcast, mpsc};
use unicode_segmentation::UnicodeSegmentation;

use crate::config::{BarPosition, CommonConfig};
use crate::desktop_file::find_desktop_files;
use crate::image::ImageProvider;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::script::Script;
use crate::{glib_recv, module_impl, spawn, try_send};
use tracing::{debug, error};

use super::ModuleLocation;

const fn default_length() -> usize {
    25
}

fn default_menu_popup_label() -> Option<String> {
    Some("≡".to_string())
}

const fn default_menu_popup_icon_size() -> i32 {
    16
}

const OTHER_LABEL: &str = "Other";

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MenuConfig {
    XdgEntry(XdgEntry),
    XdgOther,
    Custom(CustomEntry),
}

#[derive(Debug, Deserialize, Clone)]
pub struct XdgEntry {
    label: String,

    #[serde(default)]
    icon: Option<String>,

    categories: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomEntry {
    icon: Option<String>,
    label: String,
    on_click: String,
}

#[derive(Debug, Clone)]
pub struct XdgSection {
    label: String,
    icon: Option<String>,
    applications: IndexMap<String, MenuApplication>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MenuApplication {
    label: String,
    file_name: String,
    categories: Vec<String>,
}
enum MenuEntry {
    Xdg(XdgSection),
    Custom(CustomEntry),
}

impl MenuEntry {
    fn label(&self) -> String {
        match self {
            Self::Xdg(entry) => entry.label.clone(),
            Self::Custom(entry) => entry.label.clone(),
        }
    }
    fn icon(&self) -> Option<String> {
        match self {
            Self::Xdg(entry) => entry.icon.clone(),
            Self::Custom(entry) => entry.icon.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MenuModule {
    #[serde(default)]
    start: Vec<MenuConfig>,

    #[serde(default = "default_menu")]
    center: Vec<MenuConfig>,

    #[serde(default)]
    end: Vec<MenuConfig>,

    #[serde(default)]
    height: Option<i32>,

    #[serde(default)]
    width: Option<i32>,

    #[serde(default = "default_length")]
    max_label_length: usize,

    #[serde(default = "default_menu_popup_label")]
    label: Option<String>,

    #[serde(default)]
    label_icon: Option<String>,

    #[serde(default = "default_menu_popup_icon_size")]
    label_icon_size: i32,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for MenuModule {
    fn default() -> Self {
        MenuModule {
            start: vec![],
            center: default_menu(),
            end: vec![],
            height: None,
            width: None,
            max_label_length: default_length(),
            label: default_menu_popup_label(),
            label_icon: None,
            label_icon_size: default_menu_popup_icon_size(),
            common: Some(CommonConfig::default()),
        }
    }
}

fn default_menu() -> Vec<MenuConfig> {
    vec![
        MenuConfig::XdgEntry(XdgEntry {
            label: "Settings".to_string(),
            icon: Some("preferences-system".to_string()),
            categories: vec!["Settings".to_string(), "Screensaver".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Accessories".to_string(),
            icon: Some("accessories".to_string()),
            categories: vec![
                "Accessibility".to_string(),
                "Core".to_string(),
                "Legacy".to_string(),
                "Utility".to_string(),
            ],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Development".to_string(),
            icon: Some("applications-development".to_string()),
            categories: vec!["Development".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Education".to_string(),
            icon: Some("applications-education".to_string()),
            categories: vec!["Education".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Games".to_string(),
            icon: Some("applications-games".to_string()),
            categories: vec!["Game".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Graphics".to_string(),
            icon: Some("applications-graphics".to_string()),
            categories: vec!["Graphics".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Multimedia".to_string(),
            icon: Some("applications-multimedia".to_string()),
            categories: vec![
                "Audio".to_string(),
                "Video".to_string(),
                "AudioVideo".to_string(),
            ],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Network".to_string(),
            icon: Some("applications-internet".to_string()),
            categories: vec!["Network".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Office".to_string(),
            icon: Some("applications-office".to_string()),
            categories: vec!["Office".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "Science".to_string(),
            icon: Some("applications-science".to_string()),
            categories: vec!["Science".to_string()],
        }),
        MenuConfig::XdgEntry(XdgEntry {
            label: "System".to_string(),
            icon: Some("applications-system".to_string()),
            categories: vec!["Emulator".to_string(), "System".to_string()],
        }),
        MenuConfig::XdgOther,
    ]
}

/*
type:xdg
categories: [Foo, Bar]
 */

fn parse_config(
    section_config: Vec<MenuConfig>,
    mut sections_by_cat: IndexMap<String, Vec<String>>,
) -> (IndexMap<String, MenuEntry>, IndexMap<String, Vec<String>>) {
    let mut entries = IndexMap::<String, MenuEntry>::new();
    section_config
        .iter()
        .for_each(|entry_config| match entry_config {
            MenuConfig::XdgEntry(entry) => {
                entry.categories.iter().for_each(|cat| {
                    let existing = sections_by_cat.get_mut(cat);
                    if let Some(existing) = existing {
                        existing.push(entry.label.clone());
                    } else {
                        sections_by_cat.insert(cat.clone(), vec![entry.label.clone()]);
                    }
                });
                let _ = entries.insert_sorted(
                    entry.label.clone(),
                    MenuEntry::Xdg(XdgSection {
                        label: entry.label.clone(),
                        icon: entry.icon.clone(),
                        applications: IndexMap::new(),
                    }),
                );
            }
            MenuConfig::XdgOther => {
                let _ = entries.insert_sorted(
                    OTHER_LABEL.to_string(),
                    MenuEntry::Xdg(XdgSection {
                        label: OTHER_LABEL.to_string(),
                        icon: Some("applications-other".to_string()),
                        applications: IndexMap::new(),
                    }),
                );
            }
            MenuConfig::Custom(entry) => {
                let _ = entries.insert_sorted(
                    entry.label.clone(),
                    MenuEntry::Custom(CustomEntry {
                        icon: entry.icon.clone(),
                        label: entry.label.clone(),
                        on_click: entry.on_click.clone(),
                    }),
                );
            }
        });
    (entries, sections_by_cat)
}

fn make_entry<R: Clone + 'static>(
    entry: &MenuEntry,
    tx: mpsc::Sender<ModuleUpdateEvent<R>>,
    icon_theme: IconTheme,
) -> (Button, Option<gtk::Box>) {
    let button = Button::new();
    let button_container = gtk::Box::new(Orientation::Horizontal, 4);
    let label = Label::builder().label(entry.label()).build();
    label.set_halign(Align::Start);
    button.add(&button_container);

    if let Some(icon_name) = entry.icon() {
        let gtk_image = gtk::Image::new();
        gtk_image.set_halign(Align::Start);
        let image = ImageProvider::parse(&icon_name, &icon_theme, true, 16);
        if let Some(image) = image {
            button_container.add(&gtk_image);

            if let Err(err) = image.load_into_image(gtk_image) {
                error!("{err:?}");
            }
        };
    }
    button_container.add(&label);
    button_container.foreach(|child| {
        child.set_halign(Align::Start);
    });
    if let MenuEntry::Xdg(_) = entry {
        let right_arrow = Label::builder().label("🢒").build();
        right_arrow.set_halign(Align::End);
        button_container.pack_end(&right_arrow, false, false, 0);
    }

    button.show_all();

    let sub_menu = match entry {
        MenuEntry::Xdg(entry) => {
            let sub_menu = gtk::Box::new(Orientation::Vertical, 0);
            entry.applications.values().for_each(|sub_entry| {
                let mut button = Button::builder();
                button = button.label(sub_entry.label.clone());
                let button = button.build();

                let icon_name = sub_entry.file_name.trim_end_matches(".desktop");
                let gtk_image = gtk::Image::new();
                let image = ImageProvider::parse(icon_name, &icon_theme, true, 16);
                if let Some(image) = image {
                    button.set_image(Some(&gtk_image));
                    button.set_always_show_image(true);

                    if let Err(err) = image.load_into_image(gtk_image) {
                        error!("{err:?}");
                    }
                };
                button.foreach(|child| {
                    child.set_halign(Align::Start);
                });
                sub_menu.add(&button);

                {
                    let sub_menu = sub_menu.clone();
                    let file_name = sub_entry.file_name.clone();
                    let tx = tx.clone();
                    button.connect_clicked(move |_button| {
                        let _ = Command::new("gtk-launch")
                            .arg(file_name.clone())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn();
                        sub_menu.hide();
                        try_send!(tx, ModuleUpdateEvent::ClosePopup);
                    });
                }

                button.show_all();
            });
            Some(sub_menu)
        }
        MenuEntry::Custom(_) => None,
    };

    (button, sub_menu)
}

fn add_entries(
    entry: &MenuEntry,
    button: Button,
    sub_menu: Option<gtk::Box>,
    main_menu: gtk::Box,
    container: gtk::Box,
    height: Option<i32>,
) {
    let container1 = container.clone();
    main_menu.add(&button);

    if let Some(sub_menu) = sub_menu {
        if let Some(height) = height {
            container.set_height_request(height);
            let scrolled = gtk::ScrolledWindow::builder()
                .max_content_height(height)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build();
            sub_menu.show();
            scrolled.add(&sub_menu);
            container.add(&scrolled);

            let sub_menu1 = scrolled.clone();
            let sub_menu_popup_container = sub_menu.clone();
            button.connect_clicked(move |_button| {
                container1.children().iter().skip(1).for_each(|sub_menu| {
                    if sub_menu.get_visible() {
                        sub_menu.hide();
                    }
                });
                sub_menu1.show();
                // Reset scroll to top.
                if let Some(w) = sub_menu_popup_container.children().first() {
                    w.set_has_focus(true)
                }
            });
        } else {
            container.add(&sub_menu);
            let sub_menu1 = sub_menu.clone();
            button.connect_clicked(move |_button| {
                container1.children().iter().skip(1).for_each(|sub_menu| {
                    if sub_menu.get_visible() {
                        sub_menu.hide();
                    }
                });
                sub_menu1.show();
            });
        }
    }
    if let MenuEntry::Custom(entry) = entry {
        let label = entry.on_click.clone();
        let container = container.clone();
        button.connect_clicked(move |_button| {
            container.children().iter().skip(1).for_each(|sub_menu| {
                sub_menu.hide();
            });
            let script = Script::from(label.as_str());
            debug!("executing command: '{}'", script.cmd);

            let args = Vec::new();

            spawn(async move {
                if let Err(err) = script.get_output(Some(&args)).await {
                    error!("{err:?}");
                }
            });
        });
    }
    main_menu.show_all();
}

impl Module<Button> for MenuModule {
    type SendMessage = Vec<MenuApplication>;
    type ReceiveMessage = ();

    module_impl!("menu");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        let max_label_length = self.max_label_length;

        spawn(async move {
            let files = find_desktop_files();
            let apps = files
                .iter()
                .filter_map(|file_path| {
                    let file_name = file_path
                        .as_path()
                        .file_name()
                        .expect("find_desktop_files returned empty pathbuf")
                        .to_string_lossy()
                        .into_owned();
                    // TODO filter out results that fail TryExec
                    let entry = Entry::parse_file(file_path).ok()?;
                    let desktop = entry.section("Desktop Entry");
                    let typ = desktop.attr("Type").unwrap_or("N/A");
                    if typ != "Application" {
                        return None;
                    }
                    let raw_cats = desktop.attr("Categories").unwrap_or("Misc");
                    let categories = raw_cats
                        .trim_end_matches(';')
                        .split(';')
                        .map(|s| s.to_string())
                        .collect();
                    let mut name = desktop.attr("Name")?.to_string();

                    if name.graphemes(true).count() > max_label_length {
                        name = name
                            .graphemes(true)
                            .take(max_label_length - 1)
                            .collect::<String>();
                        name += "…";
                    }

                    // Some .desktop files are only for associating mimetypes
                    if desktop.attr("NoDisplay") == Some("true") {
                        return None;
                    }
                    Some(MenuApplication {
                        label: name,
                        file_name,
                        categories,
                    })
                })
                .collect::<Vec<MenuApplication>>();
            try_send!(tx, ModuleUpdateEvent::Update(apps));
            Ok::<(), Report>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<Button>> {
        let icon_theme = info.icon_theme.clone();
        let button = Button::new();

        if let Some(ref label) = self.label {
            button.set_label(label);
        }

        if let Some(ref label_icon) = self.label_icon {
            let gtk_image = gtk::Image::new();
            let image = ImageProvider::parse(label_icon, &icon_theme, true, self.label_icon_size);
            if let Some(image) = image {
                button.set_image(Some(&gtk_image));
                button.set_always_show_image(true);

                if let Err(err) = image.load_into_image(gtk_image) {
                    error!("{err:?}");
                }
            };
        }

        let tx = context.tx.clone();
        button.connect_clicked(move |button| {
            try_send!(tx, ModuleUpdateEvent::TogglePopup(button.popup_id()));
        });

        let popup = self
            .into_popup(
                context.controller_tx.clone(),
                context.subscribe(),
                context,
                info,
            )
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        _tx: mpsc::Sender<Self::ReceiveMessage>,
        rx: broadcast::Receiver<Self::SendMessage>,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let icon_theme = info.icon_theme.clone();
        let alignment = {
            match info.bar_position {
                // For fixed height menus always align to the top
                _ if self.height.is_some() => gtk::Align::Start,
                // Otherwise alignment is based on menu position
                BarPosition::Top => gtk::Align::Start,
                BarPosition::Bottom => gtk::Align::End,
                _ => match &info.location {
                    &ModuleLocation::Left | &ModuleLocation::Center => gtk::Align::Start,
                    &ModuleLocation::Right => gtk::Align::End,
                },
            }
        };

        let sections_by_cat = IndexMap::<String, Vec<String>>::new();
        let container = gtk::Box::new(Orientation::Horizontal, 4);
        container.style_context().add_class("menu-popup");
        let main_menu = gtk::Box::new(Orientation::Vertical, 0);
        main_menu.set_valign(alignment);
        main_menu.set_vexpand(false);
        main_menu.style_context().add_class("menu-popup_main");

        if let Some(width) = self.width {
            main_menu.set_width_request(width / 2);
        }

        if let Some(max_height) = self.height {
            container.set_height_request(max_height);
            let scrolled = gtk::ScrolledWindow::builder()
                .max_content_height(max_height)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build();
            scrolled.add(&main_menu);
            container.add(&scrolled);
        } else {
            container.add(&main_menu);
        }
        container.show_all();

        let (mut start_entries, sections_by_cat) = parse_config(self.start, sections_by_cat);
        let (mut center_entries, sections_by_cat) = parse_config(self.center, sections_by_cat);
        let (mut end_entries, sections_by_cat) = parse_config(self.end, sections_by_cat);

        let container2 = container.clone();
        {
            let main_menu = main_menu.clone();
            let container = container.clone();
            glib_recv!(rx, applications => {
                for application in applications.iter() {
                    let mut inserted = false;
                    for category in application.categories.iter() {
                        if let Some(section_names) = sections_by_cat.get(category) {
                                for section_name in section_names.iter() {
                                    let existing = start_entries.get_mut(section_name);
                                    if let Some(MenuEntry::Xdg(existing)) = existing {
                                        let _ = existing.applications.insert_sorted(application.label.clone(), application.clone());
                                    }
                                    let existing = center_entries.get_mut(section_name);
                                    if let Some(MenuEntry::Xdg(existing)) = existing {
                                        let _ = existing.applications.insert_sorted(application.label.clone(), application.clone());
                                    }
                                    let existing = end_entries.get_mut(section_name);
                                    if let Some(MenuEntry::Xdg(existing)) = existing {
                                        let _ = existing.applications.insert_sorted(application.label.clone(), application.clone());
                                    }
                                };
                                inserted = true;
                            }
                    };
                    if !inserted {
                        let other = center_entries.get_mut(OTHER_LABEL);
                        if let Some(MenuEntry::Xdg(other)) = other {
                            let _ = other.applications.insert_sorted(application.label.clone(), application.clone());
                        }
                    }
                };

                main_menu.foreach(|child| {
                    main_menu.remove(child);
                });
                let start_section = gtk::Box::new(Orientation::Vertical, 0);
                start_section.style_context().add_class("menu-popup_main_start");
                main_menu.add(&start_section);
                for entry in start_entries.values() {
                    let container1 = container.clone();
                    let start_section = start_section.clone();
                    let tx = context.tx.clone();
                    let (button, sub_menu) = make_entry(entry, tx, icon_theme.clone());
                    if let Some(sub_menu) = sub_menu.clone() {
                        sub_menu.set_valign(alignment);
                        sub_menu.style_context().add_class("menu-popup_sub-menu");
                        if let Some(width) = self.width {
                            sub_menu.set_width_request(width / 2);
                        }
                    }
                    add_entries(entry, button, sub_menu, start_section, container1, self.height);
                };
                let center_section = gtk::Box::new(Orientation::Vertical, 0);
                center_section.style_context().add_class("menu-popup_main_center");
                main_menu.add(&center_section);
                for entry in center_entries.values() {
                    let container1 = container.clone();
                    let center_section = center_section.clone();
                    let tx = context.tx.clone();
                    let (button, sub_menu) = make_entry(entry, tx, icon_theme.clone());
                    if let Some(sub_menu) = sub_menu.clone() {
                        sub_menu.set_valign(alignment);
                        sub_menu.style_context().add_class("menu-popup_sub-menu");
                        if let Some(width) = self.width {
                            sub_menu.set_width_request(width / 2);
                        }
                    }
                    add_entries(entry, button, sub_menu, center_section, container1, self.height);
                };
                let end_section = gtk::Box::new(Orientation::Vertical, 0);
                end_section.style_context().add_class("menu-popup_main_end");
                main_menu.add(&end_section);
                for entry in end_entries.values() {
                    let container1 = container.clone();
                    let end_section = end_section.clone();
                    let tx = context.tx.clone();
                    let (button, sub_menu) = make_entry(entry, tx, icon_theme.clone());
                    if let Some(sub_menu) = sub_menu.clone() {
                        sub_menu.set_valign(alignment);
                        sub_menu.style_context().add_class("menu-popup_sub-menu");
                        if let Some(width) = self.width {
                            sub_menu.set_width_request(width / 2);
                        }
                    }
                    add_entries(entry, button, sub_menu, end_section, container1, self.height);
                };
            });

            {
                let pos = info.bar_position;
                let container = container2;
                let win = context.popup.window.clone();
                context
                    .popup
                    .clone()
                    .window
                    .connect_leave_notify_event(move |_button, ev| {
                        const THRESHOLD: f64 = 3.0;
                        let (w, h) = win.size();
                        let (x, y) = ev.position();

                        let hide = match pos {
                            BarPosition::Top => {
                                x < THRESHOLD
                                    || y > f64::from(h) - THRESHOLD
                                    || x > f64::from(w) - THRESHOLD
                            }
                            BarPosition::Bottom => {
                                x < THRESHOLD || y < THRESHOLD || x > f64::from(w) - THRESHOLD
                            }
                            BarPosition::Left => {
                                y < THRESHOLD
                                    || x > f64::from(w) - THRESHOLD
                                    || y > f64::from(h) - THRESHOLD
                            }
                            BarPosition::Right => {
                                y < THRESHOLD || x < THRESHOLD || y > f64::from(h) - THRESHOLD
                            }
                        };

                        if hide {
                            container.children().iter().skip(1).for_each(|sub_menu| {
                                sub_menu.hide();
                            });
                        }

                        Propagation::Proceed
                    });
            }
        }

        Some(container)
    }
}
