use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use patch_db::{LockType, PatchDb};
use rpc_toolkit::command;
use sqlx::SqlitePool;

use crate::context::RpcContext;
use crate::s9pk::manifest::PackageId;
use crate::util::{display_none, display_serializable};
use crate::{Error, ErrorKind};

#[command(subcommands(list, delete, delete_before))]
pub async fn notification(#[context] _ctx: RpcContext) -> Result<(), Error> {
    Ok(())
}

#[command(display(display_serializable))]
pub async fn list(
    #[context] ctx: RpcContext,
    #[arg] before: Option<u32>,
    #[arg] limit: Option<u32>,
) -> Result<Vec<Notification>, Error> {
    let limit = limit.unwrap_or(40);
    let mut handle = ctx.db.handle();
    match before {
        None => {
            let model = crate::db::DatabaseModel::new()
                .server_info()
                .unread_notification_count();
            model.lock(&mut handle, patch_db::LockType::Write).await;
            let records = sqlx::query!(
                "SELECT id, package_id, created_at, code, level, title, message, data FROM notifications ORDER BY id DESC LIMIT ?",
                limit
            ).fetch_all(&ctx.secret_store).await?;
            let notifs = records
                .into_iter()
                .map(|r| {
                    Ok(Notification {
                        id: r.id as u32,
                        package_id: r.package_id.and_then(|p| p.parse().ok()),
                        created_at: DateTime::from_utc(r.created_at, Utc),
                        code: r.code as u32,
                        level: match r.level.parse::<NotificationLevel>() {
                            Ok(a) => a,
                            Err(e) => return Err(e.into()),
                        },
                        title: r.title,
                        message: r.message,
                        data: match r.data {
                            None => serde_json::Value::Null,
                            Some(v) => match v.parse::<serde_json::Value>() {
                                Ok(a) => a,
                                Err(e) => {
                                    return Err(Error::new(
                                        anyhow!("Invalid Notification Data: {}", e),
                                        ErrorKind::ParseDbField,
                                    ))
                                }
                            },
                        },
                    })
                })
                .collect::<Result<Vec<Notification>, Error>>()?;
            // set notification count to zero
            model.put(&mut handle, &0).await?;
            Ok(notifs)
        }
        Some(before) => {
            let records = sqlx::query!(
                "SELECT id, package_id, created_at, code, level, title, message, data FROM notifications WHERE id < ? ORDER BY id DESC LIMIT ?",
                before,
                limit
            ).fetch_all(&ctx.secret_store).await?;
            records
                .into_iter()
                .map(|r| {
                    Ok(Notification {
                        id: r.id as u32,
                        package_id: r.package_id.and_then(|p| p.parse().ok()),
                        created_at: DateTime::from_utc(r.created_at, Utc),
                        code: r.code as u32,
                        level: match r.level.parse::<NotificationLevel>() {
                            Ok(a) => a,
                            Err(e) => return Err(e.into()),
                        },
                        title: r.title,
                        message: r.message,
                        data: match r.data {
                            None => serde_json::Value::Null,
                            Some(v) => match v.parse::<serde_json::Value>() {
                                Ok(a) => a,
                                Err(e) => {
                                    return Err(Error::new(
                                        anyhow!("Invalid Notification Data: {}", e),
                                        ErrorKind::ParseDbField,
                                    ))
                                }
                            },
                        },
                    })
                })
                .collect::<Result<Vec<Notification>, Error>>()
        }
    }
}

#[command(display(display_none))]
pub async fn delete(#[context] ctx: RpcContext, #[arg] id: u32) -> Result<(), Error> {
    sqlx::query!("DELETE FROM notifications WHERE id = ?", id)
        .execute(&ctx.secret_store)
        .await?;
    Ok(())
}

#[command(display(display_none))]
pub async fn delete_before(#[context] ctx: RpcContext, #[arg] before: u32) -> Result<(), Error> {
    sqlx::query!("DELETE FROM notifications WHERE id < ?", before)
        .execute(&ctx.secret_store)
        .await?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum NotificationLevel {
    Success,
    Info,
    Warning,
    Error,
}
impl fmt::Display for NotificationLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NotificationLevel::Success => write!(f, "success"),
            NotificationLevel::Info => write!(f, "info"),
            NotificationLevel::Warning => write!(f, "warning"),
            NotificationLevel::Error => write!(f, "error"),
        }
    }
}
pub struct InvalidNotificationLevel;
impl From<InvalidNotificationLevel> for crate::Error {
    fn from(_val: InvalidNotificationLevel) -> Self {
        Error::new(
            anyhow!("Invalid Notification Level"),
            ErrorKind::ParseDbField,
        )
    }
}
impl FromStr for NotificationLevel {
    type Err = InvalidNotificationLevel;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s == "success" => Ok(NotificationLevel::Success),
            s if s == "info" => Ok(NotificationLevel::Info),
            s if s == "warning" => Ok(NotificationLevel::Warning),
            s if s == "error" => Ok(NotificationLevel::Error),
            _ => Err(InvalidNotificationLevel),
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Notification {
    id: u32,
    package_id: Option<PackageId>, // TODO change for package id newtype
    created_at: DateTime<Utc>,
    code: u32,
    level: NotificationLevel,
    title: String,
    message: String,
    data: serde_json::Value,
}

pub enum NotificationSubtype {
    General,
    BackupReport {
        server_attempted: bool,
        server_error: Option<String>,
        packages: HashMap<String, Option<String>>,
    },
}
impl NotificationSubtype {
    fn to_json(&self) -> serde_json::Value {
        match &self {
            &NotificationSubtype::General => serde_json::Value::Null,
            &NotificationSubtype::BackupReport {
                server_attempted,
                server_error,
                packages,
            } => {
                let mut pkgs_map = serde_json::Map::new();
                for (k, v) in packages.iter() {
                    pkgs_map.insert(
                        k.clone(),
                        match v {
                            None => serde_json::json!({ "error": serde_json::Value::Null }),
                            Some(x) => serde_json::json!({ "error": x }),
                        },
                    );
                }
                serde_json::json!({
                    "server": {
                        "attempted": server_attempted,
                        "error": server_error,
                    },
                    "packages": serde_json::Value::Object(pkgs_map)
                })
            }
        }
    }
    fn code(&self) -> u32 {
        match &self {
            &Self::General => 0,
            &Self::BackupReport {
                server_attempted: _,
                server_error: _,
                packages: _,
            } => 1,
        }
    }
}

pub async fn notify(
    sqlite: &SqlitePool,
    patchdb: &PatchDb,
    package_id: Option<PackageId>,
    level: NotificationLevel,
    title: String,
    message: String,
    subtype: NotificationSubtype,
) -> Result<(), Error> {
    let mut handle = patchdb.handle();
    let mut count = crate::db::DatabaseModel::new()
        .server_info()
        .unread_notification_count()
        .get_mut(&mut handle)
        .await?;
    let sql_package_id = package_id.map::<String, _>(|p| p.into());
    let sql_code = subtype.code();
    let sql_level = format!("{}", level);
    let sql_data = format!("{}", subtype.to_json());
    sqlx::query!(
        "INSERT INTO notifications (package_id, code, level, title, message, data) VALUES (?, ?, ?, ?, ?, ?)",
        sql_package_id,
        sql_code,
        sql_level,
        title,
        message,
        sql_data
    ).execute(sqlite).await?;
    *count += 1;
    count.save(&mut handle).await?;
    Ok(())
}

#[test]
fn serialization() {
    println!(
        "{}",
        serde_json::json!({ "test": "abcdefg", "num": 32, "nested": { "inner": null, "xyz": [0,2,4]}})
    )
}