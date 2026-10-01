//! mRatio 种子会话存档(.mRSave)解析。
//!
//! mRatio 把每个种子的会话状态写成明文 bencode 字典(mRatioTorrents/<info-hash>.mRSave),
//! 共 47 个 mRTorrent* 字段:名称/announce/info-hash(Base64)/大小/伪造统计/
//! 模拟客户端("qBittorrent 4.6.7")/代理/停止条件等。
//! 注意:存档不含原始 .torrent 文件,但元数据足以构建 TorrentInfo(无需种子文件即可恢复)。

use crate::protocol::bencode;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Error)]
pub enum MrSaveError {
    #[error("bencode: {0}")]
    Bencode(#[from] bencode::BencodeError),
    #[error("not a dictionary")]
    NotADictionary,
    #[error("missing info-hash")]
    MissingHash,
    #[error("invalid info-hash encoding")]
    BadHash,
}

/// 一条历史会话记录(已转换为 Rustatio 语义)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrSaveRecord {
    /// 来源 .mRSave 文件路径
    pub file_path: String,
    /// 文件修改时间(Unix 秒,用于排序)
    pub modified_at: i64,
    /// 种子名称(mRTorrentName)
    pub name: String,
    /// announce URL(mRTorrentAnnounce)
    pub announce: String,
    /// info-hash 十六进制
    pub info_hash_hex: String,
    /// 总大小(字节)
    pub total_size: u64,
    /// piece 大小
    pub piece_length: u64,
    /// 伪造累计上传(字节,mRTorrentTotalUpload)
    pub total_upload: u64,
    /// 伪造累计下载(字节,mRTorrentTotalDownload)
    pub total_download: u64,
    /// mRatio 模拟客户端名(如 "qBittorrent 4.6.7")
    pub emulation: String,
    /// 代理类型(0=无 1..3=有,原样保留)
    pub proxy_type: i64,
    /// 代理主机
    pub proxy_host: String,
    /// 代理端口
    pub proxy_port: i64,
    /// 停止比率(mRTorrentStopRatioEnabled/Value)
    pub stop_ratio: Option<f64>,
    /// 做种时长秒(mRTorrentTotalSeedingTime)
    pub seeding_time: i64,
    /// 标签(mRTorrentTags,逗号分隔)
    pub tags: String,
}

fn bstr(v: &Option<Vec<u8>>) -> String {
    v.as_deref().map(decode_text).unwrap_or_default()
}

/// 存档声明 encoding=UTF-8:优先按 UTF-8 解码,失败回退 latin1(旧档案兼容)
fn decode_text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec())
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect())
}

/// 解析 .mRSave 内容
pub fn parse_mr_save(path: &str, modified_at: i64, data: &[u8]) -> Result<MrSaveRecord, MrSaveError> {
    let dict = match bencode::parse(data)? {
        serde_bencode::value::Value::Dict(d) => d,
        _ => return Err(MrSaveError::NotADictionary),
    };
    let get = |key: &str| dict.get(key.as_bytes()).and_then(|v| match v {
        serde_bencode::value::Value::Bytes(b) => Some(b.clone()),
        _ => None,
    });
    let get_str = |key: &str| get(key).map(|b| decode_text(&b));
    let get_i64 = |key: &str| get_str(key).and_then(|s| s.trim().parse::<i64>().ok());
    let get_f64 = |key: &str| get_str(key).and_then(|s| s.trim().parse::<f64>().ok());

    // info-hash:Base64 编码的 20 字节
    let hash_b64 = get_str("mRTorrentHash").ok_or(MrSaveError::MissingHash)?;
    let hash_bytes = crate::mrclient::base64_info_hash(&hash_b64).ok_or(MrSaveError::BadHash)?;
    let info_hash_hex: String = hash_bytes.iter().map(|b| format!("{b:02x}")).collect();

    let stop_ratio = if get_str("mRTorrentStopRatioEnabled")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        get_f64("mRTorrentStopRatioValue")
    } else {
        None
    };

    Ok(MrSaveRecord {
        file_path: path.to_string(),
        modified_at,
        name: get_str("mRTorrentName").unwrap_or_else(|| "未知名称".into()),
        announce: get_str("mRTorrentAnnounce").unwrap_or_default(),
        info_hash_hex,
        total_size: get_i64("mRTorrentTotalSize").unwrap_or(0).max(0) as u64,
        piece_length: get_i64("mRTorrentPieceSize").unwrap_or(0).max(0) as u64,
        total_upload: get_i64("mRTorrentTotalUpload").unwrap_or(0).max(0) as u64,
        total_download: get_i64("mRTorrentTotalDownload").unwrap_or(0).max(0) as u64,
        emulation: get_str("mRTorrentEmulation").unwrap_or_default(),
        proxy_type: get_i64("mRTorrentProxyType").unwrap_or(0),
        proxy_host: get_str("mRTorrentProxyHost").unwrap_or_default(),
        proxy_port: get_i64("mRTorrentProxyPort").unwrap_or(0),
        stop_ratio,
        seeding_time: get_i64("mRTorrentTotalSeedingTime").unwrap_or(0),
        tags: get_str("mRTorrentTags").unwrap_or_default(),
    })
}

/// 扫描目录下的 *.mRSave(按修改时间新→旧),返回 (成功, 失败[文件名, 错误])
pub fn load_mr_save_dir(dir: &std::path::Path) -> (Vec<MrSaveRecord>, Vec<(String, String)>) {
    let mut ok = Vec::new();
    let mut failed = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return (ok, failed),
    };
    let mut files: Vec<(PathBuf, Option<std::fs::Metadata>)> = entries
        .flatten()
        .map(|e| {
            let path = e.path();
            let meta = e.metadata().ok();
            (path, meta)
        })
        .filter(|(p, _)| {
            p.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mRSave"))
        })
        .collect();
    // 新修改的排前面
    files.sort_by(|a, b| {
        let ta = a.1.as_ref().and_then(|m| m.modified().ok());
        let tb = b.1.as_ref().and_then(|m| m.modified().ok());
        tb.cmp(&ta)
    });
    for (path, meta) in files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let modified_at = meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        match std::fs::read(&path)
            .map_err(|e| MrSaveError::Bencode(bencode::BencodeError::ParseError(e.to_string())))
            .and_then(|data| parse_mr_save(&path.to_string_lossy(), modified_at, &data))
        {
            Ok(r) => ok.push(r),
            Err(e) => failed.push((name, e.to_string())),
        }
    }
    (ok, failed)
}

/// 历史目录候选(与伪装档案目录同样的查找顺序,目录名 mRatioTorrents)
pub fn candidate_save_dirs() -> Vec<PathBuf> {
    crate::mrclient::candidate_client_dirs()
        .into_iter()
        .map(|p| p.with_file_name("mRatioTorrents"))
        .collect()
}

/// 从 mRatio 模拟客户端名(如 "qBittorrent 4.6.7"/"uTorrent 3.4.3")解析
/// (客户端类型 id, 版本)。无法识别时返回 (None, None)。
pub fn parse_emulation_name(emulation: &str) -> (Option<String>, Option<String>) {
    let e = emulation.trim();
    let lower = e.to_lowercase();
    let known: &[(&str, &str)] = &[
        ("qbittorrent", "qbittorrent"),
        ("qBittorrent", "qbittorrent"),
        ("µtorrent", "utorrent"),
        ("utorrent", "utorrent"),
        ("transmission", "transmission"),
        ("deluge", "deluge"),
        ("bittorrent", "bittorrent"),
        ("rtorrent", "rtorrent"),
    ];
    for (needle, id) in known {
        if lower.starts_with(&needle.to_lowercase()) {
            // 版本 = 名称去掉客户端名后的剩余部分
            let version = e
                .get(needle.len()..)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            return (Some(id.to_string()), version);
        }
    }
    (None, None)
}

/// 由存档元数据构建 Rustatio 代理 URL(mRatio 代理类型 1=SOCKS4 2=SOCKS5 3=HTTP)
pub fn proxy_url_from_record(record: &MrSaveRecord) -> Option<String> {
    if record.proxy_type == 0 || record.proxy_host.is_empty() || record.proxy_port == 0 {
        return None;
    }
    let scheme = match record.proxy_type {
        1 => "socks5", // SOCKS4 上游未支持,SOCKS5 近似
        2 => "socks5",
        3 => "http",
        _ => return None,
    };
    Some(format!("{}://{}:{}", scheme, record.proxy_host, record.proxy_port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use serde_bencode::value::Value;

    #[test]
    fn parse_synthetic_save() {
        let mut d: HashMap<Vec<u8>, Value> = HashMap::new();
        let mut put = |k: &str, v: &str| {
            d.insert(k.as_bytes().to_vec(), Value::Bytes(v.as_bytes().to_vec()));
        };
        put("mRTorrentName", "测试种子");
        put("mRTorrentAnnounce", "https://t.example/announce");
        put("mRTorrentHash", "AACko2nFJYbJsJHH8V1KcAqkEjw=");
        put("mRTorrentTotalSize", "1024000");
        put("mRTorrentPieceSize", "16384");
        put("mRTorrentTotalUpload", "5242880");
        put("mRTorrentTotalDownload", "1024000");
        put("mRTorrentEmulation", "qBittorrent 4.6.7");
        put("mRTorrentProxyType", "3");
        put("mRTorrentProxyHost", "127.0.0.1");
        put("mRTorrentProxyPort", "8080");
        put("mRTorrentStopRatioEnabled", "true");
        put("mRTorrentStopRatioValue", "1.5");
        let data = bencode::encode(&Value::Dict(d)).unwrap();
        let r = parse_mr_save("x.mRSave", 0, &data).unwrap();
        assert_eq!(r.name, "测试种子");
        assert_eq!(r.info_hash_hex, "0000a4a369c52586c9b091c7f15d4a700aa4123c");
        assert_eq!(r.total_upload, 5242880);
        assert_eq!(r.emulation, "qBittorrent 4.6.7");
        assert_eq!(r.stop_ratio, Some(1.5));
        let (client, version) = parse_emulation_name(&r.emulation);
        assert_eq!(client.as_deref(), Some("qbittorrent"));
        assert_eq!(version.as_deref(), Some("4.6.7"));
        assert_eq!(
            proxy_url_from_record(&r).as_deref(),
            Some("http://127.0.0.1:8080")
        );
    }

    #[test]
    fn real_saves_if_present() {
        let dirs = vec![
            std::path::PathBuf::from("../../mRatioTorrents"),
            std::path::PathBuf::from("../../../mRatioTorrents"),
            std::path::PathBuf::from("g:/Personal/Desktop/mRatio/mRatioTorrents"),
        ];
        let dir = match dirs.iter().find(|d| d.is_dir()) {
            Some(d) => d.clone(),
            None => return,
        };
        let (ok, failed) = load_mr_save_dir(&dir);
        println!("历史存档解析成功 {} 个,失败 {} 个", ok.len(), failed.len());
        for (name, err) in failed.iter().take(5) {
            println!("  失败: {name}: {err}");
        }
        assert!(!ok.is_empty());
    }
}
