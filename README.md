<div align="center">
  <img src="rustatio-desktop/icons/icon.png" alt="Rustatio" width="110">
</div>

# Rustatio (Chinese Merge Edition)

A modern BitTorrent ratio management tool — a Chinese-localized fork of [Rustatio](https://github.com/takitsu21/rustatio) (MIT) merged with the best features reverse-engineered from [mRatio](https://www.sb-innovation.de).

Manage your share ratio by emulating popular torrent clients (qBittorrent / uTorrent / Transmission / Deluge / BitTorrent): custom upload/download rates, seeding or downloading state, proxy support, stop conditions, and more.

> [!IMPORTANT]
> This tool is for **educational purposes only**. Faking upload/download statistics on private trackers may violate their terms of service and could result in an account ban. Use at your own risk.

## ✨ Features

- **🎭 Client emulation** — qBittorrent / uTorrent / Transmission / Deluge / BitTorrent with selectable versions
- **📁 mRatioClients profiles** — auto-loads `mRatioClients/*.mRClient` (39 client profiles with exact peer_id / key patterns)
- **📜 History import** — automatically restores your previous torrent instances from `mRatioTorrents/*.mRSave` on startup
- **🌐 Proxy support** — SOCKS5 / HTTP with username & password, one-click proxy test (shows exit IP), apply to the current instance or all instances, live status indicator
- **🌱 Seeding / Downloading toggle** — switch any torrent between "Seeding (100% complete)" and "Downloading (custom completion)" right from the torrent card
- **🔗 Site shortcut** — derives the torrent's website from its tracker; click to open and log in
- **📡 Live announce feedback** — while running, shows "Announce OK · #N · Seeders S / Leechers L" so you know it is actually working
- **💾 Auto-save** — every setting change is persisted instantly and fully restored on next launch (with a visible "saved" indicator)
- **🧹 Batch management** — grid view with multi-select bulk actions, bulk import, one-click blank-instance cleanup
- **👁 Watch folder** — drop `.torrent` files into a folder to auto-create instances (auto-start optional)
- **🎲 Realistic behavior** — rate randomization, progressive rates, randomized stop ratio, idle detection (no leechers / no seeders)
- **🛑 Stop conditions** — target ratio / max uploaded / max downloaded / seed time, with post-stop actions
- **📝 Debug log** — records every feature usage and error for easy troubleshooting (see FAQ)
- **🈶 Chinese UI** — the interface is fully localized in Chinese (settings, presets, dialogs, error messages)

## 📥 Download

Grab the latest build from the [Releases](https://github.com/zhengwuji/bt-Ratio-Faker/releases) page:

- `Rustatio-vX.Y-win64.exe` — Windows 64-bit, single portable file, no installation needed

Every release ships with Chinese release notes. The window title shows the build number (e.g. `Rustatio v11`) so you can always tell which version you are running.

## 🚀 Quick Start

1. **Pick a torrent** — click【更换】on the torrent card or drag & drop a `.torrent` file
2. **Pick the state** — choose【做种】(seeding, 100% complete) or【下载中】(downloading, custom completion) on the run-state card
3. **Proxy (optional)** — in the proxy card select scheme/host/port (e.g. SOCKS5 / 127.0.0.1 / 11111) → click the test button to verify → click apply-to-current-instance
4. **Set rates** — upload/download rates in KB/s; keep randomization on and avoid round numbers
5. **Start** — hit the green ▶ button; when the status bar shows "Announce OK" you are up and running
6. **Stop conditions (optional)** — set a target ratio / upload cap, etc. The instance stops automatically when reached

### FAQ

- **"Tracker rejected the current port (blacklisted)"** — change the instance port from 6881 to a high port (e.g. 51413) and restart
- **"Tracker unavailable, retrying in N s"** — network issue or tracker outage; if you use a proxy, make sure it works and is applied to this instance
- **Logs** — `%APPDATA%\rustatio\rustatio-debug.log` records every feature usage and error

## 🔨 Building from Source

Requirements: Rust (stable) + Node.js 20 + wasm-pack

```bash
# 1. Build the WASM module
cd rustatio-wasm
wasm-pack build --target web --out-dir ../ui/src/lib/wasm --release

# 2. Build the frontend
cd ../ui
npm ci
npx vite build

# 3. Build the desktop app (output: target/release/rustatio-desktop.exe)
cd ..
cargo build --release -p rustatio
```

On Windows you can also just run `scripts/重新编译.bat` for a one-click build.

### CI (Automatic Builds)

- Every push to `main` (except README-only changes) automatically builds the Windows binary via GitHub Actions — download it from the Actions page
- Pushing a `v*` tag automatically builds and publishes a GitHub Release with Chinese release notes:

```bash
git tag -a v12 -m "Chinese release notes go here"
git push origin v12
```

## 📋 Changelog

| Version | Highlights |
| --- | --- |
| v11 | Removed outbound links to the upstream repo; added visible auto-save indicator (all settings persist across restarts) |
| v10 | Live "Announce OK" feedback while running; fixed proxy inputs being cleared |
| v9 | Tracker rejection reasons surfaced in Chinese (port blacklisting, etc.); default port changed to 51413 |
| v8 | Seeding / Downloading toggle on the torrent card |
| v7 | State-mode selector (seeding vs downloading); green full-progress bar in seeding mode |
| v6 | Fixed scrape failures being misreported as "Tracker unavailable"; proxy card gained apply-to-current / clear-proxy / status line |
| v5 | Torrent website moved into a prominent clickable banner below the card |
| v4 | Torrent card shows the tracker's website with one-click open |
| v3 | Full Chinese localization (preset cards, detection tips, grid view, all dialogs) |
| v2 | Apply-proxy-to-all-instances; vN build number in the window title |
| v1 | First Chinese merge: mRatioClients profiles, history import, blank-instance cleanup, debug log, proxy test, update prompt removed |

## 🙏 Credits

- [takitsu21/Rustatio](https://github.com/takitsu21/rustatio) — upstream project (MIT License)
- [mRatio](https://www.sb-innovation.de) — source of the emulation profiles and pattern templates
- Built with [Tauri](https://tauri.app/), [Svelte 5](https://svelte.dev/), and [Tailwind CSS](https://tailwindcss.com/)

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

<div align="center">
  <img src="screenshots/light-theme.png" alt="Standard view" width="45%">
  &nbsp;
  <img src="screenshots/dark-theme.png" alt="Dark theme" width="45%">
</div>
