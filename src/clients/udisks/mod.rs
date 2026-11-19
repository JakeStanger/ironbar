mod dbus;

use self::dbus::{BlockProxy, DriveProxy, FileManager1Proxy, FilesystemProxy};
use crate::clients::ClientResult;
use crate::{register_fallible_client, spawn};
use futures_lite::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, warn};
use zbus::zvariant::OwnedObjectPath;
use zbus::zvariant::OwnedValue;
use zbus::zvariant::Value;
use zbus::{Connection, Result, fdo::ObjectManagerProxy, names::InterfaceName};

#[derive(Debug, Clone)]
pub struct Drive {
    pub id: String,
    pub model: String,
    pub size: u64,
    pub removable: bool,
    pub object_path: String,
}

#[derive(Debug, Clone)]
pub enum UdisksEvent {
    Added(Drive),
    Removed(Drive),
}

fn parse_drive(object_path: &str, props: &HashMap<String, OwnedValue>) -> Drive {
    let prop_str = |props: &HashMap<String, OwnedValue>, key: &str| -> String {
        props
            .get(key)
            .and_then(|v| <&str>::try_from(&**v).ok())
            .unwrap_or_default()
            .to_string()
    };
    Drive {
        id: prop_str(props, "Id"),
        object_path: object_path.to_string(),
        model: prop_str(props, "Model"),
        size: props
            .get("Size")
            .and_then(|v| u64::try_from(&**v).ok())
            .unwrap_or_default(),
        removable: props
            .get("Removable")
            .and_then(|v| bool::try_from(&**v).ok())
            .unwrap_or_default(),
    }
}

fn path_to_uri(path: &[u8]) -> String {
    let path = path.strip_suffix(&[0]).unwrap_or(path);
    let mut uri = String::from("file://");

    for &byte in path {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                uri.push(byte as char)
            }
            _ => uri.push_str(&format!("%{byte:02X}")),
        }
    }

    uri
}

const BLOCK_IFACE: &str = "org.freedesktop.UDisks2.Block";
const FS_IFACE: &str = "org.freedesktop.UDisks2.Filesystem";
const DRIVE_IFACE: &str = "org.freedesktop.UDisks2.Drive";

#[derive(Debug)]
pub struct Client {
    drives: Arc<Mutex<HashMap<String, Drive>>>,
    tx: broadcast::Sender<UdisksEvent>,
    _rx: broadcast::Receiver<UdisksEvent>,
    dbus: Connection,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let (tx, rx) = broadcast::channel(32);
        let drives: Arc<Mutex<HashMap<String, Drive>>> = Arc::new(Mutex::new(HashMap::new()));
        let dbus = Box::pin(zbus::Connection::system()).await?;

        let object_manager_proxy =
            ObjectManagerProxy::new(&dbus, "org.freedesktop.UDisks2", "/org/freedesktop/UDisks2")
                .await?;

        let interface_name = InterfaceName::from_static_str(DRIVE_IFACE)
            .expect("failed to create zbus InterfaceName");

        let mut added_stream = object_manager_proxy.receive_interfaces_added().await?;
        let mut removed_stream = object_manager_proxy.receive_interfaces_removed().await?;

        for (path, ifaces) in object_manager_proxy.get_managed_objects().await? {
            for (key, value) in ifaces {
                if key != DRIVE_IFACE {
                    continue;
                }

                let drive = parse_drive(path.as_str(), &value);

                if drive.removable {
                    drives.lock().expect("lock").insert(path.to_string(), drive);
                }
            }
        }

        spawn({
            let tx = tx.clone();
            let drives = drives.clone();

            async move {
                loop {
                    tokio::select! {

                    Some(ev) = added_stream.next() => {
                        let args = ev.args().expect("Invalid signal arguments");

                        for (key, value) in args.interfaces_and_properties {
                            if key != interface_name {
                                continue;
                            }

                            let props = value
                            .iter()
                            .filter_map(|(k, v)| Some((k.to_string(), v.try_to_owned().ok()?)))
                            .collect();

                            let drive = parse_drive(args.object_path.as_str(), &props);


                            let is_new = drives
                                .lock().expect("lock")
                                .insert(drive.object_path.clone(), drive.clone())
                                .is_none();

                            if is_new {
                                 let _ = tx.send(UdisksEvent::Added(drive));
                            }
                        }
                    }
                        Some(ev) = removed_stream.next() => {
                            let Ok(args) = ev.args() else { continue };
                            let path = args.object_path().to_string();

                            if let Some(drive)=  drives.lock().expect("lock").remove(&path) {
                                let _ = tx.send(UdisksEvent::Removed(drive));
                            }
                        }
                        else => break
                    }
                }
            }
        });

        Ok(Arc::new(Self {
            drives,
            tx,
            _rx: rx,
            dbus,
        }))
    }

    /// Returns a new event receiver.
    /// No async work may happen between subscribing and returning.
    pub fn subscribe(&self) -> broadcast::Receiver<UdisksEvent> {
        self.tx.subscribe()
    }

    /// Returns the current state, used to initialise newly created modules.
    pub fn drives(&self) -> Vec<Drive> {
        self.drives
            .lock()
            .expect("lock")
            .values()
            .cloned()
            .collect()
    }

    /// Object paths of every filesystem block device that belongs to the drive.
    async fn filesystems_of(&self, object_path: &str) -> Result<Vec<OwnedObjectPath>> {
        let object_manager_proxy = ObjectManagerProxy::new(
            &self.dbus,
            "org.freedesktop.UDisks2",
            "/org/freedesktop/UDisks2",
        )
        .await?;

        let mut found = Vec::new();

        for (path, ifaces) in object_manager_proxy.get_managed_objects().await? {
            let has = |name: &str| ifaces.keys().any(|k| k.as_str() == name);

            if !(has(BLOCK_IFACE) && has(FS_IFACE)) {
                continue;
            }

            let block = BlockProxy::builder(&self.dbus)
                .path(path.as_str())?
                .build()
                .await?;

            if block.drive().await?.as_str() == object_path {
                found.push(path);
            }
        }

        Ok(found)
    }

    /// Opens the drive in the default file manager, mounting it first if needed.
    pub async fn open_folder(
        &self,
        object_path: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let yes = Value::from(true);
        let options = || HashMap::from([("auth.no_user_interaction", &yes)]);

        let mut uris = Vec::new();

        for path in self.filesystems_of(object_path).await? {
            let fs = FilesystemProxy::builder(&self.dbus)
                .path(path.as_str())?
                .build()
                .await?;

            let mount_points = fs.mount_points().await?;

            let uri = match mount_points.first() {
                Some(bytes) => path_to_uri(bytes),
                None => {
                    debug!("mounting {path}");
                    let mount_path = fs.mount(options()).await?;

                    path_to_uri(mount_path.as_bytes())
                }
            };

            uris.push(uri);
        }

        if uris.is_empty() {
            debug!("drive has no filesystem to open");
        }

        // FileManager1 is on the session bus, so it needs its own connection
        let session = Box::pin(Connection::session()).await?;
        let fm = FileManager1Proxy::new(&session).await?;

        let uri_refs: Vec<&str> = uris.iter().map(String::as_str).collect();

        debug!("{:?}", uri_refs);
        fm.show_folders(&uri_refs, "ironbar").await?;

        Ok(())
    }

    pub async fn eject(&self, object_path: &str) -> Result<()> {
        let object_manager_proxy = ObjectManagerProxy::new(
            &self.dbus,
            "org.freedesktop.UDisks2",
            "/org/freedesktop/UDisks2",
        )
        .await?;

        // 1. Unmount every mounted filesystem that belongs to this drive
        for (path, ifaces) in object_manager_proxy.get_managed_objects().await? {
            let is_filesystem_block = ifaces.keys().any(|k| k.as_str() == BLOCK_IFACE)
                && ifaces.keys().any(|k| k.as_str() == FS_IFACE);

            if !is_filesystem_block {
                continue;
            }

            let block = BlockProxy::builder(&self.dbus)
                .path(path.as_str())?
                .build()
                .await?;

            if block.drive().await?.as_str() != object_path {
                continue;
            }

            let fs = FilesystemProxy::builder(&self.dbus)
                .path(path.as_str())?
                .build()
                .await?;

            // Unmount fails with "not mounted" otherwise
            if fs.mount_points().await?.is_empty() {
                continue;
            }

            debug!("unmounting {path}");

            fs.unmount(HashMap::from([(
                "auth.no_user_interaction",
                &Value::from(true),
            )]))
            .await?;
        }

        // 2. Eject the drive, then power it off if the hardware supports it
        let drive = DriveProxy::builder(&self.dbus)
            .path(object_path)?
            .build()
            .await?;

        if drive.ejectable().await? {
            debug!("ejecting {object_path}");
            drive
                .eject(HashMap::from([(
                    "auth.no_user_interaction",
                    &Value::from(true),
                )]))
                .await?;
        }

        if drive.can_power_off().await? {
            // The drive object may disappear right after Eject, so a failure
            // here is not fatal
            if let Err(err) = drive
                .power_off(HashMap::from([(
                    "auth.no_user_interaction",
                    &Value::from(true),
                )]))
                .await
            {
                warn!("power off failed: {err}");
            }
        }

        Ok(())
    }
}

register_fallible_client!(Client, udisks);
