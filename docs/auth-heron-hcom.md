# Hehel Zip × Heron × h-com Auth

## Flow

1. Desktop binds `127.0.0.1:0`, opens `{HERON_AUTH_URL}/login?return_to=http://127.0.0.1:{port}/callback?state=…`
2. Heron redirects with `#access_token=…`; callback HTML POSTs token to `/finish`
3. Desktop calls `POST {HCOM_API}/api/auth/heron/exchange` with `X-Client-App: hehel-zip`
4. Session token stored in OS keychain (`service=hehel-zip`, `user=hcom_session`)
5. API calls use `Authorization: Bearer {sessionToken}`

## Session TTL

Desktop sessions (`X-Client-App: hehel-zip`): **72h** (3 days).

## Guards

- `InviteSessionMiddleware` accepts `x-session-token` or `Authorization: Bearer`
- `HybridAuthGuard` requires valid invite session

## Heron allowlist

Loopback regex (PR-0b):

```
^http://127\.0\.0\.1:\d+/callback$
```

Host must be exactly `127.0.0.1` (ASCII).
