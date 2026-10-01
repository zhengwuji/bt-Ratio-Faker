//! Peer 客户端探测:直连标准 BT 握手(明文 + MSE 加密)识别对方真实客户端。
//!
//! 安全性:tracker 完全感知不到 peer 直连,PT 封号依据全部来自 tracker 侧 announce
//! 数据;握手使用本实例自己的 info_hash 与伪装 peer_id(与 announce 指纹一致),
//! 拿到对方握手响应后立即断开,不交换任何 piece/扩展数据。仅由用户手动触发。
//!
//! 连接路径:实例配置了代理时走代理(SOCKS5/HTTP CONNECT),否则直连。
//! 走代理可绕过运营商对明文 BT 握手的 DPI 干扰(实测国内网络直连握手会被黑洞)。

use num_bigint::BigUint;
use rand::Rng;
use sha1::{Digest, Sha1};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const MSE_PRIME_HEX: &str = "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD129024E088A67CC74020BBEA63B139B22514A08798E3404DDEF9519B3CD3A431B302B0A6DF25F14374FE1356D6D51C245E485B576625E7EC6F44C42E9A63A36210000000000090563";

const PLAIN_TIMEOUT: Duration = Duration::from_secs(4);
const MSE_TIMEOUT: Duration = Duration::from_secs(8);

/// 连接隧道:直连 / SOCKS5 / HTTP CONNECT
#[derive(Clone)]
enum Tunnel {
    Direct,
    Socks5(SocketAddr),
    HttpConnect(SocketAddr),
}

fn parse_proxy(proxy_url: Option<&str>) -> Tunnel {
    let Some(url) = proxy_url.map(str::trim).filter(|s| !s.is_empty()) else {
        return Tunnel::Direct;
    };
    // 形如 socks5://host:port 或 http://host:port
    let (scheme, rest) = match url.split_once("://") {
        Some(v) => v,
        None => return Tunnel::Direct,
    };
    let host_port = rest.split('/').next().unwrap_or("");
    let Ok(addr) = host_port.parse::<SocketAddr>() else {
        return Tunnel::Direct;
    };
    match scheme.to_ascii_lowercase().as_str() {
        "socks5" | "socks5h" | "socks" => Tunnel::Socks5(addr),
        "http" | "https" => Tunnel::HttpConnect(addr),
        _ => Tunnel::Direct,
    }
}

async fn connect_target(tunnel: &Tunnel, addr: SocketAddr) -> std::io::Result<TcpStream> {
    match tunnel {
        Tunnel::Direct => TcpStream::connect(addr).await,
        Tunnel::Socks5(proxy) => socks5_connect(*proxy, addr).await,
        Tunnel::HttpConnect(proxy) => http_connect(*proxy, addr).await,
    }
}

/// 最小 SOCKS5 客户端(无认证),CONNECT 到 target
async fn socks5_connect(proxy: SocketAddr, target: SocketAddr) -> std::io::Result<TcpStream> {
    let mut s = TcpStream::connect(proxy).await?;
    s.write_all(&[5, 1, 0]).await?;
    let mut g = [0u8; 2];
    s.read_exact(&mut g).await?;
    if g != [5, 0] {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("代理不支持无认证 SOCKS5(greeting {g:02x?})"),
        ));
    }
    let mut req = vec![5u8, 1, 0];
    match target.ip() {
        IpAddr::V4(v4) => {
            req.push(1);
            req.extend(v4.octets());
        }
        IpAddr::V6(v6) => {
            req.push(4);
            req.extend(v6.octets());
        }
    }
    req.extend(target.port().to_be_bytes());
    s.write_all(&req).await?;
    let mut head = [0u8; 4];
    s.read_exact(&mut head).await?;
    if head[1] != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("代理 CONNECT 失败 code={}", head[1]),
        ));
    }
    let alen = match head[3] {
        1 => 4usize,
        4 => 16usize,
        3 => {
            let mut l = [0u8; 1];
            s.read_exact(&mut l).await?;
            l[0] as usize
        }
        _ => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "代理返回异常地址类型"));
        }
    };
    let mut rest = vec![0u8; alen + 2];
    s.read_exact(&mut rest).await?;
    Ok(s)
}

/// 最小 HTTP CONNECT 隧道
async fn http_connect(proxy: SocketAddr, target: SocketAddr) -> std::io::Result<TcpStream> {
    let mut s = TcpStream::connect(proxy).await?;
    let host = match target.ip() {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => format!("[{v6}]"),
    };
    let req = format!(
        "CONNECT {host}:{} HTTP/1.1\r\nHost: {host}:{}\r\n\r\n",
        target.port(),
        target.port()
    );
    s.write_all(req.as_bytes()).await?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 256];
    loop {
        let n = s.read(&mut chunk).await?;
        if n == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "代理连接被关闭"));
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 8192 {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "代理响应异常"));
        }
    }
    let head = String::from_utf8_lossy(&buf);
    if !head.starts_with("HTTP/") || !head.contains(" 2") {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "代理 CONNECT 被拒绝"));
    }
    Ok(s)
}

fn handshake_bytes(info_hash: &[u8; 20], our_peer_id: &[u8; 20], reserved: &[u8; 8]) -> Vec<u8> {
    let mut hs = Vec::with_capacity(68);
    hs.push(19u8);
    hs.extend_from_slice(b"BitTorrent protocol");
    hs.extend_from_slice(reserved);
    hs.extend_from_slice(info_hash);
    hs.extend_from_slice(our_peer_id);
    hs
}

/// 明文 BT 握手,返回对方 peer_id
async fn plain_handshake(
    stream: &mut TcpStream,
    info_hash: &[u8; 20],
    our_peer_id: &[u8; 20],
) -> Result<[u8; 20], String> {
    stream
        .write_all(&handshake_bytes(info_hash, our_peer_id, &[0u8; 8]))
        .await
        .map_err(|e| format!("发送失败: {e}"))?;
    let mut resp = [0u8; 68];
    stream
        .read_exact(&mut resp)
        .await
        .map_err(|e| format!("无响应({e})"))?;
    if &resp[1..20] != b"BitTorrent protocol" {
        return Err("协议串不匹配".into());
    }
    let mut pid = [0u8; 20];
    pid.copy_from_slice(&resp[48..68]);
    Ok(pid)
}

fn rc4_new(key: &[u8]) -> (Vec<u8>, usize, usize) {
    let mut s: Vec<u8> = (0..=255u8).collect();
    let mut j = 0usize;
    for k in 0..256 {
        j = (j + s[k] as usize + key[k % key.len()] as usize) & 0xff;
        s.swap(k, j);
    }
    (s, 0, 0)
}

fn rc4_apply(state: &mut (Vec<u8>, usize, usize), buf: &mut [u8]) {
    let (s, i, j) = state;
    for b in buf.iter_mut() {
        *i = (*i + 1) & 0xff;
        *j = (*j + s[*i] as usize) & 0xff;
        s.swap(*i, *j);
        *b ^= s[(s[*i] as usize + s[*j] as usize) & 0xff];
    }
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h = Sha1::new();
    h.update(data);
    h.finalize().into()
}

fn pad96(b: &[u8]) -> [u8; 96] {
    let mut o = [0u8; 96];
    o[96 - b.len()..].copy_from_slice(b);
    o
}

/// MSE (BEP-31 / Message Stream Encryption) 发起端握手,返回对方 peer_id。
/// 与 libtorrent/qBittorrent、Transmission 的实现逐字节核对过(含互测通过)。
async fn mse_handshake(
    stream: &mut TcpStream,
    info_hash: &[u8; 20],
    our_peer_id: &[u8; 20],
) -> Result<[u8; 20], String> {
    let p = BigUint::from_bytes_be(&{
        let mut v = Vec::new();
        for k in 0..MSE_PRIME_HEX.len() / 2 {
            v.push(u8::from_str_radix(&MSE_PRIME_HEX[k * 2..k * 2 + 2], 16).unwrap());
        }
        v
    });
    let g = BigUint::from(2u32);

    // 私钥 160 位随机
    let mut x_bytes = [0u8; 20];
    rand::rng().fill(&mut x_bytes);
    let x = BigUint::from_bytes_be(&x_bytes);
    let xa = pad96(&g.modpow(&x, &p).to_bytes_be());
    stream
        .write_all(&xa)
        .await
        .map_err(|e| format!("MSE 发送失败: {e}"))?;

    // 收 XB(96 字节),算共享密钥 S(固定 96 字节大端参与哈希)
    let mut xb = [0u8; 96];
    stream
        .read_exact(&mut xb)
        .await
        .map_err(|e| format!("MSE 收 XB 失败: {e}"))?;
    let s = BigUint::from_bytes_be(&xb).modpow(&x, &p);
    let s_bytes = pad96(&s.to_bytes_be());

    let key_out = sha1(&[b"keyA".as_slice(), &s_bytes, info_hash].concat());
    let key_in = sha1(&[b"keyB".as_slice(), &s_bytes, info_hash].concat());
    let mut rc4_out = rc4_new(&key_out);
    let mut rc4_in = rc4_new(&key_in);
    rc4_apply(&mut rc4_out, &mut vec![0u8; 1024]); // 丢弃首 1024 字节
    rc4_apply(&mut rc4_in, &mut vec![0u8; 1024]);

    // 发送 req1 || (req2^req3) || ENC(VC || crypto_provide(RC4) || len(Pa)=0)
    let req1 = sha1(&[b"req1".as_slice(), &s_bytes].concat());
    let req2 = sha1(&[b"req2".as_slice(), info_hash.as_slice()].concat());
    let req3 = sha1(&[b"req3".as_slice(), &s_bytes].concat());
    let req2x3: Vec<u8> = req2.iter().zip(req3.iter()).map(|(a, b)| a ^ b).collect();

    let mut enc = Vec::new();
    enc.extend_from_slice(&[0u8; 8]);
    enc.extend_from_slice(&2u32.to_be_bytes());
    enc.extend_from_slice(&0u16.to_be_bytes());
    rc4_apply(&mut rc4_out, &mut enc);

    let mut out = Vec::with_capacity(40 + enc.len());
    out.extend_from_slice(&req1);
    out.extend_from_slice(&req2x3);
    out.extend_from_slice(&enc);
    stream
        .write_all(&out)
        .await
        .map_err(|e| format!("MSE 握手发送失败: {e}"))?;

    // 收 ENC(VC || crypto_select || len(Pb) || Pb)
    let mut head = [0u8; 14];
    stream
        .read_exact(&mut head)
        .await
        .map_err(|e| format!("MSE 收响应失败: {e}"))?;
    rc4_apply(&mut rc4_in, &mut head);
    if head[0..8] != [0u8; 8] {
        return Err("MSE VC 不匹配".into());
    }
    let crypto_select = u32::from_be_bytes([head[8], head[9], head[10], head[11]]);
    let pb_len = u16::from_be_bytes([head[12], head[13]]) as usize;
    if pb_len > 512 {
        return Err("MSE Pb 过长".into());
    }
    if pb_len > 0 {
        let mut pb = vec![0u8; pb_len];
        stream.read_exact(&mut pb).await.map_err(|e| e.to_string())?;
        if crypto_select == 2 {
            rc4_apply(&mut rc4_in, &mut pb);
        }
    }

    // 发送我方加密 BT 握手,再收对方握手取 peer_id
    let mut hs = handshake_bytes(info_hash, our_peer_id, &[0u8; 8]);
    rc4_apply(&mut rc4_out, &mut hs);
    stream
        .write_all(&hs)
        .await
        .map_err(|e| format!("MSE BT 握手发送失败: {e}"))?;

    let mut resp = [0u8; 68];
    stream
        .read_exact(&mut resp)
        .await
        .map_err(|e| format!("MSE 收 BT 握手失败: {e}"))?;
    if crypto_select == 2 {
        rc4_apply(&mut rc4_in, &mut resp);
    } else if crypto_select != 1 {
        return Err(format!("MSE 未知 crypto_select: {crypto_select}"));
    }
    if &resp[1..20] != b"BitTorrent protocol" {
        return Err("MSE 协议串不匹配".into());
    }
    let mut pid = [0u8; 20];
    pid.copy_from_slice(&resp[48..68]);
    Ok(pid)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PeerProbeOutcome {
    pub ip: String,
    pub port: u16,
    /// 对方完成握手(端口真实可达、真实在线)
    pub online: bool,
    /// 对方握手响应中的 peer_id
    pub peer_id: Option<String>,
    /// true = 经 MSE 加密握手识别,false = 明文握手
    pub encrypted: bool,
}

pub struct ProbeParams {
    pub targets: Vec<(IpAddr, u16, String)>,
    pub info_hash: [u8; 20],
    pub our_peer_id: [u8; 20],
    pub proxy_url: Option<String>,
}

/// 对一批 peer 探测:先明文握手,失败再用 MSE 加密握手。并发 8。
pub async fn probe_peers(params: ProbeParams) -> Vec<PeerProbeOutcome> {
    let tunnel = parse_proxy(params.proxy_url.as_deref());
    let semaphore = Arc::new(tokio::sync::Semaphore::new(8));
    let mut tasks = Vec::with_capacity(params.targets.len());

    for (ip, port, ip_text) in params.targets {
        let permit = semaphore.clone();
        let info_hash = params.info_hash;
        let our_peer_id = params.our_peer_id;
        let tunnel = tunnel.clone();
        tasks.push(tokio::spawn(async move {
            let _permit = permit.acquire_owned().await;
            let addr = SocketAddr::new(ip, port);

            // 1) 明文握手
            let plain = tokio::time::timeout(PLAIN_TIMEOUT, async {
                let mut stream =
                    connect_target(&tunnel, addr).await.map_err(|e| e.to_string())?;
                plain_handshake(&mut stream, &info_hash, &our_peer_id).await
            })
            .await;
            if let Ok(Ok(pid)) = plain {
                return PeerProbeOutcome {
                    ip: ip_text,
                    port,
                    online: true,
                    peer_id: Some(pid.iter().map(|&b| b as char).collect()),
                    encrypted: false,
                };
            }

            // 2) MSE 加密握手
            let mse = tokio::time::timeout(MSE_TIMEOUT, async {
                let mut stream =
                    connect_target(&tunnel, addr).await.map_err(|e| e.to_string())?;
                mse_handshake(&mut stream, &info_hash, &our_peer_id).await
            })
            .await;
            if let Ok(Ok(pid)) = mse {
                return PeerProbeOutcome {
                    ip: ip_text,
                    port,
                    online: true,
                    peer_id: Some(pid.iter().map(|&b| b as char).collect()),
                    encrypted: true,
                };
            }

            PeerProbeOutcome { ip: ip_text, port, online: false, peer_id: None, encrypted: false }
        }));
    }

    let mut results = Vec::with_capacity(tasks.len());
    for t in tasks {
        if let Ok(r) = t.await {
            results.push(r);
        }
    }
    results
}

use std::sync::Arc;
