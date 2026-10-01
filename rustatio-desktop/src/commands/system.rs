use serde::Serialize;
use std::sync::Arc;
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

#[derive(serde::Deserialize)]
pub struct PeerProbeTarget {
    pub ip: String,
    pub port: u16,
}

#[derive(serde::Serialize)]
pub struct PeerProbeResult {
    pub ip: String,
    pub port: u16,
    /// 对方接受连接并完成 BT 握手(端口真实可达)
    pub online: bool,
    /// 对方握手响应中的 peer_id,可解析出真实客户端
    pub peer_id: Option<String>,
}

/// 与 peer 列表中的对方做一次标准 BT 握手以获取其 peer_id,从而识别真实客户端。
///
/// 安全性:这是所有下载器拿到 tracker peer 列表后的正常行为(qB/Transmission 亦然),
/// tracker 完全不感知 peer 直连,PT 站点无法据此判定任何异常。握手使用本实例自己的
/// info_hash 与伪装 peer_id(与 announce 指纹完全一致),拿到对方握手响应后立即断开,
/// 不交换任何 piece/扩展数据。仅由用户手动触发。
#[tauri::command]
pub async fn probe_peer_clients(
    instance_id: u32,
    targets: Vec<PeerProbeTarget>,
    state: State<'_, crate::state::AppState>,
) -> Result<Vec<PeerProbeResult>, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::Semaphore;

    let faker = {
        let fakers = state.fakers.read().await;
        let instance =
            fakers.get(&instance_id).ok_or_else(|| format!("Instance {instance_id} not found"))?;
        Arc::clone(&instance.faker)
    };

    let info_hash = faker.info_hash().await;
    let our_peer_id: [u8; 20] = faker.peer_id_bytes().await.unwrap_or([0u8; 20]);

    // 去重 + 过滤非法 IP,上限 100 个
    let mut unique: Vec<(std::net::IpAddr, u16, String)> = Vec::new();
    for t in targets {
        if t.ip.is_empty() {
            continue;
        }
        let Ok(ip) = t.ip.parse::<std::net::IpAddr>() else { continue };
        if !unique.iter().any(|(uip, uport, _)| *uip == ip && *uport == t.port) {
            unique.push((ip, t.port, t.ip));
        }
    }
    unique.truncate(100);
    if unique.is_empty() {
        return Ok(Vec::new());
    }

    let semaphore = Arc::new(Semaphore::new(8));
    let mut tasks = Vec::with_capacity(unique.len());

    for (ip, port, ip_text) in unique {
        let permits = Arc::clone(&semaphore);
        tasks.push(tokio::spawn(async move {
            let _permit = permits.acquire_owned().await;
            let addr = std::net::SocketAddr::new(ip, port);
            let outcome = tokio::time::timeout(std::time::Duration::from_secs(4), async {
                let mut stream = tokio::net::TcpStream::connect(addr).await?;
                let mut hs = Vec::with_capacity(68);
                hs.push(19u8);
                hs.extend_from_slice(b"BitTorrent protocol");
                hs.extend_from_slice(&[0u8; 8]); // 保留字段全零(经典客户端行为)
                hs.extend_from_slice(&info_hash);
                hs.extend_from_slice(&our_peer_id);
                stream.write_all(&hs).await?;
                stream.flush().await?;
                let mut buf = [0u8; 68];
                stream.read_exact(&mut buf).await?;
                if &buf[1..20] != b"BitTorrent protocol" {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid protocol string",
                    ));
                }
                Ok::<Vec<u8>, std::io::Error>(buf[48..68].to_vec())
            })
            .await;

            let (online, peer_id) = match outcome {
                Ok(Ok(raw)) => {
                    let cleaned: String = raw
                        .iter()
                        .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { ' ' })
                        .collect();
                    (true, Some(cleaned))
                }
                _ => (false, None),
            };

            PeerProbeResult { ip: ip_text, port, online, peer_id }
        }));
    }

    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        if let Ok(r) = task.await {
            results.push(r);
        }
    }
    Ok(results)
}
