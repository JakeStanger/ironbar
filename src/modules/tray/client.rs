use async_once::AsyncOnce;
use lazy_static::lazy_static;
use stray::message::{NotifierItemCommand, NotifierItemMessage};
use stray::StatusNotifierWatcher;
use tokio::spawn;
use tokio::sync::{broadcast, mpsc};
use tracing::debug;

pub struct TrayEventReceiver {
    tx: mpsc::Sender<NotifierItemCommand>,
    b_tx: broadcast::Sender<NotifierItemMessage>,
    _b_rx: broadcast::Receiver<NotifierItemMessage>,
}

impl TrayEventReceiver {
    async fn new() -> stray::error::Result<Self> {
        let (tx, rx) = mpsc::channel(16);
        let (b_tx, b_rx) = broadcast::channel(16);

        let tray = StatusNotifierWatcher::new(rx).await?;
        let mut host = tray.create_notifier_host("ironbar").await?;

        let b_tx2 = b_tx.clone();
        spawn(async move {
            while let Ok(message) = host.recv().await {
                b_tx2.send(message)?;
            }

            Ok::<(), broadcast::error::SendError<NotifierItemMessage>>(())
        });

        Ok(Self {
            tx,
            b_tx,
            _b_rx: b_rx,
        })
    }

    pub fn subscribe(
        &self,
    ) -> (
        mpsc::Sender<NotifierItemCommand>,
        broadcast::Receiver<NotifierItemMessage>,
    ) {
        (self.tx.clone(), self.b_tx.subscribe())
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

            if tray.is_ok() || retries == MAX_RETRIES {
                break tray;
            }

            debug!("Failed to create StatusNotifierWatcher (attempt {retries})");
        };

        value.expect("Failed to create StatusNotifierWatcher")
    });
}

pub async fn get_tray_event_client() -> &'static TrayEventReceiver {
    CLIENT.get().await
}
