use lazy_static::lazy_static;
use mpd_client::client::{CommandError, Connection, ConnectionEvent, Subsystem};
use mpd_client::commands::Command;
use mpd_client::protocol::MpdProtocolError;
use mpd_client::responses::Status;
use mpd_client::Client;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpStream, UnixStream};
use tokio::spawn;
use tokio::sync::broadcast::{channel, error::SendError, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::debug;

lazy_static! {
    static ref CONNECTIONS: Arc<Mutex<HashMap<String, Arc<MpdClient>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub struct MpdClient {
    client: Client,
    tx: Sender<()>,
    _rx: Receiver<()>,
}

#[derive(Debug)]
pub enum MpdConnectionError {
    MaxRetries,
    ProtocolError(MpdProtocolError),
}

impl Display for MpdConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxRetries => write!(f, "Reached max retries"),
            Self::ProtocolError(e) => write!(f, "{:?}", e),
        }
    }
}

impl std::error::Error for MpdConnectionError {}

impl MpdClient {
    async fn new(host: &str) -> Result<Self, MpdConnectionError> {
        debug!("Creating new MPD connection to {}", host);

        let (client, mut state_changes) =
            wait_for_connection(host, Duration::from_secs(5), None).await?;

        let (tx, rx) = channel(16);
        let tx2 = tx.clone();

        spawn(async move {
            while let Some(change) = state_changes.next().await {
                debug!("Received state change: {:?}", change);

                if let ConnectionEvent::SubsystemChange(Subsystem::Player | Subsystem::Queue) =
                    change
                {
                    tx2.send(())?;
                }
            }

            Ok::<(), SendError<()>>(())
        });

        Ok(Self {
            client,
            tx,
            _rx: rx,
        })
    }

    pub fn subscribe(&self) -> Receiver<()> {
        self.tx.subscribe()
    }

    pub async fn command<C: Command>(&self, command: C) -> Result<C::Response, CommandError> {
        self.client.command(command).await
    }
}

pub async fn get_client(host: &str) -> Result<Arc<MpdClient>, MpdConnectionError> {
    let mut connections = CONNECTIONS.lock().await;
    match connections.get(host) {
        None => {
            let client = MpdClient::new(host).await?;
            let client = Arc::new(client);
            connections.insert(host.to_string(), Arc::clone(&client));
            Ok(client)
        }
        Some(client) => Ok(Arc::clone(client)),
    }
}

async fn wait_for_connection(
    host: &str,
    interval: Duration,
    max_retries: Option<usize>,
) -> Result<Connection, MpdConnectionError> {
    let mut retries = 0;
    let max_retries = max_retries.unwrap_or(usize::MAX);

    loop {
        if retries == max_retries {
            break Err(MpdConnectionError::MaxRetries);
        }

        retries += 1;

        match try_get_mpd_conn(host).await {
            Ok(conn) => break Ok(conn),
            Err(err) => {
                if retries == max_retries {
                    break Err(MpdConnectionError::ProtocolError(err));
                }
            }
        }

        sleep(interval).await;
    }
}

/// Cycles through each MPD host and
/// returns the first one which connects,
/// or none if there are none
async fn try_get_mpd_conn(host: &str) -> Result<Connection, MpdProtocolError> {
    if is_unix_socket(host) {
        connect_unix(host).await
    } else {
        connect_tcp(host).await
    }
}

fn is_unix_socket(host: &str) -> bool {
    let path = PathBuf::from(host);
    path.exists()
        && match path.metadata() {
            Ok(metadata) => metadata.file_type().is_socket(),
            Err(_) => false,
        }
}

async fn connect_unix(host: &str) -> Result<Connection, MpdProtocolError> {
    let connection = UnixStream::connect(host).await?;
    Client::connect(connection).await
}

async fn connect_tcp(host: &str) -> Result<Connection, MpdProtocolError> {
    let connection = TcpStream::connect(host).await?;
    Client::connect(connection).await
}

/// Gets the duration of the current song
pub fn get_duration(status: &Status) -> Option<u64> {
    status.duration.map(|duration| duration.as_secs())
}

/// Gets the elapsed time of the current song
pub fn get_elapsed(status: &Status) -> Option<u64> {
    status.elapsed.map(|duration| duration.as_secs())
}
