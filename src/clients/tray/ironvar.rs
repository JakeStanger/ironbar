use crate::lock;

const TRAY_ICONS_NAMES: &str = "icon_names";

#[cfg(feature = "ipc")]
impl crate::ironvar::Namespace for super::Client {
    fn get(&self, key: &str) -> Option<String> {
        match key {
            TRAY_ICONS_NAMES => {
                let items = self.client.items();
                let ids: Vec<String> = lock!(items)
                    .values()
                    .map(|(item, _)| item.id.clone())
                    .collect();

                Some(ids.join(", "))
            }
            _ => None,
        }
    }

    fn list(&self) -> Vec<String> {
        tracing::info!("Test");
        vec![TRAY_ICONS_NAMES.to_owned()]
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _key: &str) -> Option<crate::ironvar::NamespaceTrait> {
        None
    }
}
