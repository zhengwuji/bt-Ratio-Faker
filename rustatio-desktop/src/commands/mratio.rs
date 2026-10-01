//! mRatio 生态集成命令:.mRClient 伪装档案、.mRSave 历史存档。

use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

use rustatio_core::mrclient::{self, MrClientProfile};
use rustatio_core::mrsave::{self, MrSaveRecord};
use rustatio_core::torrent::{ClientType, TorrentInfo};
use rustatio_core::{FakerConfig, RatioFaker};

use crate::logging::log_and_emit;
use crate::state::{AppState, FakerInstance};
use rustatio_watch::InstanceSource;

fn find_mr_dirs(dir_name: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut push_unique = |p: std::path::PathBuf| {
        if p.is_dir() && !out.contains(&p) {
            out.push(p);
        }
    };
    // 目录约定:exe 同级向上逐级查找(兼容免安装、target/release 等布局)
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent().map(std::path::Path::to_path_buf);
        for _ in 0..5 {
            let Some(dir) = cur else { break };
            push_unique(dir.join(dir_name));
            cur = dir.parent().map(std::path::Path::to_path_buf);
        }
    }
    // 用户工作目录兜底
    if let Ok(cwd) = std::env::current_dir() {
        push_unique(cwd.join(dir_name));
        if let Some(p1) = cwd.parent() {
            push_unique(p1.join(dir_name));
        }
    }
    out
}

/// 列出可用的 .mRClient 伪装档案
#[tauri::command]
pub async fn list_mr_clients(app: AppHandle) -> Result<Vec<MrClientProfile>, String> {
    let dirs = find_mr_dirs("mRatioClients");
    let mut profiles = Vec::new();
    let mut failed = 0usize;
    for dir in &dirs {
        let (ok, bad) = mrclient::load_mr_clients_dir(dir);
        failed += bad.len();
        for p in ok {
            // 多目录去重(按显示名)
            if !profiles.iter().any(|x: &MrClientProfile| x.name == p.name) {
                profiles.push(p);
            }
        }
    }
    log_and_emit!(
        &app,
        info,
        "Loaded {} mRatio client profiles ({} failed)",
        profiles.len(),
        failed
    );
    Ok(profiles)
}

/// 列出 .mRSave 历史存档(按修改时间新→旧)
#[tauri::command]
pub async fn list_mr_history(app: AppHandle) -> Result<Vec<MrSaveRecord>, String> {
    let dirs = find_mr_dirs("mRatioTorrents");
    let mut records = Vec::new();
    let mut seen_hashes = std::collections::HashSet::new();
    for dir in &dirs {
        let (ok, _bad) = mrsave::load_mr_save_dir(dir);
        for r in ok {
            if seen_hashes.insert(r.info_hash_hex.clone()) {
                records.push(r);
            }
        }
    }
    records.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    log_and_emit!(
        &app,
        info,
        "Found {} mRatio history records",
        records.len()
    );
    Ok(records)
}

/// 导入结果(供前端构建实例)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryImportResult {
    pub info_hash_hex: String,
    pub torrent: TorrentInfo,
    pub config: FakerConfig,
    pub tags: Vec<String>,
    pub skipped_existing: bool,
    pub instance_id: u32,
}

fn parse_info_hash(hex: &str) -> Option<[u8; 20]> {
    let bytes = (0..20)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    bytes.try_into().ok()
}

fn build_config_from_record(record: &MrSaveRecord) -> FakerConfig {
    let (client, version) = mrsave::parse_emulation_name(&record.emulation);
    let mut config = FakerConfig {
        client_type: client
            .as_deref()
            .and_then(ClientType::from_id)
            .unwrap_or(ClientType::QBittorrent),
        client_version: version,
        initial_uploaded: record.total_upload,
        initial_downloaded: record.total_download,
        proxy_url: mrsave::proxy_url_from_record(record),
        ..FakerConfig::default()
    };
    if let Some(ratio) = record.stop_ratio {
        config.stop_at_ratio = Some(ratio);
    }
    config
}

/// 批量导入历史记录:按元数据构建 TorrentInfo 并创建后端实例(暂停状态)
#[tauri::command]
pub async fn import_history_instances(
    records: Vec<MrSaveRecord>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<HistoryImportResult>, String> {
    let mut results = Vec::new();
    let mut fakers = state.fakers.write().await;
    let mut next_id = state.next_instance_id.write().await;

    // 单次导入上限(避免大量历史拖慢 UI);调用前已按修改时间新→旧排序
    const IMPORT_CAP: usize = 200;

    // 同一代理共享一个 reqwest 客户端,避免每个实例各建连接池
    let mut proxied_clients: std::collections::HashMap<String, rustatio_core::reqwest::Client> =
        std::collections::HashMap::new();

    for record in records.into_iter().take(IMPORT_CAP) {
        let Some(info_hash) = parse_info_hash(&record.info_hash_hex) else {
            continue;
        };
        // 与现有实例/本批次去重
        if fakers
            .values()
            .any(|inst| inst.torrent.info_hash == info_hash)
            || results
                .iter()
                .any(|r: &HistoryImportResult| r.torrent.info_hash == info_hash)
        {
            continue;
        }

        let torrent = TorrentInfo::from_metadata(
            record.name.clone(),
            record.announce.clone(),
            info_hash,
            record.total_size,
            record.piece_length,
        );
        let config = build_config_from_record(&record);

        // 命中缓存的代理客户端;无代理则用共享客户端
        let http_client = match &config.proxy_url {
            Some(url) => {
                if let Some(c) = proxied_clients.get(url) {
                    Some(c.clone())
                } else {
                    let proxy = rustatio_core::reqwest::Proxy::all(url)
                        .map_err(|e| format!("invalid proxy url: {e}"))?;
                    let client = rustatio_core::reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(30))
                        .gzip(true)
                        .proxy(proxy)
                        .build()
                        .map_err(|e| format!("failed to build proxied client: {e}"))?;
                    proxied_clients.insert(url.clone(), client.clone());
                    Some(client)
                }
            }
            None => Some(state.http_client.clone()),
        };
        let faker = RatioFaker::new(
            Arc::new(torrent.clone()),
            config.clone(),
            http_client,
        )
        .map_err(|e| format!("Failed to create faker for '{}': {e}", record.name))?;

        let instance_id = *next_id;
        *next_id += 1;
        let now = crate::state::now_secs();

        fakers.insert(
            instance_id,
            FakerInstance {
                faker: Arc::new(rustatio_core::RatioFakerHandle::new(faker)),
                torrent: Arc::new(torrent.clone()),
                summary: Arc::new(torrent.summary()),
                config: config.clone(),
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: vec!["mRatio 历史".to_string()],
                created_at: now,
                source: InstanceSource::Manual,
            },
        );

        log_and_emit!(
            &app,
            instance_id,
            info,
            "Restored mRatio history: {} (uploaded {} MB)",
            record.name,
            record.total_upload / 1024 / 1024
        );

        results.push(HistoryImportResult {
            info_hash_hex: record.info_hash_hex.clone(),
            torrent,
            config,
            tags: vec!["mRatio 历史".to_string()],
            skipped_existing: false,
            instance_id,
        });
    }

    Ok(results)
}

/// 会话恢复:为无路径实例(历史导入)重接元数据 torrent
#[tauri::command]
pub async fn reattach_metadata_torrent(
    instance_id: u32,
    record: MrSaveRecord,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<TorrentInfo, String> {
    let Some(info_hash) = parse_info_hash(&record.info_hash_hex) else {
        return Err("invalid info-hash".into());
    };
    let torrent = TorrentInfo::from_metadata(
        record.name.clone(),
        record.announce.clone(),
        info_hash,
        record.total_size,
        record.piece_length,
    );

    let mut fakers = state.fakers.write().await;
    let response_torrent = torrent.clone();
    let torrent_arc = Arc::new(torrent.clone());
    let summary_arc = Arc::new(torrent.summary());
    match fakers.entry(instance_id) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            let config = build_config_from_record(&record);
            let faker = RatioFaker::new(
                Arc::clone(&torrent_arc),
                config.clone(),
                Some(state.http_client.clone()),
            )
            .map_err(|e| format!("创建伪造器失败:{e}"))?;
            let now = crate::state::now_secs();
            entry.insert(FakerInstance {
                faker: Arc::new(rustatio_core::RatioFakerHandle::new(faker)),
                torrent: torrent_arc,
                summary: summary_arc,
                config,
                cumulative_uploaded: 0,
                cumulative_downloaded: 0,
                tags: vec!["mRatio 历史".to_string()],
                created_at: now,
                source: InstanceSource::Manual,
            });
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let instance = entry.get_mut();
            let config = instance.config.clone();
            let faker = RatioFaker::new(Arc::clone(&torrent_arc), config, Some(state.http_client.clone()))
                .map_err(|e| format!("创建伪造器失败:{e}"))?;
            instance.torrent = torrent_arc;
            instance.summary = summary_arc;
            instance.faker = Arc::new(rustatio_core::RatioFakerHandle::new(faker));
        }
    }

    log_and_emit!(
        &app,
        instance_id,
        info,
        "Reattached metadata torrent: {}",
        record.name
    );
    Ok(response_torrent)
}

/// 测试代理连通性:经代理访问 ipify 获取出口 IP(10 秒超时)
#[tauri::command]
pub async fn test_proxy(proxy_url: String) -> Result<String, String> {
    use std::time::Instant;
    let url = proxy_url.trim().to_string();
    if url.is_empty() {
        return Err("代理地址为空".into());
    }
    log::info!("[代理测试] 开始: {url}");
    let proxy = rustatio_core::reqwest::Proxy::all(&url)
        .map_err(|e| format!("代理地址无效: {e}"))?;
    let client = rustatio_core::reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .proxy(proxy)
        .build()
        .map_err(|e| format!("客户端构建失败: {e}"))?;
    let started = Instant::now();
    let body = client
        .get("https://api.ipify.org?format=json")
        .send()
        .await
        .map_err(|e| {
            let msg = format!("连接失败: {e}");
            log::warn!("[代理测试] 失败 url={url}: {msg}");
            msg
        })?
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {e}"))?;
    let ip = body
        .split("\"ip\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or("未知")
        .to_string();
    let ms = started.elapsed().as_millis();
    log::info!("[代理测试] 成功 url={url} 出口IP={ip} 耗时={ms}ms");
    Ok(format!("{ip} ({ms}ms)"))
}

/// 前端事件日志(记录界面侧功能使用,写入调试文件)
#[tauri::command]
pub async fn frontend_log(level: String, message: String) -> Result<(), String> {
    let msg = format!("[UI] {message}");
    match level.as_str() {
        "error" => log::error!("{msg}"),
        "warn" => log::warn!("{msg}"),
        "debug" => log::debug!("{msg}"),
        _ => log::info!("{msg}"),
    }
    Ok(())
}

/// 应用代理结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProxyResult {
    pub updated: usize,
    pub skipped_running: usize,
}

/// 把代理一键应用到全部实例(运行中的实例跳过,下次启动时由前端配置生效)
#[tauri::command]
pub async fn apply_proxy_all(
    proxy_url: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ApplyProxyResult, String> {
    use rustatio_core::FakerState;

    let url = proxy_url
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty());
    log::info!("[功能] apply_proxy_all proxy={:?}", url);

    let mut fakers = state.fakers.write().await;
    let ids: Vec<u32> = fakers.keys().copied().collect();
    let mut updated = 0usize;
    let mut skipped_running = 0usize;
    for id in ids {
        let instance = fakers.get_mut(&id).expect("id from keys");
        let stats = instance.faker.stats_snapshot();
        if matches!(
            stats.state,
            FakerState::Running | FakerState::Starting | FakerState::Paused
        ) {
            skipped_running += 1;
            continue;
        }
        let mut config = instance.config.clone();
        if config.proxy_url == url {
            continue; // 本来就一致
        }
        config.proxy_url = url.clone();
        instance
            .faker
            .update_config(config.clone(), Some(state.http_client.clone()))
            .await
            .map_err(|e| format!("instance {id}: {e}"))?;
        instance.config = config;
        updated += 1;
    }
    drop(fakers);
    let _ = state.save_state().await;
    log_and_emit!(
        &app,
        info,
        "Proxy applied to {} instances ({} skipped: running)",
        updated,
        skipped_running
    );
    Ok(ApplyProxyResult { updated, skipped_running })
}
