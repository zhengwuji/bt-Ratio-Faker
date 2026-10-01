use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{Manager, State};

const RPM_LIKE: [&str; 9] =
    ["fedora", "rhel", "centos", "rocky", "almalinux", "suse", "opensuse", "sles", "mageia"];
const DEB_LIKE: [&str; 8] =
    ["debian", "ubuntu", "linuxmint", "pop", "elementary", "kali", "raspbian", "zorin"];

fn matches_family<'a>(mut tokens: impl Iterator<Item = &'a str>, family: &[&str]) -> bool {
    tokens.any(|token| family.contains(&token))
}

#[tauri::command]
pub fn detect_linux_package_type() -> String {
    #[cfg(target_os = "linux")]
    {
        use os_release::OsRelease;

        let Ok(os_release) = OsRelease::new() else {
            return "unknown".to_string();
        };

        let id = os_release.id.trim().to_lowercase();
        let id_like = os_release.id_like.trim().to_lowercase();
        let tokens = id.split_whitespace().chain(id_like.split_whitespace());

        if matches_family(tokens.clone(), &RPM_LIKE) {
            return "rpm".to_string();
        }

        if matches_family(tokens, &DEB_LIKE) {
            return "deb".to_string();
        }

        "unknown".to_string()
    }

    #[cfg(not(target_os = "linux"))]
    {
        "unknown".to_string()
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn close_to_tray(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::AppState>();
    state.close_prompt_open.store(false, Ordering::Relaxed);

    let Some(window) = app.get_webview_window("main") else {
        return Err("Main window not found".to_string());
    };

    if let Err(e) = window.hide() {
        return Err(e.to_string());
    }

    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn quit_app(app: tauri::AppHandle) {
    let state = app.state::<crate::state::AppState>();
    state.should_exit.store(true, Ordering::Relaxed);
    state.close_prompt_open.store(false, Ordering::Relaxed);

    app.exit(0);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cancel_close_prompt(app: tauri::AppHandle) {
    let state = app.state::<crate::state::AppState>();
    state.close_prompt_open.store(false, Ordering::Relaxed);
}

#[derive(Serialize)]
pub struct DesktopNetworkStatus {
    #[serde(rename = "peer_listener_port")]
    port: Option<u16>,
    #[serde(rename = "peer_listener_active")]
    active: bool,
    #[serde(rename = "peer_listener_error")]
    error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct GeoResult {
    pub ip: String,
    pub country: String,
    pub country_code: String,
}

/// 批量查询 IP 归属地(ip-api.com 免费接口,45 次/分钟,支持 v4/v6)
#[tauri::command]
pub async fn geo_lookup_ips(
    ips: Vec<String>,
    state: State<'_, crate::state::AppState>,
) -> Result<Vec<GeoResult>, String> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct BatchItem {
        status: String,
        #[serde(default)]
        country: String,
        #[serde(rename = "countryCode", default)]
        country_code: String,
        query: String,
    }

    // 去重、限制数量
    let mut unique: Vec<String> = Vec::new();
    for ip in ips {
        if !ip.is_empty() && !unique.contains(&ip) {
            unique.push(ip);
        }
    }
    unique.truncate(100);
    if unique.is_empty() {
        return Ok(Vec::new());
    }

    let url = "http://ip-api.com/batch?fields=status,country,countryCode,query";
    let client = state.http_client.clone();
    let body = serde_json::to_string(&unique).map_err(|e| format!("Geo serialize failed: {e}"))?;
    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Geo lookup failed: {e}"))?;

    let text = resp.text().await.map_err(|e| format!("Geo read failed: {e}"))?;
    let items: Vec<BatchItem> =
        serde_json::from_str(&text).map_err(|e| format!("Geo parse failed: {e}"))?;

    Ok(items
        .into_iter()
        .map(|it| GeoResult {
            ip: it.query,
            country: if it.status == "success" { it.country } else { String::new() },
            country_code: if it.status == "success" { it.country_code } else { String::new() },
        })
        .collect())
}

#[tauri::command]
pub async fn get_network_status(
    state: State<'_, crate::state::AppState>,
) -> Result<DesktopNetworkStatus, String> {
    let status = state.peer_listener_status().await;
    Ok(DesktopNetworkStatus {
        port: status.bound_port,
        active: status.bound_port.is_some(),
        error: status.last_error,
    })
}
