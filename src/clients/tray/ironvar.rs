use std::sync::Arc;

use system_tray::item::StatusNotifierItem;

use crate::lock;

const NAME: &str = "name";
const ID: &str = "id";

struct App {
    status: StatusNotifierItem,
}

#[cfg(feature = "ipc")]
impl crate::ironvar::Namespace for App {
    fn get(&self, key: &str) -> Option<String> {
        match key {
            ID => Some(self.status.id.clone()),
            NAME => self.status.title.clone(),
            _ => None,
        }
    }

    fn list(&self) -> Vec<String> {
        vec![ID.to_owned(), NAME.to_owned()]
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _: &str) -> Option<crate::ironvar::NamespaceTrait> {
        None
    }
}

#[cfg(feature = "ipc")]
impl crate::ironvar::Namespace for super::Client {
    fn get(&self, _: &str) -> Option<String> {
        None
    }

    fn list(&self) -> Vec<String> {
        Vec::new()
    }

    fn namespaces(&self) -> Vec<String> {
        let items = self.client.items();
        lock!(items)
            .values()
            .map(|(item, _)| item.id.clone())
            .collect()
    }

    fn get_namespace(&self, key: &str) -> Option<crate::ironvar::NamespaceTrait> {
        let items = self.client.items();
        let status = lock!(items)
            .iter()
            .find_map(|(_, (status, _))| {
                if status.id.as_str() == key {
                    Some(status)
                } else {
                    None
                }
            })?
            .clone();

        Some(Arc::new(App { status }))
    }
}
