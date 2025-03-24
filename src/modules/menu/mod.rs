mod config;
mod ui;

use color_eyre::Result;
use color_eyre::eyre::Report;
use config::{CustomEntry, OTHER_LABEL, parse_config};
use gtk::prelude::*;
use gtk::{Align, Button, ContentFit, Label, Orientation};
use indexmap::IndexMap;
use serde::Deserialize;
use tokio::sync::mpsc;

use super::{ModuleLocation, PopupButton};
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::BarPosition;
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext,
};
use crate::{module_impl, spawn};

pub use config::MenuModule;

/// XDG button and menu from parsed config.
#[derive(Debug, Clone)]
pub struct XdgSection {
    pub label: String,
    pub icon: Option<String>,
    pub applications: IndexMap<String, MenuApplication>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MenuApplication {
    pub label: String,
    pub file_name: String,
    pub categories: Vec<String>,
}

#[derive(Debug)]
pub enum MenuEntry {
    Xdg(XdgSection),
    Custom(CustomEntry),
}

impl MenuEntry {
    pub fn label(&self) -> String {
        match self {
            Self::Xdg(entry) => entry.label.clone(),
            Self::Custom(entry) => entry.label.clone(),
        }
    }

    pub fn icon(&self) -> Option<String> {
        match self {
            Self::Xdg(entry) => entry.icon.clone(),
            Self::Custom(entry) => entry.icon.clone(),
        }
    }
}

impl Module<Button> for MenuModule {
    type SendMessage = Vec<MenuApplication>;
    type ReceiveMessage = ();

    module_impl!("menu");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        // let max_label_length = self.max_label_length;

        let desktop_files = context.ironbar.desktop_files();

        spawn(async move {
            // parsing all desktop files is heavy so wait until the popup is first opened before loading
            rx.recv().await;

            let apps = desktop_files
                .get_all()
                .await?
                .into_iter()
                .filter(|file| {
                    file.no_display != Some(true)
                        && file.app_type.as_deref().is_some_and(|v| v == "Application")
                })
                .map(|file| MenuApplication {
                    label: file.name.unwrap_or_default(),
                    file_name: file.file_name,
                    categories: file.categories,
                })
                .collect::<Vec<_>>();

            tx.send_update_spawn(apps);
            Ok::<(), Report>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<Button>> {
        let button = Button::new();
        let button_contents = gtk::Box::new(Orientation::Horizontal, 5);
        button.set_child(Some(&button_contents));

        if let Some(ref label_icon) = self.label_icon {
            let image_provider = context.ironbar.image_provider();

            let gtk_image = gtk::Picture::builder()
                .content_fit(ContentFit::ScaleDown)
                .build();
            button_contents.append(&gtk_image);

            let label_icon = label_icon.clone();

            glib::spawn_future_local(async move {
                image_provider
                    .load_into_picture_silent(&label_icon, self.label_icon_size, true, &gtk_image)
                    .await;
            });
        }

        if let Some(ref label) = self.label {
            let label = Label::new(Some(label));
            button_contents.append(&label);
        }

        let tx = context.tx.clone();
        let controller_tx = context.controller_tx.clone();
        button.connect_clicked(move |button| {
            tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));

            // channel will close after init event
            if !controller_tx.is_closed() {
                controller_tx.send_spawn(());
            }
        });

        let popup = self
            .into_popup(context, info)
            .into_popup_parts(vec![&button]);

        Ok(ModuleParts::new(button, popup))
    }

    fn into_popup(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box> {
        let image_provider = context.ironbar.image_provider();

        let alignment = {
            match info.bar_position {
                // For fixed height menus always align to the top
                _ if self.height.is_some() => Align::Start,

                // Otherwise alignment is based on menu position
                BarPosition::Top => Align::Start,
                BarPosition::Bottom => Align::End,

                _ => match &info.location {
                    &ModuleLocation::Left | &ModuleLocation::Center => Align::Start,
                    &ModuleLocation::Right => Align::End,
                },
            }
        };

        let mut sections_by_cat = IndexMap::<String, Vec<String>>::new();

        let container = gtk::Box::new(Orientation::Horizontal, 4);

        let main_menu = gtk::Box::new(Orientation::Vertical, 0);
        main_menu.set_valign(alignment);
        main_menu.set_vexpand(false);
        main_menu.add_css_class("main");

        if let Some(width) = self.width {
            main_menu.set_width_request(width / 2);
        }

        if let Some(max_height) = self.height {
            container.set_height_request(max_height);

            let scrolled = gtk::ScrolledWindow::builder()
                .max_content_height(max_height)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build();

            scrolled.set_child(Some(&main_menu));
            container.append(&scrolled);
        } else {
            container.append(&main_menu);
        }

        let mut start_entries = parse_config(self.start, &mut sections_by_cat);
        let mut center_entries = parse_config(self.center, &mut sections_by_cat);
        let mut end_entries = parse_config(self.end, &mut sections_by_cat);

        let start_section = gtk::Box::new(Orientation::Vertical, 0);
        let center_section = gtk::Box::new(Orientation::Vertical, 0);
        let end_section = gtk::Box::new(Orientation::Vertical, 0);

        start_section.add_css_class("main-start");
        center_section.add_css_class("main-center");
        end_section.add_css_class("main-end");

        let truncate_mode = self.truncate;

        context.subscribe().recv_glib(
            (
                &main_menu,
                &container,
                &start_section,
                &center_section,
                &end_section,
            ),
            move |(main_menu, container, start_section, center_section, end_section),
                  applications| {
                for application in &applications {
                    let mut inserted = false;

                    for category in &application.categories {
                        if let Some(section_names) = sections_by_cat.get(category) {
                            for section_name in section_names {
                                for entries in
                                    [&mut start_entries, &mut center_entries, &mut end_entries]
                                {
                                    {
                                        let existing = entries.get_mut(section_name);
                                        if let Some(MenuEntry::Xdg(existing)) = existing {
                                            existing.applications.insert_sorted(
                                                application.label.clone(),
                                                application.clone(),
                                            );
                                        }
                                    }
                                }
                            }
                            inserted = true;
                        }
                    }

                    if !inserted {
                        let other = center_entries.get_mut(OTHER_LABEL);
                        if let Some(MenuEntry::Xdg(other)) = other {
                            let _ = other
                                .applications
                                .insert_sorted(application.label.clone(), application.clone());
                        }
                    }
                }

                main_menu.children().for_each(|child| {
                    main_menu.remove(&child);
                });

                macro_rules! add_entries {
                    ($entries:expr, $section:expr) => {
                        for entry in $entries.values() {
                            let container1 = container.clone();
                            let tx = context.tx.clone();
                            let (button, sub_menu) = ui::make_entry(
                                entry,
                                &tx,
                                &image_provider,
                                truncate_mode,
                                &self.launch_command,
                            );

                            if let Some(sub_menu) = sub_menu.clone() {
                                sub_menu.set_valign(alignment);
                                sub_menu.add_css_class("sub-menu");
                                if let Some(width) = self.width {
                                    sub_menu.set_width_request(width / 2);
                                }
                            }

                            ui::add_entries(
                                entry,
                                &button,
                                sub_menu.as_ref(),
                                $section,
                                &container1,
                                self.height,
                            );
                        }
                    };
                }

                add_entries!(&start_entries, &start_section);
                add_entries!(&center_entries, &center_section);
                add_entries!(&end_entries, &end_section);

                main_menu.append(start_section);
                main_menu.append(center_section);
                main_menu.append(end_section);
            },
        );

        {
            let container = container.clone();
            context.popup.popover.connect_hide(move |_| {
                start_section.children().for_each(|child| {
                    child.remove_css_class("open");
                });

                center_section.children().for_each(|child| {
                    child.remove_css_class("open");
                });

                end_section.children().for_each(|child| {
                    child.remove_css_class("open");
                });

                container.children().skip(1).for_each(|sub_menu| {
                    sub_menu.set_visible(false);
                });
            });
        }

        Some(container)
    }
}
