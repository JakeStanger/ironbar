use crate::modules::launcher::item::LauncherWindow;
use crate::modules::launcher::FocusEvent;
pub use crate::popup::Popup;
use gtk::prelude::*;
use gtk::Button;
use tokio::sync::mpsc;

impl Popup {
    pub fn set_windows(&self, windows: &[LauncherWindow], tx: &mpsc::Sender<FocusEvent>) {
        // clear
        for child in self.container.children() {
            self.container.remove(&child);
        }

        for window in windows {
            let mut button_builder = Button::builder().height_request(40);

            if let Some(name) = &window.name {
                button_builder = button_builder.label(name);
            }

            let button = button_builder.build();

            let con_id = window.con_id;
            let window = self.window.clone();
            let tx = tx.clone();
            button.connect_clicked(move |_| {
                tx.try_send(FocusEvent::ConId(con_id)).unwrap();
                window.hide();
            });

            self.container.add(&button);
        }
    }
}
