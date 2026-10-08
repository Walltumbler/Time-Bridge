use crate::model::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use std::path::Path;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: i64,
    pub item_id: String,
    pub event: String,
    pub occurred_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub origin: Option<String>,
    pub external_id: Option<String>,
    pub reason: String,
    pub title: String,
    pub notified: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub sound: bool,
    pub notifications: bool,
    #[serde(default)]
    pub alarm_volume_override: bool,
    #[serde(default = "default_alarm_volume")]
    pub alarm_volume_percent: u8,
    #[serde(default)]
    pub alarm_output_device: Option<String>,
    #[serde(default)]
    pub mute_other_apps: bool,
}
fn default_alarm_volume() -> u8 { 75 }
impl Default for Settings {
    fn default() -> Self {
        Self {
            sound: true,
            notifications: true,
            alarm_volume_override: false,
            alarm_volume_percent: default_alarm_volume(),
            alarm_output_device: None,
            mute_other_apps: false,
        }
    }
}

pub struct Store {
    pub(crate) conn: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS items(id TEXT PRIMARY KEY, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY AUTOINCREMENT, item_id TEXT NOT NULL, payload TEXT NOT NULL, notified INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS site_permissions(origin TEXT PRIMARY KEY, name TEXT NOT NULL, credential_hash TEXT NOT NULL, approved_at TEXT NOT NULL, revoked INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS event_acks(origin TEXT NOT NULL, event_id INTEGER NOT NULL, PRIMARY KEY(origin,event_id));
            PRAGMA user_version=2;").map_err(err)?;
        // SQLite interprets bare `false` as integer 0; wrapping the JSON text keeps
        // serde's bool field a real JSON boolean during migration from old schemas.
        conn.execute_batch("UPDATE items SET payload=json_set(payload,'$.alertSilenced',json(CASE WHEN json_extract(payload,'$.alertSilenced')!=0 THEN 'true' ELSE 'false' END)) WHERE json_type(payload,'$.alertSilenced') IS NULL OR json_type(payload,'$.alertSilenced') NOT IN ('true','false');").map_err(err)?;
        Ok(Self { conn })
    }
    pub fn list(&self) -> Result<Vec<Item>> {
        read_items(&self.conn)
    }
    pub fn events(&self) -> Result<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id,payload,notified FROM events ORDER BY id DESC LIMIT 500")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            })
            .map_err(err)?;
        rows.map(|r| {
            let (id, payload, notified) = r.map_err(err)?;
            let mut e: Event = serde_json::from_str(&payload).map_err(err)?;
            e.id = id;
            e.notified = notified;
            Ok(e)
        })
        .collect()
    }
    pub fn pending_notifications(&self) -> Result<Vec<Event>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id,payload FROM events WHERE notified=0 ORDER BY id LIMIT 50")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(err)?;
        rows.map(|r| {
            let (id, payload) = r.map_err(err)?;
            let mut e: Event = serde_json::from_str(&payload).map_err(err)?;
            e.id = id;
            e.notified = false;
            Ok(e)
        })
        .collect()
    }
    pub fn create(&mut self, input: CreateItem, now: DateTime<Utc>) -> Result<Item> {
        self.create_as(input, now, "user")
    }
    pub fn create_as(&mut self, input: CreateItem, now: DateTime<Utc>, reason: &str) -> Result<Item> {
        let item = Item::create(input, now)?;
        let tx = self.conn.transaction().map_err(err)?;
        save(&tx, &item)?;
        record(&tx, &item, "item.created", now, now, reason, false)?;
        tx.commit().map_err(err)?;
        Ok(item)
    }
    pub fn action(&mut self, id: &str, action: &str, now: DateTime<Utc>) -> Result<()> {
        self.action_as(id, action, now, "user")
    }
    pub fn action_as(&mut self, id: &str, action: &str, now: DateTime<Utc>, reason: &str) -> Result<()> {
        self.tick(now)?;
        let mut item = self
            .list()?
            .into_iter()
            .find(|i| i.id == id)
            .ok_or("Item not found")?;
        let event = item.action(action, now)?;
        let tx = self.conn.transaction().map_err(err)?;
        save(&tx, &item)?;
        record(&tx, &item, event, now, now, reason, false)?;
        if matches!(action, "dismiss" | "cancel" | "done" | "silence") {
            tx.execute("UPDATE events SET notified=1 WHERE item_id=?1", [id])
                .map_err(err)?;
        }
        tx.commit().map_err(err)
    }
    pub fn rename(&mut self, id: &str, title: &str, now: DateTime<Utc>) -> Result<()> {
        if title.trim().is_empty() || title.len() > 200 {
            return Err("Title must be 1–200 bytes".into());
        }
        let mut item = self
            .list()?
            .into_iter()
            .find(|i| i.id == id)
            .ok_or("Item not found")?;
        item.title = title.trim().into();
        let tx = self.conn.transaction().map_err(err)?;
        save(&tx, &item)?;
        record(&tx, &item, "item.updated", now, now, "user", false)?;
        tx.commit().map_err(err)
    }
    pub fn snooze(&mut self, id: &str, now: DateTime<Utc>) -> Result<Item> {
        let mut original = self
            .list()?
            .into_iter()
            .find(|i| i.id == id)
            .ok_or("Item not found")?;
        if original.alert.is_none() {
            return Err("Only a ringing item can be snoozed".into());
        }
        let snooze = Item::create(
            CreateItem {
                icon: original.icon.clone(),
                title: format!("{}", original.title),
                kind: Kind::Countdown,
                duration_seconds: Some(300),
                minimum_seconds: None,
                maximum_seconds: None,
                fire_at: None,
                timezone: None,
                schedule: None,
                source: original.source.clone(),
                external_id: original.external_id.clone(),
                return_url: original.return_url.clone(),
            },
            now,
        )?;
        original.action("dismiss", now)?;
        let tx = self.conn.transaction().map_err(err)?;
        save(&tx, &original)?;
        save(&tx, &snooze)?;
        tx.execute("UPDATE events SET notified=1 WHERE item_id=?1", [id])
            .map_err(err)?;
        record(&tx, &original, "item.dismissed", now, now, "snooze", false)?;
        record(&tx, &snooze, "item.created", now, now, "snooze", false)?;
        tx.commit().map_err(err)?;
        Ok(snooze)
    }
    pub fn tick(&mut self, now: DateTime<Utc>) -> Result<bool> {
        let mut items = self.list()?;
        let tx = self.conn.transaction().map_err(err)?;
        let mut changed = false;
        for item in &mut items {
            let events = item.advance(now);
            if events.is_empty() {
                continue;
            }
            changed = true;
            // Preserve both range milestones, but avoid two notifications after waking past the maximum.
            let last_alert = events
                .iter()
                .rposition(|(e, _)| e != "item.completed" || item.kind == Kind::Countdown);
            for (index, (event, at)) in events.iter().enumerate() {
                record(
                    &tx,
                    item,
                    event,
                    *at,
                    now,
                    "scheduler",
                    Some(index) == last_alert,
                )?;
            }
            save(&tx, item)?;
        }
        tx.commit().map_err(err)?;
        Ok(changed)
    }
    pub fn mark_notified(&self, id: i64) -> Result<()> {
        self.conn
            .execute("UPDATE events SET notified=1 WHERE id=?1", [id])
            .map_err(err)?;
        Ok(())
    }
    pub fn settings(&self) -> Result<Settings> {
        use rusqlite::OptionalExtension;
        let payload: Option<String> = self
            .conn
            .query_row("SELECT payload FROM settings WHERE key='app'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(err)?;
        payload
            .map(|s| serde_json::from_str(&s).map_err(err))
            .unwrap_or_else(|| Ok(Settings::default()))
    }
    pub fn set_settings(&self, settings: Settings) -> Result<()> {
        if !(1..=100).contains(&settings.alarm_volume_percent) {
            return Err("Alarm volume must be between 1 and 100 percent".into());
        }
        if (settings.alarm_volume_override || settings.alarm_output_device.is_some() || settings.mute_other_apps) && !cfg!(windows) {
            return Err("Advanced alarm audio preferences are currently supported on Windows".into());
        }
        if settings.alarm_output_device.as_ref().is_some_and(|id| id.is_empty() || id.len() > 1024) {
            return Err("Invalid alarm output device".into());
        }
        self.conn
            .execute(
                "INSERT OR REPLACE INTO settings(key,payload) VALUES('app',?1)",
                [serde_json::to_string(&settings).map_err(err)?],
            )
            .map_err(err)?;
        Ok(())
    }
    pub fn clear_history(&mut self) -> Result<()> {
        let items = self.list()?;
        let tx = self.conn.transaction().map_err(err)?;
        for item in items.iter().filter(|i| !i.active() && i.alert.is_none()) {
            tx.execute("DELETE FROM events WHERE item_id=?1", [&item.id])
                .map_err(err)?;
            tx.execute("DELETE FROM items WHERE id=?1", [&item.id])
                .map_err(err)?;
        }
        tx.commit().map_err(err)
    }
}
fn read_items(conn: &Connection) -> Result<Vec<Item>> {
    let mut stmt = conn
        .prepare("SELECT payload FROM items ORDER BY rowid DESC")
        .map_err(err)?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(err)?;
    rows.map(|r| serde_json::from_str(&r.map_err(err)?).map_err(err))
        .collect()
}
fn save(tx: &Transaction<'_>, item: &Item) -> Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO items(id,payload) VALUES(?1,?2)",
        params![item.id, serde_json::to_string(item).map_err(err)?],
    )
    .map_err(err)?;
    Ok(())
}
fn record(
    tx: &Transaction<'_>,
    item: &Item,
    event: &str,
    at: DateTime<Utc>,
    now: DateTime<Utc>,
    reason: &str,
    notify: bool,
) -> Result<()> {
    let e = Event {
        id: 0,
        item_id: item.id.clone(),
        event: event.into(),
        occurred_at: at,
        recorded_at: now,
        origin: item.source.as_ref().map(|s| s.origin.clone()),
        external_id: item.external_id.clone(),
        reason: reason.into(),
        title: item.title.clone(),
        notified: !notify,
    };
    tx.execute(
        "INSERT INTO events(item_id,payload,notified) VALUES(?1,?2,?3)",
        params![item.id, serde_json::to_string(&e).map_err(err)?, !notify],
    )
    .map_err(err)?;
    Ok(())
}
