# AGENTS — hehel-zip (Hehel)

Агент Cursor **обязан** следовать правилам в [`.cursor/rules/`](.cursor/rules/).

| Файл | Назначение |
|------|------------|
| `00-ecosystem.mdc` | Всегда включено (`alwaysApply: true`) |
| `minimal-code.mdc` | YAGNI/KISS, DRY, слои UI/hooks/API |
| `00global.md` | Глобальные приоритеты и формат работы |
| `glossary.md` | Имена доменов (Hehel, Hemonea, Tekta и др.) |
| `architecture.md` | Архитектура и слои |
| `postgres.md` | PostgreSQL, Raw SQL (шаблон) |
| `telegram.md` | Telegram-боты |
| `testing.md` | Тестирование |
| `port-registry.md` | Пулы host-портов (2000–7999) |
| `acceptance-to-execution.md` | Фиксация принятия правил к исполнению |

Родительские правила монорепозитория: `hehe-ecosystem/.cursor/rules/`, `hehe-ecosystem/AGENTS.md`.
