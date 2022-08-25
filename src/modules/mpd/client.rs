use lazy_static::lazy_static;
use mpd_client::commands::responses::Status;
use mpd_client::raw::MpdProtocolError;
use mpd_client::{Client, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpStream, UnixStream};
use tokio::sync::Mutex;
use tokio::time::sleep;

lazy_static! {
    static ref CLIENTS: Arc<Mutex<HashMap<String, Arc<Client>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub async fn get_connection(host: &str) -> Option<Arc<Client>> {
    let mut clients = CLIENTS.lock().await;

    match clients.get(host) {
        Some(client) => Some(Arc::clone(client)),
        None => {
            let client = wait_for_connection(host, Duration::from_secs(5), None).await?;
            let client = Arc::new(client);
            clients.insert(host.to_string(), Arc::clone(&client));
            Some(client)
        }
    }
}

async fn wait_for_connection(
    host: &str,
    interval: Duration,
    max_retries: Option<usize>,
) -> Option<Client> {
    let mut retries = 0;
    let max_retries = max_retries.unwrap_or(usize::MAX);

    loop {
        if retries == max_retries {
            break None;
        }

        if let Some(conn) = try_get_mpd_conn(host).await {
            break Some(conn.0);
        }

        retries += 1;
        sleep(interval).await;
    }
}

/// Cycles through each MPD host and
/// returns the first one which connects,
/// or none if there are none
async fn try_get_mpd_conn(host: &str) -> Option<Connection> {
    let connection = if is_unix_socket(host) {
        connect_unix(host).await
    } else {
        connect_tcp(host).await
    };

    connection.ok()
}

fn is_unix_socket(host: &str) -> bool {
    PathBuf::from(host).is_file()
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
