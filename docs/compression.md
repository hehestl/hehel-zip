# Сжатие `.hehe`

Формат **HEHE** — собственный контейнер Hehel (не 7z). Сжатие **пофайловое** (zstd), без solid-режима как у `7z -ms=on`.

## Пресеты (Настройки → Сжатие)

| Пресет | zstd | Словарь | STL/OBJ |
|--------|------|---------|---------|
| **Быстро** | 3 | default | store (без сжатия) |
| **Баланс** (по умолчанию) | 12 | default | zstd, если меньше store |
| **Ultra** | 19 | 128 MB (`w27`) | zstd, если меньше store |

Метка в `metadata.hehestl`: `Compression: zstd:12`, `zstd:19:w27` и т.д.

Если сжатый blob не меньше оригинала — entry пишется как **store** (без потери данных).

## Сравнение с внешним 7z

Для папки ~750 МБ (STL + превью + текстуры):

| Метод | Ориентир |
|-------|----------|
| WinRAR | ~425 МБ |
| 7z zip | ~577 МБ |
| **7z 7z solid LZMA2** | **~353 МБ** (лучший внешний) |
| `.hehe` balanced | зависит от доли STL; без solid обычно больше 7z |
| `.hehe` ultra | ближе к zstd-максимуму на медиа; STL ASCII может сжаться |

**Почему 7z solid меньше:** один LZMA2-поток по всему архиву, общий словарь 64–128 MB, повторы между PNG/JPG/STL.

**Почему `.hehe`:** мгновенный targeted extract, `metadata.hehestl` index 0, drag-out cache, без зависимости от `7z.exe` при открытии.

## Внешний бенчмарк (PowerShell)

```powershell
# 7z ultra (как в рекомендации, не .hehe)
7z a -t7z -m0=LZMA2 -mx=9 -mfb=64 -md=64m -ms=on -scc=UTF-8 bench.7z "C:\path\to\folder"

# Сравнить с .hehe ultra из Hehel (Настройки → Сжатие: ultra → Создать .hehe)
```

Проверка после создания: открыть в Hehel, превью, `metadata.hehestl`, drag STL.

## Дальше (не в v0.3)

- Solid-block v2 в HEHE (общий zstd/LZMA слой) — смена формата
- Progress events при create ultra
- Опциональный LZMA2 backend для create-only
