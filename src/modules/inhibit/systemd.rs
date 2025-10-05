use chrono::{DateTime, Utc};
use color_eyre::Result;
use std::time::Duration;
use zbus::{
    Connection,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

const UNIT_NAME: &str = "ironbar-inhibit.service";
const CMD: &str = "/usr/bin/systemd-inhibit";

fn exec_tuple(duration: Duration) -> (String, Vec<String>, bool) {
    let sleep_arg = if duration == Duration::MAX {
        "infinity"
    } else {
        &duration.as_secs().to_string()
    };
    (
        CMD.to_string(),
        vec![
            CMD.to_string(),
            "--what=sleep:idle".into(),
            "--who=ironbar".into(),
            "--why=User requested".into(),
            "sleep".into(),
            sleep_arg.to_string(),
        ],
        false,
    )
}

fn read_env(env: &[String], key: &str) -> Option<u64> {
    env.iter()
        .find_map(|s| s.strip_prefix(&format!("{key}="))?.parse().ok())
}

#[zbus::proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait SystemdManager {
    fn start_transient_unit(
        &self,
        name: &str,
        mode: &str,
        properties: Vec<(&str, OwnedValue)>,
        aux: Vec<(&str, Vec<(&str, OwnedValue)>)>,
    ) -> zbus::Result<OwnedObjectPath>;
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<()>;
    fn get_unit(&self, name: &str) -> zbus::Result<OwnedObjectPath>;
}

#[zbus::proxy(
    interface = "org.freedesktop.DBus.Properties",
    default_service = "org.freedesktop.systemd1"
)]
trait Properties {
    fn get(&self, interface: &str, property: &str) -> zbus::Result<OwnedValue>;
}

pub struct SystemdBackend {
    conn: Connection,
    unit_path: Option<OwnedObjectPath>,
    pub(super) expiry: Option<DateTime<Utc>>,
}

impl SystemdBackend {
    pub async fn new() -> Result<Self> {
        let conn = Connection::session().await?;
        let proxy = SystemdManagerProxy::new(&conn).await?;
        let unit_path = proxy.get_unit(UNIT_NAME).await.ok();

        // Restore expiry from existing unit's Environment to survive ironbar restarts
        let expiry = async {
            let path = unit_path.as_ref()?;
            let props = PropertiesProxy::builder(&conn)
                .path(path)
                .ok()?
                .build()
                .await
                .ok()?;
            let env_value = props
                .get("org.freedesktop.systemd1.Service", "Environment")
                .await
                .ok()?;
            let env = <Vec<String>>::try_from(env_value).ok()?;
            read_env(&env, "INHIBIT_EXPIRY").and_then(|ts| {
                let last_timestamp = ts as i64;
                // i64::MAX represents infinite expiry in the environment variable
                if last_timestamp == i64::MAX {
                    Some(DateTime::<Utc>::MAX_UTC)
                } else {
                    DateTime::from_timestamp(last_timestamp, 0)
                }
            })
        }
        .await;

        Ok(Self {
            conn,
            unit_path,
            expiry,
        })
    }

    pub async fn start(&mut self, duration: Duration) -> Result<()> {
        self.stop().await?;

        self.expiry = super::calculate_expiry(duration);

        let expiry_timestamp = match self.expiry {
            Some(dt) if dt == DateTime::<Utc>::MAX_UTC => i64::MAX,
            Some(dt) => dt.timestamp(),
            None => unreachable!(),
        };

        self.unit_path = Some(
            SystemdManagerProxy::new(&self.conn)
                .await?
                .start_transient_unit(
                    UNIT_NAME,
                    "replace",
                    vec![
                        ("Description", Value::new("Ironbar Inhibit").try_to_owned()?),
                        ("Type", Value::new("simple").try_to_owned()?),
                        (
                            "ExecStart",
                            Value::new(vec![exec_tuple(duration)]).try_to_owned()?,
                        ),
                        (
                            "Environment",
                            Value::new(vec![format!("INHIBIT_EXPIRY={}", expiry_timestamp)])
                                .try_to_owned()?,
                        ),
                    ],
                    vec![],
                )
                .await?,
        );
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if self.unit_path.is_some() {
            let proxy = SystemdManagerProxy::new(&self.conn).await?;
            proxy.stop_unit(UNIT_NAME, "replace").await.ok();
            self.unit_path = None;
        }
        self.expiry = None;
        Ok(())
    }
}
