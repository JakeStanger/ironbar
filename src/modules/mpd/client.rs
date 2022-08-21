use mpd_client::commands::responses::Status;
use mpd_client::raw::MpdProtocolError;
use mpd_client::{Client, Connection};
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::{TcpStream, UnixStream};
use tokio::spawn;
use tokio::time::sleep;

pub async fn wait_for_connection(
    hosts: Vec<String>,
    interval: Duration,
    max_retries: Option<usize>,
) -> Option<Client> {
    let mut retries = 0;

    spawn(async move {
        let max_retries = max_retries.unwrap_or(usize::MAX);
        loop {
            if retries == max_retries {
                break None;
            }

            if let Some(conn) = try_get_mpd_conn(&hosts).await {
                break Some(conn.0);
            }

            retries += 1;
            sleep(interval).await;
        }
    })
    .await
    .expect("Error occurred while handling tasks")
}

/// Cycles through each MPD host and
/// returns the first one which connects,
/// or none if there are none
async fn try_get_mpd_conn(hosts: &[String]) -> Option<Connection> {
    for host in hosts {
        let connection = if is_unix_socket(host) {
            connect_unix(host).await
        } else {
            connect_tcp(host).await
        };

        if let Ok(connection) = connection {
            return Some(connection);
        }
    }

    None
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
