# Changelog

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/).

## [0.3.0] - 2026-06-17

### Added

- Парсер `metadata.hehestl` v2: двуязычные поля `|`, O/S с размерами, inline-ссылки `Слово (url)`; UI масштабов и кликабельных ссылок без сырого URL
- Пресеты сжатия `.hehe`: Fast / Balanced / Ultra (zstd:3 / :12 / :19+w27); STL/OBJ сжимаются при выгоде; `docs/compression.md`
- Опция «JPG/PNG → WebP (lossless)» при создании `.hehe` — без потери качества; замена только если WebP меньше оригинала

- `ArchiveBackend` trait, `ArchiveService`, registry probe (magic + extension)
- Zip Slip guard (`path_safety.rs`, camino, Windows reserved names)
- `ExtractResult` с partial skip для небезопасных путей
- `ZipBackend` (crate `zip`, feature `zip-native`) с fallback на `7z.exe`
- `list_archive_entries_paginated` IPC
- TanStack Query + лёгкий FSD (`shared/api`, `features/archive`, `entities/file-entry`)
- Tauri event `hehel:status-changed` + invalidation queries
- Virtual Session reuse по `archive_hash` (TTL 30 min)
- `@tanstack/react-virtual` в таблице (>200 строк)
- WebP thumbnails (max 256px) для превью изображений
- `tracing` + `tracing-subscriber`, `ARCHITECTURE.md`, `docs/perf-baseline.md`
- Playwright smoke (`npm run test:e2e`)

### Changed

- `AppState`: `ArchiveService` вместо `CompositeArchiveAdapter`
- `ArchiveEntryDto`: `Hash`/`Eq` для TanStack Query keys

## [0.2.1] - 2026-06-17

### Added

- Extract cache: SHA256 archive key, hard_link→copy, LRU ~5GB
- Targeted HEHE extract по TOC `by_path`, `METHOD_STORE` для STL/OBJ
- Warm extract cache на hover STL/OBJ (debounce 400ms)
- Настройка каталога extract cache в меню

### Fixed

- Shift+drag grip: cascade close дочерних окон через `WebviewWindow.getAll()`
- Scrollbars в layout areas

## [0.2.0] - 2026-06-17

### Performance

- HEHE extract: стриминг zstd/deflate на диск без полного `Vec` в RAM
- TOC cache: парсинг оглавления архива один раз на файл
- 7z extract: флаг `-mmt=on` для многопоточной распаковки
- `extract_to_session` вынесен в `spawn_blocking`
- PreviewCache: LRU (до 100 МБ) вместо FIFO
- Галерея: превью через `hehe://`, prefetch соседних кадров, UI-кэш

### UX Improvements

- Меню «Файл», «Настройки», «Синхронизация» вместо ряда кнопок
- «Только STL/OBJ» перенесён в Настройки (persist в `localStorage`)
- Status bar: счётчики изображений / моделей / файлов в текущей папке
- Галерея: режим «Сетка» + fullscreen с навигацией
- Версия приложения в правой части меню

### Fixed

- Уничтожение дочерних окон (`WindowManager`, cascade при закрытии main)
- Ctrl+W на единственной вкладке закрывает окно
- Drag-out из secondary window (передаётся `window_label`)

### Added

- Визуальный drag-out STL (v1.1): призрак, overlay, иконка Hehel при drag

## [0.1.0] - 2026-06-16

- Просмотр архивов `.hehe`, ZIP, RAR, 7z
- Создание `.hehe`, drag-out STL, статусы печати
