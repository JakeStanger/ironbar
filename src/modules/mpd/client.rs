use mpd_client::commands::responses::Status;
use mpd_client::raw::MpdProtocolError;
use mpd_client::{Client, Connection};
use std::path::PathBuf;
use tokio::net::{TcpStream, UnixStream};

fn is_unix_socket(host: &String) -> bool {
    PathBuf::from(host).is_file()
}

pub async fn get_connection(host: &String) -> Result<Connection, MpdProtocolError> {
    if is_unix_socket(host) {
        connect_unix(host).await
    } else {
        connect_tcp(host).await
    }
}

async fn connect_unix(host: &String) -> Result<Connection, MpdProtocolError> {
    let connection = UnixStream::connect(host)
        .await
        .unwrap_or_else(|_| panic!("Error connecting to unix socket: {}", host));

    Client::connect(connection).await
}

async fn connect_tcp(host: &String) -> Result<Connection, MpdProtocolError> {
    let connection = TcpStream::connect(host)
        .await
        .unwrap_or_else(|_| panic!("Error connecting to unix socket: {}", host));

    Client::connect(connection).await
}

// /// Gets MPD server status.
// /// Panics on error.
// pub async fn get_status(client: &Client) -> Status {
//     client
//         .command(commands::Status)
//         .await
//         .expect("Failed to get MPD server status")
// }

/// Gets the duration of the current song
pub fn get_duration(status: &Status) -> u64 {
    status
        .duration
        .expect("Failed to get duration from MPD status")
        .as_secs()
}

/// Gets the elapsed time of the current song
pub fn get_elapsed(status: &Status) -> u64 {
    status
        .elapsed
        .expect("Failed to get elapsed time from MPD status")
        .as_secs()
}
