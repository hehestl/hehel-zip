# Performance baseline — Hehel Zip

Зафиксировать **до** сравнения ZipBackend vs `7z.exe` (F3) и перед F6 scale work.

## Метрики

| Метрика | Сценарий | Как снять |
|---------|----------|-----------|
| List time | ZIP 10k entries | `cargo test -p hehel-zip zip_list_baseline -- --nocapture` |
| Extract time | 500 STL drag-out | ручной: hover+drag batch в UI, замер в DevTools |
| Peak RAM | extract 285 MB `.hehe` | Task Manager / `tracing` span |
| Alloc count | optional | `dhat` feature (dev only) |

## Win-критерий F3

Zip crate быстрее `7z x` на **30–50%** для обычных ZIP без encryption/split → primary backend.

## Fallback log

При откате на `7z.exe`:

```
info!(backend="sevenz-fallback", reason=%reason)
```

(после F6 tracing в `ZipBackend::extract_entries`)

## RAR/7Z extract cache (0.3.1+)

| Сценарий | Win-критерий (release, SSD cache dir) |
|----------|--------------------------------------|
| 1× STL ~300 МБ из `.rar` cold drag | нет полного буфера в RAM; время ≤ native `7z x` + 10% |
| Повторный drag того же STL | cache hit < 200 ms (hard link) |
| 5× STL batch из `.7z` | 1× `7z x -mmt=on`, не 5× `7z -so` |

`tracing` при populate cache: `cache_populate`, `ensure_cached_entry` (`backend`, `elapsed_ms`, `cache_hit`).

Тест: `sevenz_extract_to_path_writes_file` в `seven_zip.rs`.

## Sprint 3 (0.3.3) — preview & grid

| Изменение | Эффект |
|-----------|--------|
| `PreviewCache::get` → `Arc<[u8]>` | Нет clone 100MB thumb в RAM |
| Disk thumb cache (`thumb-cache/*.webp`) | Повторный open архива — hit с диска |
| Virtual `ImageGridView` + prefetch queue (4) | 500+ картинок без DOM-лавины |
| Extract cache `lru::LruCache` O(1) | Evict без sort O(n log n) |
| Zip handle reuse (`zip_handle_cache`) | List/read без reopen ZIP |
| `r2d2` pool SQLite (4 conn) | Параллельные IPC без global Mutex |

Автотесты: `thumb_roundtrip`, extract_cache, `zip_list_paginated_returns_slice`.

## Sprint 2 (0.3.2) — scale 10k

| Изменение | Эффект |
|-----------|--------|
| `list_paginated` Zip/HEHE — slice без full scan | Первая страница 5k без полного IPC |
| 7z `l -ba` + cache на первом paginated | List RAR/7z ~1.5–2× vs `-slt` |
| Frontend chunked load + virtual table always | UI отзывчив на 10k+ строк |
| `rayon` parallel cache populate (4 threads) | Batch warm/drag ~2× на multi-miss |
| `opt-level=3`, `lto=thin` | Release hot paths +10–15% |

Автотесты: `zip_list_paginated_returns_slice`, `parse_ba_extracts_file_and_folder`, `fetchAllArchiveEntries`.

## Sprint 1 (0.3.1) — observability spans

| Span | Поля | Когда |
|------|------|-------|
| `archive_open_probe` | `elapsed_ms`, `is_hehe`, `has_hehestl` | open, blocking pool |
| `archive_open_finalize` | path | SQLite register/sidecar |
| `archive_list` | `elapsed_ms`, `count`, `cache_hit` | list entries |
| `read_hehestl` | `elapsed_ms`, `found` | без full list для ZIP/RAR |
| `archive_extract` | `elapsed_ms`, `written` | extract_archive |
| `preview_bytes` / `preview_uri` | `elapsed_ms`, `cache_hit` | превью |

Проверка: `RUST_LOG=info npm run tauri:dev` → open архив → в консоли `archive_list cache_hit=false`, повторный list в той же сессии → `cache_hit=true`.

Автотесты Sprint 1:
- `read_hehestl_reads_metadata_without_listing`
- `listing_cache_avoids_second_backend_list`

## Baseline snapshot (0.3.0-dev)

| Backend | List 10k | Single STL extract |
|---------|----------|-------------------|
| 7z.exe | TBD | TBD |
| zip-native | TBD | TBD |

Обновить таблицу после первого прогона на CI/dev machine (release build).

## Команды

```powershell
cd f:\-_-hehe-ecosystem\hehel-zip
node scripts/with-cargo-path.mjs cargo test --manifest-path src-tauri/Cargo.toml zip_backend_lists_and_extracts -- --nocapture
npm run test:rust
```

## Environment

- Windows 10/11, release build для production numbers
- `RUST_LOG=info` для tracing spans
