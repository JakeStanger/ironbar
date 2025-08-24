use super::MenuEntry;
use crate::channels::AsyncSenderExt;
use crate::config::TruncateMode;
use crate::desktop_file::open_program;
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::image::IconLabel;
use crate::modules::ModuleUpdateEvent;
use crate::script::Script;
use crate::{image, spawn};
use gtk::prelude::*;
use gtk::{Align, Button, Label, Orientation};
use tokio::sync::mpsc;
use tracing::{debug, error};

pub fn make_entry<R>(
    entry: &MenuEntry,
    tx: mpsc::Sender<ModuleUpdateEvent<R>>,
    image_provider: &image::Provider,
    truncate_mode: TruncateMode,
    launch_command_str: &str,
) -> (Button, Option<gtk::Box>)
where
    R: Send + Clone + 'static,
{
    let button = Button::new();
    button.add_class("category");

    let button_container = gtk::Box::new(Orientation::Horizontal, 4);
    button.add(&button_container);

    let label = Label::new(Some(&entry.label()));
    label.set_halign(Align::Start);
    label.truncate(truncate_mode);

    if let Some(icon_name) = entry.icon() {
        let image = IconLabel::new(&icon_name, 16, image_provider);
        image.set_halign(Align::Start);
        button_container.add(&*image);
    }

    button_container.add(&label);
    button_container.foreach(|child| {
        child.set_halign(Align::Start);
    });

    if let MenuEntry::Xdg(_) = entry {
        let right_arrow = Label::new(Some("🢒"));
        right_arrow.set_halign(Align::End);
        button_container.pack_end(&right_arrow, false, false, 0);
    }

    button.show_all();

    let sub_menu = match entry {
        MenuEntry::Xdg(entry) => {
            let sub_menu = gtk::Box::new(Orientation::Vertical, 0);

            entry.applications.values().for_each(|sub_entry| {
                let button = Button::new();
                button.add_class("application");

                let button_container = gtk::Box::new(Orientation::Horizontal, 4);
                button.add(&button_container);

                let label = Label::new(Some(&sub_entry.label));
                label.set_halign(Align::Start);
                label.truncate(truncate_mode);

                let icon_name = sub_entry.file_name.trim_end_matches(".desktop").to_string();
                let gtk_image = gtk::Image::new();
                gtk_image.set_halign(Align::Start);

                button_container.add(&gtk_image);
                button_container.add(&label);

                let image_provider = image_provider.clone();

                glib::spawn_future_local(async move {
                    image_provider
                        .load_into_image_silent(&icon_name, 16, true, &gtk_image)
                        .await;
                });

                button.foreach(|child| {
                    child.set_halign(Align::Start);
                });

                sub_menu.add(&button);

                {
                    let sub_menu = sub_menu.clone();
                    let file_name = sub_entry.file_name.clone();
                    let command = launch_command_str.to_string();
                    let tx = tx.clone();

                    button.connect_clicked(move |_button| {
                        // TODO: this needs refactoring to call open from the controller
                        let file_name = file_name.clone();
                        let command = command.clone();

                        spawn(async move { open_program(&file_name, &command).await });

                        sub_menu.hide();
                        tx.send_spawn(ModuleUpdateEvent::ClosePopup);
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

pub fn add_entries(
    entry: &MenuEntry,
    button: Button,
    sub_menu: Option<&gtk::Box>,
    main_menu: &gtk::Box,
    container: &gtk::Box,
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
            scrolled.add(sub_menu);
            container.add(&scrolled);

            let sub_menu1 = scrolled.clone();
            let sub_menu_popup_container = sub_menu.clone();

            button.connect_clicked(move |button| {
                container1.children().iter().skip(1).for_each(|sub_menu| {
                    if sub_menu.get_visible() {
                        sub_menu.hide();
                    }
                });

                button
                    .parent()
                    .expect("button parent should exist")
                    .downcast::<gtk::Box>()
                    .expect("button container should be gtk::Box")
                    .children()
                    .iter()
                    .for_each(|child| child.remove_class("open"));

                sub_menu1.show_all();
                button.add_class("open");

                // Reset scroll to top.
                if let Some(w) = sub_menu_popup_container.children().first() {
                    w.set_has_focus(true)
                }
            });
        } else {
            container.add(sub_menu);
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
