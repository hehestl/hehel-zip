# Hehel Zip kanban in Hestia

## UI

Project page tab **Hehel Zip** → [`HehelArchiveBoard`](../../h-com/frontend/src/app/components/hehel/HehelArchiveBoard.tsx)

- Archive cards with **pipeline bar** (segment colors = status colors)
- Counters: `12 To Print · 8 Paint`
- Expand → status groups list
- Guest: `readOnly` + server 403 on POST

## Column rule

Card column = **majority** of entry links (`majorityStatusId` from board API).

## API

See [cloud-sync-manifest.md](./cloud-sync-manifest.md).
