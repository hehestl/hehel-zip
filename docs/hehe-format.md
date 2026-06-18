# HEHE archive format v1

Native container for Hehel Zip. Magic `HEHE`, little-endian.

## Header (32 bytes)

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | Magic `HEHE` |
| 4 | 2 | Version `1` |
| 6 | 2 | Flags (reserved) |
| 8 | 8 | TOC offset |
| 16 | 4 | TOC entry count |
| 20 | 12 | Reserved |

## Compression methods

| ID | Name |
|----|------|
| 0 | store |
| 1 | deflate (optional) |
| 2 | zstd level 12 (balanced preset default) |

## TOC entry

Repeated `toc_count` times at `toc_offset`:

- `path_len` u16
- `path` UTF-8
- `method` u8
- `crc32` u32
- `comp_size` u64
- `raw_size` u64
- `data_offset` u64

Data blobs are stored before TOC, referenced by `data_offset`.

## Conventions

- App-created archives include `metadata.hehestl` with `FormatVersion: 1`, `ArchiveId: {uuid-v4}`, `Created`, `Compression: zstd:12`
- `metadata.hehestl` is always TOC entry index 0; other paths sorted lexicographically
- Single-writer v1; no streaming to socket

## Creation guarantees (v1)

- **Atomicity:** write to `{stem}.hehe.tmp` → per-entry verify → `rename` to `.hehe` (with retry on Windows AV locks)
- **Deterministic TOC:** `metadata.hehestl` first; remaining entries sorted lexicographically
- **Valid metadata:** BOM-safe parse; user fields preserved; system fields refreshed on create
- **Per-entry verify on create:** CRC32 (on read) + raw size + first 64 bytes
- **Default compression:** balanced `zstd:12`; presets fast/ultra — см. [`compression.md`](compression.md)

## Edit policy (v2)

Metadata size change requires full archive rebuild.
