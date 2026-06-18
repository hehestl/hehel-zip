# Hehel Zip — Cloud Sync Manifest (SSOT)

Contract gate for PR-1+. Version: RFC v6.

## A. Threat model

| Actor | Capability | Mitigation |
|-------|------------|------------|
| Local attacker | Port squat; file read | `127.0.0.1:0`; listener TTL 5m; IPC workspace boundary |
| Network attacker | MITM, DoS, CSRF | HTTPS; PKCE; CORS allowlist; rate limits |
| Malicious guest | POST spam | 403 + audit log |
| Compromised desktop | Stolen keychain token | 72h session TTL; 401 wipe |
| Compromised keychain | Session abuse | Re-login; v2 rotation |

## B. Security invariants

| ID | Invariant |
|----|-----------|
| SEC-1 | PKCE required on OAuth login |
| SEC-2 | `state`: 32+ byte CSPRNG, constant-time compare, in-memory only |
| SEC-2a | `state` + listener TTL = 5 min |
| SEC-3 | Loopback `127.0.0.1` only; shutdown after first valid callback |
| SEC-4 | Session token never in SQLite/plaintext/localStorage |
| SEC-5 | OS keychain primary; v1 fail-closed |
| SEC-5a | UI «Ожидание подтверждения ОС» during keyring ops |
| SEC-6 | All `/sync`: server-side membership/role |
| SEC-7 | Tokens redacted in logs |
| SEC-8 | Desktop HTTPS only |
| SEC-9 | IPC-1 typed args, path canonicalize |
| SEC-10 | Session max 72h |
| SEC-11 | Guest 403 POST logged; guest rate 10/min/IP |
| SEC-12 | No alternative token storage in v1 |
| SEC-13 | POST sync: `application/json` only |
| SEC-14 | CSP: loopback only in Tauri desktop |
| SEC-15 | `/sync` CORS explicit allowlist |

## C. Sync invariants

| ID | Invariant |
|----|-----------|
| SYNC-1 | Server `updated_at = NOW()` on writes |
| SYNC-2 | Tombstone single txn + reassign links → Unassigned |
| SYNC-3 | No orphan links |
| SYNC-4 | `init-board` idempotent UUID v5 |
| SYNC-5 | Unassigned non-deletable |
| SYNC-6 | Defaults via `?force=1` only |
| SYNC-7 | Entries unique by normalized path |
| SYNC-8 | Pull merge: server timestamps only |
| SYNC-9 | Tombstone GC 90 days |
| SYNC-10 | `clientRequestId` max 64 B, TTL 24h |
| SYNC-11 | POSIX path normalize before dedup |
| SYNC-12 | `manifestHash` server-computed only |
| SYNC-13 | Link upsert rejects tombstoned status |

## D. Normative `manifestDigest`

Do **not** hash canonical JSON.

```
line = "{normalizedPath}|{sizeBytes}|{isDirFlag}"
manifestDigest = sort(lines, UTF-8 lex).join("\n")
manifestHash = hex(sha256(utf8(manifestDigest + "\x1e" + HCOM_MANIFEST_HASH_PEPPER)))
```

| Field | Rule |
|-------|------|
| path | POSIX normalized, UTF-8 NFC |
| sizeBytes | Decimal integer `>= 0` |
| isDirFlag | `0` or `1` |

### Golden vector 1

```json
[
  { "path": "models/part.stl", "sizeBytes": 4096, "isDir": false },
  { "path": "readme.txt", "sizeBytes": 128, "isDir": false }
]
```

```
manifestDigest = "models/part.stl|4096|0\nreadme.txt|128|0"
sha256(manifestDigest) = 0a4409b0e4d4780c688b7883fe8809deac0266df1ccbfbefce8f0dbb624fef13
```

With pepper `test-pepper-v0` (dev tests only):

```
manifestHash = 077631eb9265a1c04b4cf72420eccc8434dd1f79ad3e5b4a04fb3e961a088a66
```

### Golden vector 2 (path normalize)

Input paths `./a`, `a/`, `a` → single line `a|512|0` after SYNC-11.

## E. API endpoints

Base: `/api/hehel/projects/:projectId`

| Method | Path | Description |
|--------|------|-------------|
| POST | `/archives/init-board` | Idempotent default columns + Unassigned |
| GET | `/archives` | Light list + manifestHash/version |
| GET | `/archives/:archiveId/manifest` | Full entries; `304` if `If-None-Match` |
| GET | `/archives/:archiveId/board` | Aggregated kanban |
| POST | `/archives/sync` | gzip JSON manifest + links; requires `clientRequestId` |
| POST | `/workflow-statuses/sync` | Batch status upsert/tombstone |
| GET | `/workflow-statuses` | Active statuses (no tombstones) |

Auth: `Authorization: Bearer {sessionToken}` or `x-session-token`.

## F. Failure handling

See plan RFC v6 section F (OAuth TTL, gzip ratio >100:1, partial sync retry, etc.).

## G. Logging

Tokens `[REDACTED]`; guest 403 structured JSON; max line 8 KB; retention 30d/90d.

## H. Testing gate

- manifestDigest golden Rust ↔ NestJS
- Loopback TTL + wrong state
- Tombstone cascade + SYNC-13 concurrent sync
- gzip bomb ratio abort
- SQLite INTEGER→UUID migration fixture
