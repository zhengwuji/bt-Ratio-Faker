//! mRatio 伪装客户端档案(.mRClient)解密与解析。
//!
//! 算法逆向自 mRatio 5.4.1 的 ClientEmulationLoader/ClientEmulationProfile
//! (详见 ../源码/文档/架构总览.md):
//! 1. 外层是 bencode 字典:a=作者、e=名称、key=密钥串、v=最低 mRatio 版本(可选)、
//!    zdata=3DES 加密的内层 bencode
//! 2. 3DES 密钥 = MD5(a+e+key[+v]) 的十六进制小写字符串前 24 个字符(ASCII)
//! 3. 3DES-CBC-PKCS7,IV 为固定字节(取 9 字节常量的前 8 字节,.NET 行为等价)
//! 4. 内层字典字段:Emulation(显示名)、PI_RegExp(peer_id 模板)、
//!    K_RegExp/K_Size/K_BaseChars(key 模板)等

use crate::protocol::bencode;
use thiserror::Error;
use base64::Engine;
use cipher::{BlockDecryptMut, BlockEncryptMut};
use cipher::{KeyIvInit, block_padding::Pkcs7};
use md5::Digest;
use serde::Serialize;

type TdesCbcDec = cbc::Decryptor<des::TdesEde3>;
type TdesCbcEnc = cbc::Encryptor<des::TdesEde3>;

/// 无 v 字段时的固定 IV(mRatio 常量前 8 字节)
const IV_NO_VERSION: [u8; 8] = [24, 98, 211, 60, 41, 0, 37, 251];
/// 有 v 字段时的固定 IV
const IV_WITH_VERSION: [u8; 8] = [210, 188, 25, 132, 160, 213, 153, 70];

#[derive(Debug, Error)]
pub enum MrClientError {
    #[error("bencode: {0}")]
    Bencode(#[from] bencode::BencodeError),
    #[error("missing field: {0}")]
    MissingField(&'static str),
    #[error("crypto: {0}")]
    Crypto(String),
    #[error("inner data is not a dictionary")]
    NotADictionary,
}

impl MrClientError {
    fn crypto(e: impl std::fmt::Display) -> Self {
        Self::Crypto(e.to_string())
    }
}

/// 一个 .mRClient 伪装档案
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MrClientProfile {
    /// 文件名(不含扩展名)
    pub file_name: String,
    /// 显示名(内层 Emulation 字段)
    pub name: String,
    /// 作者(外层 a 字段)
    pub author: String,
    /// peer_id 正则模板(PI_RegExp)
    pub peer_id_pattern: String,
    /// key 正则模板(K_RegExp;无则由 K_BaseChars+K_Size 合成)
    pub key_pattern: Option<String>,
    /// 要求的最低 mRatio 版本(外层 v 字段,如 "5.0.0")
    pub min_version: Option<String>,
    /// announce 查询参数模板(Announce 字段,占位符 [HASH][PEERID][KEY][PORT][EVENT][UPLOAD][DOWNLOAD][LEFT][NUMWANT])
    pub announce_query_template: Option<String>,
    /// 上传报告口径(ReportUploadAs:0=字节,1=按 piece 取整,2=按 16KB 取整)
    pub report_upload_as: i8,
    /// 下载报告口径(ReportDownloadAs)
    pub report_download_as: i8,
    /// 剩余量报告口径(ReportLeftAs)
    pub report_left_as: i8,
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn md5_hex_lower(data: &[u8]) -> String {
    let digest = md5::Md5::digest(data);
    let mut s = String::with_capacity(32);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// 解密 zdata 并解析内层 bencode 字典
fn decrypt_inner(zdata: &[u8], key: &[u8; 24], iv: &[u8; 8]) -> Result<Vec<u8>, MrClientError> {
    let mut dec = TdesCbcDec::new(key.into(), iv.into());
    dec.decrypt_padded_vec_mut::<Pkcs7>(zdata)
        .map_err(MrClientError::crypto)
}

/// 为测试/工具提供加密能力(与 mRatio 写档格式一致)
#[cfg(test)]
fn encrypt_inner(plain: &[u8], key: &[u8; 24], iv: &[u8; 8]) -> Vec<u8> {
    let mut enc = TdesCbcEnc::new(key.into(), iv.into());
    enc.encrypt_padded_vec_mut::<Pkcs7>(plain)
}

/// 解析 .mRClient 文件内容
pub fn parse_mr_client(file_name: &str, data: &[u8]) -> Result<MrClientProfile, MrClientError> {
    let outer = match bencode::parse(data)? {
        serde_bencode::value::Value::Dict(d) => d,
        _ => return Err(MrClientError::NotADictionary),
    };

    let author = bencode::get_bytes(&outer, "a")
        .map_err(|_| MrClientError::MissingField("a"))?;
    let emu = bencode::get_bytes(&outer, "e")
        .map_err(|_| MrClientError::MissingField("e"))?;
    let key = bencode::get_bytes(&outer, "key")
        .map_err(|_| MrClientError::MissingField("key"))?;
    let zdata = bencode::get_bytes(&outer, "zdata")
        .map_err(|_| MrClientError::MissingField("zdata"))?;
    let min_version = bencode::get_bytes(&outer, "v").ok().map(|v| latin1(&v));

    let (iv, key_material) = match &min_version {
        Some(v) => (
            IV_WITH_VERSION,
            [author.as_slice(), emu.as_slice(), key.as_slice(), v.as_bytes()].concat(),
        ),
        None => (
            IV_NO_VERSION,
            [author.as_slice(), emu.as_slice(), key.as_slice()].concat(),
        ),
    };

    // mRatio:MD5 十六进制小写串的前 24 个字符本身作为 3DES 密钥(ASCII)
    let hex = md5_hex_lower(&key_material);
    let des_key: [u8; 24] = hex[..24]
        .as_bytes()
        .try_into()
        .map_err(|_| MrClientError::Crypto("key length".into()))?;

    let plain = decrypt_inner(&zdata, &des_key, &iv)?;
    let inner = match bencode::parse(&plain)? {
        serde_bencode::value::Value::Dict(d) => d,
        _ => return Err(MrClientError::NotADictionary),
    };

    let name = latin1(&bencode::get_bytes(&inner, "Emulation")
        .map_err(|_| MrClientError::MissingField("Emulation"))?);
    let peer_id_pattern = latin1(
        &bencode::get_bytes(&inner, "PI_RegExp")
            .or_else(|_| bencode::get_bytes(&inner, "PI_Sample"))
            .map_err(|_| MrClientError::MissingField("PI_RegExp"))?,
    );

    // key:优先 K_RegExp;否则由 K_BaseChars + K_Size 合成字符类模板
    let key_pattern = match bencode::get_bytes(&inner, "K_RegExp") {
        Ok(k) if !k.is_empty() => Some(latin1(&k)),
        _ => {
            let base = bencode::get_bytes(&inner, "K_BaseChars").ok();
            let size = bencode::get_int(&inner, "K_Size").unwrap_or(8).clamp(1, 128) as usize;
            let chars: String = match base {
                Some(b) if !b.is_empty() => latin1(&b)
                    .chars()
                    .map(|c| match c {
                        '\\' | ']' | '^' | '-' => format!("\\{c}"),
                        other => other.to_string(),
                    })
                    .collect(),
                _ => "0-9a-f".to_string(),
            };
            Some(format!("[{chars}]{size}"))
        }
    };

    // mRatio announce 模板与报告口径(旧档案可能缺省,退化为内置参数序)
    let announce_query_template = bencode::get_bytes(&inner, "Announce")
        .ok()
        .map(|b| latin1(&b))
        .filter(|t| !t.is_empty());
    let parse_mode = |key: &str| -> i8 {
        bencode::get_bytes(&inner, key)
            .ok()
            .and_then(|b| latin1(&b).trim().parse::<i8>().ok())
            .unwrap_or(0)
    };
    let report_upload_as = parse_mode("ReportUploadAs");
    let report_download_as = parse_mode("ReportDownloadAs");
    let report_left_as = parse_mode("ReportLeftAs");

    Ok(MrClientProfile {
        file_name: file_name.to_string(),
        name,
        author: latin1(&author),
        peer_id_pattern,
        key_pattern,
        min_version,
        announce_query_template,
        report_upload_as,
        report_download_as,
        report_left_as,
    })
}

/// 扫描目录下的 *.mRClient,返回 (成功档案, 失败列表[文件名, 错误])
pub fn load_mr_clients_dir(
    dir: &std::path::Path,
) -> (Vec<MrClientProfile>, Vec<(String, String)>) {
    let mut ok = Vec::new();
    let mut failed = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return (ok, failed),
    };
    let mut files: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mRClient"))
        })
        .collect();
    files.sort();
    for path in files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        match std::fs::read(&path)
            .map_err(|e| MrClientError::Crypto(e.to_string()))
            .and_then(|data| parse_mr_client(&name, &data))
        {
            Ok(p) => ok.push(p),
            Err(e) => failed.push((name, e.to_string())),
        }
    }
    (ok, failed)
}

/// Rustatio 配置里的 mrClientsDir / 历史目录推断:
/// 在 exe 所在目录、其上一级、上两级查找(适配 target/release 与免安装布局)
pub fn candidate_client_dirs() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("mRatioClients"));
            if let Some(p1) = dir.parent() {
                out.push(p1.join("mRatioClients"));
                if let Some(p2) = p1.parent() {
                    out.push(p2.join("mRatioClients"));
                }
            }
        }
    }
    out
}

/// base64(info-hash) -> 20 字节(.mRSave 的 mRTorrentHash 字段)
pub fn base64_info_hash(s: &str) -> Option<[u8; 20]> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(s.trim()).ok()?;
    bytes.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_bencode::value::Value;
    use std::collections::HashMap;

    fn build_fake_file(min_version: Option<&str>) -> Vec<u8> {
        // 构造内层字典
        let mut inner: HashMap<Vec<u8>, Value> = HashMap::new();
        inner.insert(b"Emulation".to_vec(), Value::Bytes(b"Test Client 1.0".to_vec()));
        inner.insert(b"PI_RegExp".to_vec(), Value::Bytes(b"-UT3550-[\\w]{12}".to_vec()));
        inner.insert(b"K_BaseChars".to_vec(), Value::Bytes(b"abcdef0123456789".to_vec()));
        inner.insert(b"K_Size".to_vec(), Value::Bytes(b"8".to_vec()));
        let inner_bytes = bencode::encode(&Value::Dict(inner)).unwrap();

        // 派生密钥并加密
        let a = b"The248";
        let e = b"Test 1.0";
        let k = b"testkey123";
        let (iv, material) = match min_version {
            Some(v) => (IV_WITH_VERSION, [a.as_slice(), e.as_slice(), k.as_slice(), v.as_bytes()].concat()),
            None => (IV_NO_VERSION, [a.as_slice(), e.as_slice(), k.as_slice()].concat()),
        };
        let hex = md5_hex_lower(&material);
        let des_key: [u8; 24] = hex[..24].as_bytes().try_into().unwrap();
        let zdata = encrypt_inner(&inner_bytes, &des_key, &iv);

        // 外层字典
        let mut outer: HashMap<Vec<u8>, Value> = HashMap::new();
        outer.insert(b"a".to_vec(), Value::Bytes(a.to_vec()));
        outer.insert(b"e".to_vec(), Value::Bytes(e.to_vec()));
        outer.insert(b"key".to_vec(), Value::Bytes(k.to_vec()));
        outer.insert(b"zdata".to_vec(), Value::Bytes(zdata));
        if let Some(v) = min_version {
            outer.insert(b"v".to_vec(), Value::Bytes(v.as_bytes().to_vec()));
        }
        bencode::encode(&Value::Dict(outer)).unwrap()
    }

    #[test]
    fn round_trip_without_version() {
        let data = build_fake_file(None);
        let p = parse_mr_client("Test.mRClient", &data).unwrap();
        assert_eq!(p.name, "Test Client 1.0");
        assert_eq!(p.author, "The248");
        assert_eq!(p.peer_id_pattern, "-UT3550-[\\w]{12}");
        assert_eq!(p.key_pattern.as_deref(), Some("[abcdef0123456789]8"));
    }

    #[test]
    fn round_trip_with_version() {
        let data = build_fake_file(Some("5.0.0"));
        let p = parse_mr_client("Test.mRClient", &data).unwrap();
        assert_eq!(p.name, "Test Client 1.0");
        assert_eq!(p.min_version.as_deref(), Some("5.0.0"));
    }

    #[test]
    fn real_files_if_present() {
        // 本机集成验证:若 mRatio 目录存在则对真实档案做全量解析
        let dirs = vec![
            std::path::PathBuf::from("../../mRatioClients"),
            std::path::PathBuf::from("../../../mRatioClients"),
            std::path::PathBuf::from("g:/Personal/Desktop/mRatio/mRatioClients"),
        ];
        let dir = match dirs.iter().find(|d| d.is_dir()) {
            Some(d) => d.clone(),
            None => return,
        };
        let (ok, failed) = load_mr_clients_dir(&dir);
        println!("解析成功 {} 个,失败 {} 个", ok.len(), failed.len());
        for (name, err) in &failed {
            println!("  失败: {name}: {err}");
        }
        assert!(!ok.is_empty(), "至少应成功解析一个真实档案");
        for p in &ok {
            assert!(!p.name.is_empty());
            assert!(!p.peer_id_pattern.is_empty());
        }
    }
}
