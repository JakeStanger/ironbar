use crate::channels::SyncSenderExt;
use crate::clients::ClientResult;
use crate::{arc_mut, lock, register_fallible_client, spawn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use system_tray::client::{ActivateRequest, Client as TrayClient, Event, UpdateEvent};
use system_tray::data::BaseMap;
use system_tray::menu::TrayMenu;
use tokio::sync::broadcast;

#[derive(Debug)]
struct MenuCache {
    path: String,
    menu: Option<TrayMenu>,
}

#[derive(Debug)]
pub struct Client {
    client: TrayClient,
    tx: broadcast::Sender<Event>,
    _rx: broadcast::Receiver<Event>,

    menus: Arc<Mutex<HashMap<Box<str>, MenuCache>>>,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let client = TrayClient::new().await?;

        let (tx, rx) = broadcast::channel(16);
        let menus = arc_mut!(HashMap::new());

        {
            let tx = tx.clone();
            let mut client_rx = client.subscribe();
            let menus = menus.clone();

            // The client will send the Menu & MenuConnect events
            // to the first module that connects to it,
            // which means subsequent modules do not receive this information.
            //
            // Some info is re-fetched when they request the *items*
            // but this is not enough to fully hydrate the menus
            // To work around this, we cache these events to re-send to any future modules.
            spawn(async move {
                while let Ok(event) = client_rx.recv().await {
                    match &event {
                        Event::Update(address, UpdateEvent::MenuConnect(path)) => {
                            lock!(menus).insert(
                                address.clone().into_boxed_str(),
                                MenuCache {
                                    path: path.to_string(),
                                    menu: None,
                                },
                            );
                        }
                        Event::Update(address, UpdateEvent::Menu(menu)) => {
                            if let Some(entry) =
                                lock!(menus).get_mut(&address.clone().into_boxed_str())
                            {
                                entry.menu = Some(menu.clone());
                            }
                        }
                        _ => {}
                    }

                    tx.send_expect(event);
                }
            });
        }

        Ok(Arc::new(Self {
            client,
            tx,
            _rx: rx,
            menus,
        }))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        let rx = self.tx.subscribe();

        for (address, menu) in lock!(self.menus).iter() {
            self.tx.send_expect(Event::Update(
                address.to_string(),
                UpdateEvent::MenuConnect(menu.path.to_string()),
            ));

            if let Some(menu) = &menu.menu {
                self.tx.send_expect(Event::Update(
                    address.to_string(),
                    UpdateEvent::Menu(menu.clone()),
                ))
            }
        }

        rx
    }

    pub fn items(&self) -> Arc<Mutex<BaseMap>> {
        self.client.items()
    }

    pub async fn activate(&self, req: ActivateRequest) -> system_tray::error::Result<()> {
        self.client.activate(req).await
    }
}

register_fallible_client!(Client, tray);
