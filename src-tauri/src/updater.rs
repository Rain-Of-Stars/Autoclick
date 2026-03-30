use std::{env, time::Duration};

use anyhow::Context;
use serde::Serialize;
use tauri::{AppHandle, Runtime, Url};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    pub configured: bool,
    pub pubkey_configured: bool,
    pub install_mode: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub configured: bool,
    pub checked: bool,
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub body: Option<String>,
    pub date: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
struct UpdaterSettings {
    endpoints: Vec<String>,
    pubkey: Option<String>,
}

pub fn status() -> UpdaterStatus {
    let settings = load_settings();
    let reason = (!settings.endpoints.is_empty() && settings.pubkey.is_none())
        .then(|| "已配置发布通道，但缺少更新签名公钥。".to_string())
        .or_else(|| {
            (settings.endpoints.is_empty())
                .then(|| "尚未配置发布通道，更新功能处于占位状态。".to_string())
        });

    UpdaterStatus {
        configured: !settings.endpoints.is_empty() && settings.pubkey.is_some(),
        pubkey_configured: settings.pubkey.is_some(),
        install_mode: "passive".to_string(),
        reason,
    }
}

pub async fn check_for_updates<R: Runtime>(
    app: &AppHandle<R>,
) -> anyhow::Result<UpdateCheckResult> {
    let settings = load_settings();
    let state = status();
    if !state.configured {
        return Ok(UpdateCheckResult {
            configured: false,
            checked: false,
            update_available: false,
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            latest_version: None,
            body: None,
            date: None,
            reason: state.reason,
        });
    }

    let mut builder = app.updater_builder();
    builder = builder.timeout(Duration::from_secs(20));
    if let Some(pubkey) = settings.pubkey.as_deref() {
        builder = builder.pubkey(pubkey);
    }

    let endpoint_urls = settings
        .endpoints
        .iter()
        .map(|value| Url::parse(value))
        .collect::<Result<Vec<_>, _>>()
        .context("更新地址配置无效")?;

    let response = builder
        .endpoints(endpoint_urls)
        .context("更新地址配置无效")?
        .build()?
        .check()
        .await?;
    let (update_available, latest_version, body, date) = if let Some(update) = response {
        (
            true,
            Some(update.version.to_string()),
            update.body,
            update.date.map(|value| value.to_string()),
        )
    } else {
        (false, None, None, None)
    };

    Ok(UpdateCheckResult {
        configured: true,
        checked: true,
        update_available,
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version,
        body,
        date,
        reason: None,
    })
}

fn load_settings() -> UpdaterSettings {
    UpdaterSettings {
        endpoints: load_list("AUTOCLICK_UPDATER_ENDPOINTS"),
        pubkey: env::var("AUTOCLICK_UPDATER_PUBKEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    }
}

fn load_list(key: &str) -> Vec<String> {
    env::var(key)
        .unwrap_or_default()
        .split(['\n', ';', ','])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}
