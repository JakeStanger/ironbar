use crate::{lock, send};
use async_once::AsyncOnce;
use color_eyre::Report;
use lazy_static::lazy_static;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use stray::message::menu::TrayMenu;
use stray::message::tray::StatusNotifierItem;
use stray::message::{NotifierItemCommand, NotifierItemMessage};
use stray::StatusNotifierWatcher;
use tokio::spawn;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, trace};
use crate::unique_id::get_unique_usize;

type Tray = BTreeMap<String, (Box<StatusNotifierItem>, Option<TrayMenu>)>;

pub struct TrayEventReceiver {
    tx: mpsc::Sender<NotifierItemCommand>,
    b_tx: broadcast::Sender<NotifierItemMessage>,
    _b_rx: broadcast::Receiver<NotifierItemMessage>,

    tray: Arc<Mutex<Tray>>,
}

impl TrayEventReceiver {
    async fn new() -> stray::error::Result<Self> {
        let id = format!("ironbar-{}", get_unique_usize());

        let (tx, rx) = mpsc::channel(16);
        let (b_tx, b_rx) = broadcast::channel(16);

        let tray = StatusNotifierWatcher::new(rx).await?;
        let mut host = tray.create_notifier_host(&id).await?;

        let tray = Arc::new(Mutex::new(BTreeMap::new()));

        {
            let b_tx = b_tx.clone();
            let tray = tray.clone();

            spawn(async move {
                while let Ok(message) = host.recv().await {
                    trace!("Received message: {message:?} ");

                    send!(b_tx, message.clone());
                    let mut tray = lock!(tray);
                    match message {
                        NotifierItemMessage::Update {
                            address,
                            item,
                            menu,
                        } => {
                            debug!("Adding item with address '{address}'");
                            tray.insert(address, (item, menu));
                        }
                        NotifierItemMessage::Remove { address } => {
                            debug!("Removing item with address '{address}'");
                            tray.remove(&address);
                        }
                    }
                }

                Ok::<(), broadcast::error::SendError<NotifierItemMessage>>(())
            });
        }

        Ok(Self {
            tx,
            b_tx,
            _b_rx: b_rx,
            tray,
        })
    }

    pub fn subscribe(
        &self,
    ) -> (
        mpsc::Sender<NotifierItemCommand>,
        broadcast::Receiver<NotifierItemMessage>,
    ) {
        let tx = self.tx.clone();
        let b_rx = self.b_tx.subscribe();

        let tray = lock!(self.tray).clone();
        for (address, (item, menu)) in tray {
            let update = NotifierItemMessage::Update {
                address,
                item,
                menu,
            };
            send!(self.b_tx, update);
        }

        (tx, b_rx)
    }
}

lazy_static! {
    static ref CLIENT: AsyncOnce<TrayEventReceiver> = AsyncOnce::new(async {
        const MAX_RETRIES: i32 = 10;

        // sometimes this can fail
        let mut retries = 0;

        let value = loop {
            retries += 1;

            let tray = TrayEventReceiver::new().await;

            match tray {
                Ok(tray) => break Some(tray),
                Err(err) => error!("{:?}", Report::new(err).wrap_err(format!("Failed to create StatusNotifierWatcher (attempt {retries})")))
            }

            if retries == MAX_RETRIES {
                break None;
            }
        };

        value.expect("Failed to create StatusNotifierWatcher")
    });
}

pub async fn get_tray_event_client() -> &'static TrayEventReceiver {
    CLIENT.get().await
}
