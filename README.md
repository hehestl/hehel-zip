# Hehel Zip

[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB)](https://react.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**EN** · Desktop archive manager (RAR / ZIP / 7z / `.hehe`) with 3D-print production workflow statuses. Part of the [Hehestl](https://hehestl.com) ecosystem.

**RU** · Десктопное приложение (Tauri v2) для просмотра и распаковки архивов **RAR / ZIP / 7z / .hehe** с колонкой статусов производственного workflow (3D-печать).

| | |
|---|---|
| **UI languages** | Русский (default), English — Settings → Language |
| **Platforms** | Windows (primary), Linux & macOS (build supported) |
| **Data** | `%APPDATA%\Hehel-Zip\data.db` (Windows) |

---

## Stack / Стек

| Layer | Technology |
|-------|------------|
| Desktop shell | [Tauri v2](https://tauri.app/) (Rust) |
| UI | React 19, TypeScript, Vite 6, Tailwind CSS 3 |
| State | TanStack Query, TanStack Virtual |
| Local DB | SQLite (`rusqlite` + `r2d2` pool) |
| Archives | Native ZIP (`zip` crate), 7-Zip CLI (RAR/7z), custom `.hehe` (zstd) |
| Auth / sync | Heron OAuth → OS keychain, Hestia REST API |
| Tests | Vitest, `cargo test`, Playwright (e2e) |

---

## Features / Возможности

- WinRAR-like archive browser, drag-and-drop
- Extract selected / all files; drag-out STL/OBJ to Explorer
- Custom workflow statuses (SQLite)
- `.hehe` archive creation (zstd presets: fast / balanced / ultra)
- Image gallery with virtual grid
- Optional Hestia cloud sync (Heron login)

- Просмотр архива в стиле WinRAR, drag-and-drop
- Извлечение выделенных / всех файлов; drag-out STL/OBJ
- Настраиваемые статусы workflow (SQLite)
- Создание архивов `.hehe` (пресеты zstd: быстро / баланс / ultra)
- Галерея изображений с виртуальной сеткой
- Опциональная синхронизация с Hestia (логин Heron)

---

## Requirements / Требования

### All platforms

- **Node.js** 20+
- **Rust** stable ([rustup](https://rustup.rs/))
- **7-Zip** — [download](https://www.7-zip.org/) (dev + `npm run copy:7z`)

### Windows

- **WebView2 Runtime** (usually pre-installed on Windows 10/11)
- Visual Studio Build Tools (C++ workload) for `cargo`

### Linux

- `webkit2gtk-4.1`, `libayatana-appindicator3`, `librsvg2`, `patchelf`
- Debian/Ubuntu example:

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

### macOS

- Xcode Command Line Tools: `xcode-select --install`

---

## Quick start / Быстрый старт

```bash
git clone https://github.com/hehestl/hehel-zip.git
cd hehel-zip
npm install
npm run copy:7z    # Windows: copies 7z.exe into src-tauri/resources/7z/
npm run tauri:dev
```

```bash
# RU: то же самое — клонируйте, установите зависимости, скопируйте 7z, запустите dev
```

Copy [`.env.example`](.env.example) to `.env.local` only if you need local overrides. **No secrets are required** for local dev.

---

## Build / Сборка

### Windows (NSIS installer)

```powershell
npm install
npm run copy:7z
npm run tauri:build
```

Output: `src-tauri\target\release\bundle\nsis\Hehel Zip_*.exe`

### Linux

```bash
npm install
# Place 7z binary for your distro or build without RAR/7z native extract
npm run tauri:build
```

Output: `src-tauri/target/release/bundle/deb/` or `appimage/` (depends on Tauri targets in `tauri.conf.json`).

Add Linux bundle targets in `src-tauri/tauri.conf.json` if needed:

```json
"bundle": { "targets": ["deb", "appimage"] }
```

### macOS

```bash
npm install
npm run tauri:build
```

Output: `src-tauri/target/release/bundle/dmg/` or `.app` in `macos/`.

> **Note:** Default `tauri.conf.json` targets **NSIS (Windows)**. Adjust `bundle.targets` per platform before release builds.

---

## Tests / Тесты

```bash
npm run test          # Vitest (frontend)
npm run test:rust     # cargo test (backend)
npm run test:e2e      # Playwright (optional)
```

---

## Configuration / Конфигурация

| Item | Location |
|------|----------|
| SQLite DB | `%APPDATA%\Hehel-Zip\data.db` (Win), `~/Library/Application Support/Hehel-Zip/` (macOS), `~/.local/share/Hehel-Zip/` (Linux) |
| Extract cache | `%LOCALAPPDATA%\Hehel-Zip\extract-cache\` |
| OAuth session | OS keychain (service `hehel-zip`) — **never stored in repo** |
| Sync URLs | UI → Sync → Hestia settings |

---

## Security / Безопасность

- No API keys or tokens in the repository
- Heron `accessToken` is written to the **OS credential store** at runtime
- `.env` and `*.local` are gitignored
- Before publishing: `git grep -i "password\|secret\|api_key"` — should return only docs/tests

---

## Third-party / Сторонние компоненты

- [7-Zip](https://www.7-zip.org/license.txt) — RAR/7z extraction via redistributable `7z.exe` / `7z.dll` (Windows)
- [Tauri](https://tauri.app/), [React](https://react.dev/), [zip](https://crates.io/crates/zip), [zstd](https://github.com/facebook/zstd)

---

## Docs

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [CHANGELOG.md](CHANGELOG.md)
- [docs/perf-baseline.md](docs/perf-baseline.md)

---

## License

[MIT](LICENSE) — © 2026 Hehestl