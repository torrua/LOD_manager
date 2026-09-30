# AUDIT_REPORT.md — Комплексный архитектурный анализ, сверка с `torrua/loglan_core`, аудит ошибок и план оптимизации LOD Manager

> **Дата аудита**: 2026-09-30  
> **Версия проекта**: `1.7.0` (`Cargo.toml`: `1.7.0`)  
> **Эталонная модель БД**: [`torrua/loglan_core`](https://github.com/torrua/loglan_core) (`v0.3.2`, commit `079fd0d`) + `loglan_convert` ([`export.db`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/export.db))  
> **Роль**: Senior Architect Audit  
> **Статус проверок на момент первичного аудита → после исправлений (TDD)**:
>
> - `cargo test`: **11 passed / 16 FAILED** → **✅ 36 passed / 0 FAILED**
> - `cargo clippy -- -D warnings`: **FAILED (56 ошибок)** → **✅ PASSED (0 предупреждений)**
> - `cargo fmt -- --check`: **FAILED (CRLF в 9 файлах + diff в 2 файлах)** → **✅ PASSED (Unix LF)**
> - `npm run lint` (ESLint): **FAILED (63 ошибки)** → **✅ PASSED (0 ошибок, 0 предупреждений)**
> - `npm run format:check` (Prettier): **FAILED (162 файла)** → **✅ PASSED**
> - `npm run check` (`tsc --noEmit`): **PASSED (0 ошибок)** → **✅ PASSED (0 ошибок)**

---

## 1. Резюме архитектора (Executive Summary)

**LOD Manager** построен на современном и отлично подходящем для задач офлайн-лексикографии стеке: **Tauri v2 + Svelte 5 (Runes) + Rust + SQLite 3 (FTS5)**. Базовая архитектура проекта компактна, не перегружена лишними абстракциями и содержит ряд сильных инженерных решений:

- 5-шаговая агрегация карточки слова в [`db::get_word`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L355-L462) через `GROUP_CONCAT(..., x'1f')` и `json_group_array(json_object(...))` (устраняет проблему N+1).
- 4-запросный пакетный HTML-экспорт в [`export::generate_html`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/export.rs#L148-L242) (4 SQL-запроса вместо ~30 000).
- Двойной полнотекстовый индекс FTS5 (`def_fts` по полному тексту и `def_kw_fts` по ключевым словам `«...»`) с автоматическим откатом на `LIKE`.
- Кастомный виртуальный скроллинг списка слов в [`Sidebar.svelte`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components/Sidebar.svelte#L36-L47) и защита от гонок запросов (`loadingWordId`) в [`store.svelte.ts`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/store.svelte.ts#L299-L324).

**Однако в ходе глубокого аудита и прямой сверки с эталонным репозиторием [`torrua/loglan_core`](https://github.com/torrua/loglan_core) (из которого происходит структура БД `export.db`) выявлена серия критических регрессий и архитектурных расхождений**:

1. **Незавершенный переход на схему `loglan_core` («Schema Split-Brain»)**: при переходе от старой схемы `LOD Manager` к схеме `loglan_core` часть кода и миграций осталась на старых именах колонок (`type_id`, `event_start_id`, `settings(key, value)`), часть — на промежуточных (`group_`, `match_`), тогда как эталонная схема `loglan_core` использует `"group"`, `"match"`, `type`, `event_start`, `event_end`, `slots` + `grammar_code`.
2. **Потеря `slots` и `grammar` в определениях**: в `loglan_core` (`BaseDefinition`) грамматическая помета вида `2a` хранится раздельно в двух колонках: `slots INTEGER` (`2`) и `grammar_code VARCHAR(8)` (`"a"`). `LOD Manager` полностью игнорирует колонку `slots` (в `export.db` заполнена у **8 558 из 18 766** определений), а из-за рассинхронизации имен полей IPC (`grammar` vs `grammar_code`, `tags` vs `case_tags`) вообще не отображает грамматические коды и теги в UI и затирает их в `NULL` при сохранении.
3. **Несовместимость связей слов (`connect_words`, `connect_authors`, `notes JSON`) с `loglan_core`**: в `loglan_core` нет таблиц `word_affixes`, `word_usage` и `word_spellings` и нет колонки `words.source`. Аффиксы (`type_x = 'Affix'`), комплексы (`"group" = 'Cpx'`) и родительские слова связываются через единую таблицу `connect_words (parent_id, child_id)`, авторы — через `connect_authors ("AID", "WID")`, а примечания к году/автору/рангу хранятся в JSON-колонке `words.notes`. Из-за этого при открытии `export.db` в `LOD Manager` пустуют блоки **Affixes**, **Used In** и **Source**, в блок **Derivatives** попадают аффиксы вперемешку с комплексами, а у 9 879 слов выводится строка `Notes: null`.
4. **Ошибочное ограничение `UNIQUE(name, type)` в `LOD Manager`**: в `loglan_core` таблица `words` **не имеет** ограничения `UNIQUE(name, type)`, так как одно и то же написание и тип слова могут существовать в разных исторических интервалах событий (`event_start` / `event_end`) с разными `id_old` (в `export.db` присутствует **11 таких пар** / 22 строки, включая `cenja`, `clika`, `cutri`, `nigro`, `zoa`, `zoi`). Навязывание `UNIQUE(name, type)` в `LOD Manager` приводит к молчаливой потере 11 актуальных слов при импорте.
5. **Ошибки внутри самого `torrua/loglan_core` и `loglan_convert`**: в ходе кросс-проверки в самом `loglan_core` (и `loglan_convert`) обнаружено **13 внутренних ошибок и несоответствий** (включая сломанный `GLOB` с заменой `*` → `%` в `BaseSelector.get_like_condition` и `filter_key_by_word_cs`, потерю определений без ключей в `DefinitionSelector.by_event`, потерю числа `0` в `Exporter.merge_by`, дублирование 46 строк `Words` и 58 строк `WordDefinition` при экспорте слов с несколькими `WordSpell`, возврат строки `"None"` в `ExportWordConverter.e_rank`, отсутствие `ondelete="CASCADE"` у всех внешних ключей и др. — см. Раздел 7).

---

## 2. Критические ошибки времени выполнения (P0 — Data Loss & Broken Features)

### P0-1. Потеря данных `slots`, `grammar_code` и `case_tags` в определениях слов (Рассинхронизация с `loglan_core` и IPC)

- **Файлы**:
  - [`src-tauri/src/models.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/models.rs#L33-L69) (`Definition`, `SaveDefinition`)
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L413-L436) (`get_word` — `json_object`) и [строки 571–601](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L571-L601) (`save_definition`)
  - [`src-tauri/src/export.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/export.rs#L199-L219) (`generate_html`)
  - [`src/types.ts`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/types.ts#L7-L14) (`interface Definition`)
  - [`src/lib/components/WordDetail.svelte`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components/WordDetail.svelte#L129-L153) и [строки 278–305](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components/WordDetail.svelte#L278-L305)
- **Суть ошибки**:
  1. **Рассинхронизация IPC**: в коммитах `dd291da` и `7c7844d` поля структур `Definition` и `SaveDefinition` в Rust и ключи `json_object` в `db::get_word` были переименованы из `grammar` и `tags` в `grammar_code` и `case_tags`. Но фронтенд (`src/types.ts` и `WordDetail.svelte`) по-прежнему читает `d.grammar` и `d.tags` и отправляет в `save_definition` объект `{ grammar, usage, body, tags }`. При редактировании определения `SaveDefinition` десериализует `grammar_code: None` и `case_tags: None`, **безвозвратно затирая существующие значения в БД на `NULL`**.
  2. **Игнорирование `definitions.slots` (нарушение контракта `loglan_core`)**: в `torrua/loglan_core` (`loglan_core/definition.py:130-161` и `addons/exporter.py:201`) грамматическая помета из `WordDefinition.txt` (например, `2a`, `3v`, `1n`) при импорте делится на число мест предиката `slots: int | None` (`2`, `3`, `1`) и буквенный код `grammar_code: str | None` (`"a"`, `"v"`, `"n"`), а при чтении/экспорте склеивается обратно: `f"{slots or ''}{grammar_code or ''}"`. В `export.db` у **8 558 из 18 766 определений** заполнена колонка `slots`. `LOD Manager` выбирает только `d.grammar_code`, из-за чего `(2a)` превращается в `(a)` и перестает работать тултип `grammarTip(g)` в `WordDetail.svelte`. А при сохранении/импорте в `LOD Manager` вся строка `"2a"` пишется целиком в `grammar_code` при `slots = NULL`, нарушая инвариант `loglan_core`.
- **Как исправить**:
  - При чтении определений в `db::get_word`, `db::search_english_*` и `export::generate_html` формировать полный грамматический код как:
    ```sql
    NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '')
    ```
  - При записи в `db::save_definition`, `import.rs` и `converter/converter.rs` разбирать ведущие цифры грамматического кода в `slots: Option<i64>`, а остаток строки — в `grammar_code: Option<String>` (точно по алгоритму `get_grammar` из `loglan_convert`).
  - Синхронизировать имена полей между Rust (`models.rs`, `db.rs`) и TypeScript (`src/types.ts`, `WordDetail.svelte`).

---

### P0-2. `db::save_type` и `db::delete_type` падают при редактировании и удалении типа слова

- **Файл**: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L681-L701)
- **Суть ошибки**:
  1. В таблице `types` колонка называется `type` (в `loglan_core`: `type_: Mapped[str_016] = mapped_column("type", nullable=False)`), однако ветка `UPDATE` в `save_type` на строке 684 выполняет:
     ```sql
     UPDATE types SET name=?1, type_x=?2, group_=?3 WHERE id=?4
     ```
     что падает с `table types has no column named name`.
  2. В `delete_type` (строка 698) выполняется:
     ```sql
     UPDATE words SET type=NULL WHERE type=?1
     ```
     однако и в `loglan_core`, и в `init_schema` (`db.rs:54`) колонка объявлена как `type INTEGER NOT NULL REFERENCES types(id)`. При удалении любого типа, к которому привязано хотя бы одно слово, запрос падает с `NOT NULL constraint failed: words.type`.
- **Как исправить**:
  - В `save_type` заменить `SET name=?1, type_x=?2, group_=?3` на `SET type=?1, type_x=?2, "group"=?3`.
  - В `delete_type` проверять наличие связанных слов (`SELECT COUNT(*) FROM words WHERE type=?1`) и возвращать понятную ошибку, если тип используется словами.

---

### P0-3. `db::get_event_words` падает с ошибкой (`no such column: w.event_start_id`)

- **Файл**: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L1202-L1221)
- **Суть ошибки**:
  Запросы в `get_event_words` обращаются к несуществующим колонкам `w.event_start_id` и `w.event_end_id`, тогда как в схеме `loglan_core` колонки таблицы `words` называются `event_start` и `event_end` и ссылаются на `events(event_id)` (при этом UI передает `event.id`).
- **Последствия**: В `EventDetail.svelte` промис `.catch(() => {})` молча глотает ошибку, и для любого события всегда выводится `"No word changes recorded for this event."`.
- **Как исправить**:
  ```sql
  SELECT w.name FROM words w
  JOIN events e ON e.event_id = w.event_start
  WHERE e.id = ?1 ORDER BY w.name
  ```
  (и аналогично для `w.event_end`).

---

### P0-4. Ошибки импорта/конвертации и сохранения слов (`save_word`, `import.rs`, `converter.rs`): `event_start`/`event_end = 9999`, потеря `id_old`/`origin_x`/`source`, перепутанные колонки `events` и `types`, ошибка парсинга `"False"`

- **Файлы**:
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L464-L536)
  - [`src/lib/components/WordForm.svelte`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components/WordForm.svelte#L143-L154)
  - [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/import.rs#L138-L362)
  - [`src-tauri/src/converter/converter.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/converter/converter.rs#L135-L593)
- **Суть ошибки**:
  1. `WordForm.svelte` передает в `data.event_start` и `data.event_end` **название** события (`e.name`, например `"Start"`), а `db::save_word` (строки 474–494) ищет его запросом `SELECT id FROM events WHERE event_id=?1`, сравнивая числовой `event_id` со строкой имени и возвращая суррогатный `id` вместо `event_id`. В результате `ev_start` становится `None`, и сохранение слова падает с `NOT NULL constraint failed: words.event_start`.
  2. В `import.rs:209-214` из-за предварительной вставки `event_id = 1, name = 'Start'` в `init_schema` строка с `event_id = 1` из `LexEvent.txt` игнорируется (`INSERT OR IGNORE`), а в `import.rs:211` и `converter.rs:448,461` в `words.event_start` записывается `events.id` вместо `events.event_id`.
  3. **Перепутаны `annotation` и `suffix` при импорте `LexEvent.txt` (`import.rs:207-208`)**: в формате `LexEvent.txt` (`AccessLexEvent` и `Exporter.export_event` в `loglan_core`) колонка 4 (`r[4]`) — это `Annotation`, а колонка 5 (`r[5]`) — `Suffix`. Однако `import.rs:207-208` считывает `let suffix = r.get(4)` и `let annotation = r.get(5)`, меняя их местами в БД!
  4. **Потеря `parentable` и `description` при импорте `Types.txt` и `Syllables.txt` (`import.rs:142-146`, `converter.rs:151,308`)**:
     - В `import.rs:142-146` колонка 3 (`parentable`: `"True"`/`"False"`) записывается в `description`, а `parentable` жестко захардкожен как `Some(true)`.
     - В `converter.rs:151` и `converter.rs:308` вызывается `s.parse::<bool>().ok().unwrap_or(true)`. Так как в файлах `Types.txt` и `Syllables.txt` булевы значения записаны в Python-формате с заглавной буквы (`"True"` / `"False"`), стандартный `str::parse::<bool>()` в Rust (принимающий только строчные `"true"`/`"false"`) возвращает `Err`, и `.unwrap_or(true)` молча превращает все `"False"` в **`true`**!
  5. **Потеря `origin_x`, `tid_old` и `source` в `import.rs:286-299` и `converter.rs:468-485`**: `import.rs` извлекает `source`, `origin_x` и `tid_old` из `Words.txt` в переменные на строке 286, но вообще не передает их в `INSERT INTO words` на строке 299 (и не заполняет `connect_authors`).
  6. **Семантика `9999` в `WordSpell.txt` (`loglan_core`)**: в формате `WordSpell.txt` (`Exporter.export_word_spell` в `loglan_core/addons/exporter.py:234`) код `event_end = 9999` означает открытый интервал (`event_end IS NULL` — слово активно в текущей версии словаря). В `loglan_convert` (`postgres/interface.py:124`) явно стоит проверка `int(item[5]) if int(item[5]) < 9999 else None`. В `import.rs` и `converter.rs` такой проверки нет.
  7. **Потеря определений для слов с несколькими написаниями (`WordSpell`)**: в `loglan_convert` (`postgres/interface.py:180-196`) одному `id_old` может соответствовать **несколько** строк в `words` (46 слов с несколькими историческими написаниями в `export.db`, например `alkooli` и `alkoholi` оба имеют `id_old = 75`), и определения привязываются ко **всем** строкам с этим `id_old`. В `import.rs:221,321` используется `HashMap<String, i64>`, который перезаписывает предыдущий `db_id` при совпадении `old_id`, оставляя первое написание без определений! А в `converter.rs:555-560` определения вообще привязываются к случайным словам через `LIMIT 1 OFFSET (old_id - 1)`.
- **Как исправить**:
  - В `save_word` искать `event_id` по имени или числу (`SELECT event_id FROM events WHERE name=?1 OR CAST(event_id AS TEXT)=?1`) и использовать `1` по умолчанию для `event_start`.
  - В `import.rs` исправить порядок колонок `annotation` (`r[4]`) и `suffix` (`r[5]`) для `events`, а также `parentable` (`r[3]`) и `description` (`r[4]`) для `types`; парсить булевы значения регистронезависимо (`s.eq_ignore_ascii_case("true") || s == "1"`).
  - В `import.rs` и `converter.rs` сохранять `origin_x` и `"TID_old"`, трактовать `event_end >= 9999` как `None` (`NULL`), использовать `HashMap<i64, Vec<i64>>` для маппинга `id_old -> Vec<word_id>` и дублировать определения, авторов и связи для всех `word_id` с данным `id_old`, как это делает `loglan_convert`.

---

### P0-5. Ошибочное ограничение `UNIQUE(name, type)` на `words` и падение миграций при открытии БД

- **Файлы**:
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L42-L58) (`init_schema`)
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L141-L288) (`add_missing_indexes`, `migrate_words_unique_if_needed`, `migrate_event_columns_if_needed`)
  - [`src-tauri/src/commands/database.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/commands/database.rs#L16-L18)
- **Суть ошибки**:
  1. **`UNIQUE(name, type)` противоречит модели данных `torrua/loglan_core`**:
     В `loglan_core` (`loglan_core/word.py`) таблица `words` **не имеет** ограничения `UNIQUE(name, type)`. В `export.db` содержится **11 пар слов (22 строки)** с одинаковыми `(name, type)`, но разными `id_old`, `origin` и интервалами событий `(event_start, event_end)`:
     `cenja`, `clika`, `cutri`, `nig`, `nigcea`, `nigcli`, `nigro`, `nigslicui`, `sliti`, `zoa`, `zoi` (например, `cenja` с `id=905, id_old=802, event_start=1, event_end=6, origin="... | 4/5G änder n"` и `cenja` с `id=906, id_old=10133, event_start=6, event_end=NULL, origin="... | 4/5G ander n"`).
     Из-за `UNIQUE(name, type)` в `db::init_schema` при импорте через `INSERT OR IGNORE INTO words` **11 новых (активных после события 6!) версий слов молча отбрасываются**, а в `import.rs:319-321` их определения ошибочно привязываются к устаревшей записи (`event_end=6`)!
  2. В `add_missing_indexes` строки 147–149 пытаются создать индексы на старые имена колонок `words(type_id)`, `words(event_start_id)`, `words(event_end_id)`. Из-за этого `conn.execute_batch(...)` немедленно падает с `no such column: type_id`, а поскольку в `open_database` результат игнорируется (`let _ = db::add_missing_indexes(&conn);`), **весь последующий SQL в батче (строки 152–173) никогда не выполняется**.
  3. `migrate_words_unique_if_needed` (`db.rs:181-260`) пытается принудительно навесить `UNIQUE(name, type)` и пересоздает таблицу `words` со старыми колонками `type_id, source, event_start_id, event_end_id`, разрушая схему БД.
  4. `migrate_event_columns_if_needed` (`db.rs:265-288`) обращается к несуществующей колонке `events.notes` вместо `events.definition`.
- **Как исправить**:
  - **Удалить** ограничение `UNIQUE(name, type)` из `db::init_schema` и полностью удалить деструктивную миграцию `migrate_words_unique_if_needed` (при необходимости уникальности на уровне импорта использовать `(id_old, name, event_start)`).
  - В `add_missing_indexes` заменить имена колонок на `words(type)`, `words(event_start)`, `words(event_end)`.
  - Удалить или исправить устаревшую `migrate_event_columns_if_needed` и не глотать ошибки миграций через `let _ = ...`.

---

### P0-6. Двойная несовместимая схема таблицы `settings` (Не работает вкладка Database Stats)

- **Файлы**:
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L99-L108) (`init_schema`)
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L791-L820) (`list_settings`, `upsert_setting`)
  - [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs#L378-L403) (`import_settings`)
- **Суть ошибки**:
  В `torrua/loglan_core` (`loglan_core/setting.py`) таблица `settings` имеет фиксированную реляционную схему:
  `id, date, db_version, last_word_id, db_release, created, updated` (в `export.db`: `date='2024-08-22 05:08:00.000000', db_version=1, last_word_id=10150, db_release='4.5.9'`).
  `init_schema` в `db.rs` создает именно эту таблицу `settings`, однако `list_settings()`, `upsert_setting()` и `import_settings()` выполняют запросы `SELECT key, value FROM settings` и `INSERT INTO settings(key, value)`.
- **Последствия**:
  Вызов `get_db_stats` всегда падает с `no such column: key` как на новой БД, так и на `export.db`, поэтому вкладка **Tools → Database** не показывает метаданные словаря (`app.dbStats` остается `null`).
- **Как исправить**:
  Оставить таблицу `settings` в точном соответствии со схемой `loglan_core` (`date, db_version, last_word_id, db_release`), а в `list_settings()` и `upsert_setting()` проецировать последнюю строку `settings` (`ORDER BY id DESC LIMIT 1`) в пары `SettingItem { key, value }` (`date`, `db_version`, `last_word_id`, `db_release`) — с fallback на чтение `(key, value)` только для старых legacy-баз.

---

### P0-7. Несовместимость с канонической схемой `torrua/loglan_core` (`export.db`): `"group"`, `"match"`, `connect_words`, `connect_authors`, `words.year` (`DATE`) и `words.notes` (`JSON`)

- **Файлы**:
  - [`export.db`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/export.db)
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs)
  - [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs)
  - [`src-tauri/src/export.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/export.rs)
  - [`src-tauri/src/converter/converter.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/converter/converter.rs)
- **Суть ошибки**:
  При прямой сверке `LOD Manager` с `torrua/loglan_core` и `export.db` выявлено 6 фундаментальных расхождений:
  1. **Имена колонок `types."group"` и `words."match"`**:
     В `loglan_core` (`BaseType.group` в `type.py:54` и `BaseWord.match` в `word.py:231`) колонки в SQLite называются **`"group"`** и **`"match"`** (а НЕ `group_` и `match_`). В `db.rs`, `import.rs` и `export.rs` ошибочно используются `group_` и `match_`. Важно: переименовывать колонки в `export.db` в `group_`/`match_` **нельзя**, так как это сломает обратную совместимость файла БД с Python-пакетом `loglan_core`! Напротив, каноническими именами колонок в `LOD Manager` должны быть `"group"` и `"match"` (с автоматической миграцией `group_ -> "group"` и `match_ -> "match"` для промежуточных баз).
  2. **Хранение аффиксов (`djifoa`), комплексов (`used_in`) и составных частей комплексов (`parents`) в `connect_words` (плюс дефис в `hei-`)**:
     В `loglan_core` **не существует** таблиц `word_affixes`, `word_usage` и `word_spellings`! Вместо этого (`loglan_core/word.py:417-479` и `addons/export_word_converter.py:50-87`):
     - Аффиксы (`types.type_x = 'Affix'`, `types.type = 'Afx'`) и комплексы (`types."group" = 'Cpx'`) хранятся как обычные слова в таблице `words`.
     - Таблица `connect_words (parent_id, child_id)` связывает исходное слово (`parent_id`) как с его производными аффиксами (`991` связь в `export.db`), так и с образованными от него комплексами (`13 960` связей в `export.db`). При этом в `export.db` аффикс `hei-` (`id=3427`) хранится с дефисом, поэтому `ExportWordConverter.e_affixes` удаляет дефисы (`afx.name.replace("-", "")`).
     - В `BaseWord`:
       - `word.djifoa` / `word.affixes` — это дочерние слова из `connect_words`, у которых `type.type_x == 'Affix'` (без дефисов);
       - `word.complexes` (`e_usedin`) — это дочерние слова из `connect_words`, у которых `type."group" == 'Cpx'`;
       - `word.parents` — это родительские слова из `connect_words` (`child_id = word.id`), из которых состоит данный комплекс (или от которых образован данный аффикс).
     - В `LOD Manager` (`db::get_word`) `affixes` и `used_in` читаются из пустых таблиц `word_affixes` и `word_usage`, а в `children` сваливаются все `connect_words` без фильтрации по типу!
  3. **Хранение авторов (`connect_authors`) и `source`**:
     В `loglan_core` нет колонки `words.source`. Авторы связаны со словами через таблицу `connect_authors ("AID", "WID")` (`12 035` записей в `export.db`), а строка источника (`ExportWordConverter.e_source`) собирается из `"/".join(sorted(a.abbreviation for a in word.authors))` плюс `notes.get("author", "")`. В `LOD Manager` (`db::get_word`) поле `source` захардкожено как `None`, а в `db::list_authors` `word_count` захардкожен как `0` вместо подсчета из `connect_authors`.
  4. **Формат `words.year` (`DATE` `'YYYY-01-01'`) и JSON-колонка `words.notes` (`'null'` и `{"year", "author", "rank"}`)**:
     - В `loglan_core` (`word.py:254`) колонка `year` имеет тип `datetime.date` (`DATE`) и хранится в `export.db` в формате `'YYYY-01-01'` (например, `'1975-01-01'`), а уточнения к году (`"(changed '16)"`), рангу и автору хранятся в JSON-колонке `words.notes` (`{"author": "...", "year": "...", "rank": "..."}`, заполнена у 294 слов). В `ExportWordConverter.e_year` год собирается как `f"{self.word.year.year} {notes.get('year', '')}".strip()` (`"1975 (changed '16)"`), а в `e_rank` — `f"{rank} {notes.get('rank', '')}".strip()`.
     - Для остальных 9 879 слов SQLAlchemy при `bulk_insert_mappings` сериализовал Python `None` в 4-символьную JSON-строку `'null'`.
     - `LOD Manager` выводит сырую дату `'1975-01-01'` вместо `'1975'` (или `'1975 (changed \'16)'`) и отображает в карточке каждого слова `Notes: null` или сырой JSON!
  5. **Отсутствие таблиц `syllables`, `keys`, `connect_keys`, `connect_authors` в `db::init_schema`**:
     `init_schema` не создает таблицы `syllables`, `keys`, `connect_keys` и `connect_authors`, которые входят в каноническую схему `loglan_core` (и используются в `converter.rs`).

---

### P0-8. Падение `db::delete_word`, `db::delete_definition` и `db::delete_author` на `export.db` с `FOREIGN KEY constraint failed`

- **Файлы**:
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L564-L567) (`delete_word`)
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L603-L606) (`delete_definition`)
  - [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L737-L740) (`delete_author`)
- **Суть ошибки**:
  При открытии БД `LOD Manager` включает проверку внешних ключей (`PRAGMA foreign_keys=ON;` в `commands/database.rs:13`). Однако в DDL `export.db` (сгенерированном моделями `torrua/loglan_core`: `definition.py:55` и `relationships.py:21-51`) внешние ключи таблиц `definitions(word_id)`, `connect_words(parent_id, child_id)`, `connect_authors("AID", "WID")` и `connect_keys("KID", "DID")` объявлены **без `ON DELETE CASCADE`**.
  При этом функции `delete_word`, `delete_definition` и `delete_author` в `db.rs` выполняют только одиночный `DELETE FROM words WHERE id=?1` / `DELETE FROM definitions WHERE id=?1` / `DELETE FROM authors WHERE id=?1`.
- **Последствия**:
  Попытка удалить любое слово, определение (у которого есть ключи в `connect_keys`) или автора в `export.db` гарантированно завершается ошибкой `sqlite3.IntegrityError: FOREIGN KEY constraint failed`!
- **Как исправить**:
  - В `db::delete_definition`: перед `DELETE FROM definitions WHERE id=?1` выполнять `DELETE FROM connect_keys WHERE "DID"=?1`.
  - В `db::delete_word`: внутри транзакции сначала удалять связанные записи из `connect_keys` (`WHERE "DID" IN (SELECT id FROM definitions WHERE word_id=?1)`), `definitions` (`WHERE word_id=?1`), `connect_words` (`WHERE parent_id=?1 OR child_id=?1`), `connect_authors` (`WHERE "WID"=?1`), `word_affixes`, `word_spellings`, `word_usage`, и лишь затем `DELETE FROM words WHERE id=?1`.
  - В `db::delete_author`: перед `DELETE FROM authors WHERE id=?1` выполнять `DELETE FROM connect_authors WHERE "AID"=?1`.

---

## 3. Проблемы тестирования, CI/CD и стабильности (P1)

### P1-1. Падение 16 из 27 Rust-тестов (`cargo test`)

- **Файлы**: [`src-tauri/src/lib.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/lib.rs#L145-L756), [`src-tauri/src/converter/tests.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/converter/tests.rs#L8-L60)
- **Причина**:
  - 15 тестов в `lib.rs` используют устаревшие SQL-вставки (`INSERT INTO types (name)`, `INSERT INTO words (name, type_id, source)`, `INSERT INTO events (..., notes)`, `INSERT INTO authors (initials, ...)`).
  - Тест `test_convert_text_files` в `src-tauri/src/converter/tests.rs` ищет несуществующую директорию `../test_converter` и удаляет ее в конце теста.
- **Решение**: Обновить SQL-фикстуры в тестах `lib.rs` под каноническую схему `loglan_core` (`init_schema`), добавить интеграционные тесты для `slots + grammar_code`, `connect_words`, `connect_authors` и JSON `notes`, а в `test_convert_text_files` создавать временную директорию с тестовыми `.txt` файлами в `std::env::temp_dir()`.

### P1-2. Паника `block_on` в `debug_update_check` на Desktop

- **Файл**: [`src-tauri/src/lib.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/lib.rs#L93-L123)
- **Причина**: Синхронная команда `fn debug_update_check` вызывает `tauri::async_runtime::block_on(updater.check())` внутри потока Tokio runtime, что вызывает панику `"cannot block the current thread from within an asynchronous context"`.
- **Решение**: Сделать команду асинхронной (`async fn debug_update_check(app: tauri::AppHandle) -> commands::Res<String>`) и использовать `updater.check().await`.

### P1-3. Невалидный JSON `latest.json` и неверный URL автообновления

- **Файлы**:
  - [`.github/workflows/release.yml`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/.github/workflows/release.yml#L94-L97)
  - [`src-tauri/tauri.conf.json`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/tauri.conf.json#L54)
- **Причина**:
  1. На строке 95 `release.yml` пропущена открывающая кавычка `\"` перед `windows-x86_64-msi`:
     ```bash
     PLATFORMS="${PLATFORMS}windows-x86_64-msi\":{\"signature\":...
     ```
     Из-за этого генерируется невалидный JSON.
  2. В `tauri.conf.json` указан endpoint `https://raw.githubusercontent.com/torrua/LOD_manager/main/latest.json`, но файл `latest.json` удален из ветки `main` и публикуется только в GitHub Releases (`https://github.com/torrua/LOD_manager/releases/latest/download/latest.json`).

### P1-4. Ошибки линтеров и форматирования (Clippy: 56, rustfmt: 11, ESLint: 63, Prettier: 162)

- **Frontend**:
  - В [`eslint.config.js`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/eslint.config.js#L69-L71) и [`.prettierignore`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/.prettierignore) не добавлены исключения `.claude/**` и `.planning/**`. Из-за наличия двух рабочих копий проекта в `.claude/worktrees/` `typescript-eslint` обнаруживает 3 файла `tsconfig.json` и падает с 63 ошибками `No tsconfigRootDir was set`.
- **Backend**:
  - 9 файлов в `src-tauri/src/` сохранены с `CRLF` вместо `LF` (`rustfmt.toml` требует `newline_style = "Unix"`).
  - В `import.rs:232-233,286` и `models.rs:47,59,60` есть неиспользуемые переменные и поля структур.
  - В `converter/mod.rs` — ошибка `clippy::module_inception` (`pub mod converter;` внутри модуля `converter`).
  - В `converter/converter.rs`, `commands/words.rs`, `commands/authors.rs`, `db.rs` — 48 ошибок `clippy::uninlined_format_args`, `clippy::doc_markdown`, `clippy::redundant_closure_for_method_calls`, `clippy::used_underscore_binding`, `clippy::type_complexity`.

---

## 4. Оптимизация производительности, UX и архитектуры (P2 / P3)

### P2-1. Устранение полной перестройки FTS5 при удалении одного слова

- **Файл**: [`src-tauri/src/commands/words.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/commands/words.rs#L46-L52)
- **Проблема**: `delete_word` вызывает `db::rebuild_fts(conn)`, что перестраивает обе виртуальные таблицы FTS5 по всем ~40 000 определениям при удалении одного слова.
- **Оптимизация**: Перед удалением слова получать список `id` его определений (`SELECT id FROM definitions WHERE word_id = ?1`) и после `DELETE FROM words` вызывать инкрементальный `db::fts_update(conn, def_id, "")` для каждого `def_id`. Время удаления сократится с сотен миллисекунд до <1 мс.

### P2-2. Использование единого соединения в `rebuild_fts` и `compact_db`

- **Файл**: [`src-tauri/src/commands/search.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/commands/search.rs#L41-L78)
- **Проблема**: Команды открывают второе соединение `Connection::open(&path)` параллельно с активным соединением в `AppState.db`. При выполнении `VACUUM` это может вызывать `SQLITE_BUSY`.
- **Оптимизация**: Выполнять `rebuild_fts` и `compact_db` через `with_db(&state, ...)` на основном соединении.

### P2-3. Прямой импорт в памяти для Android (`import_contents`)

- **Файл**: [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs#L51-L87)
- **Проблема**: `import_contents` записывает полученные из JS строки во временную папку `std::env::temp_dir()/lod_import_{pid}`, а затем снова читает их с диска через `import_files`. Кроме того, в `import.rs:374` игнорируется ошибка коммита транзакции (`let _ = tx.commit();`).
- **Оптимизация**: Выделить внутреннюю функцию `import_from_memory(conn, files: &[(String, String)])`, чтобы `import_files` читал файлы и делегировал ей, а `import_contents` работал напрямую в памяти без лишнего дискового ввода-вывода. Ошибку `tx.commit()` пробрасывать через `?`.

### P2-4. Исправление поиска и клавиатурной навигации на вкладке Events (`Sidebar.svelte`)

- **Файл**: [`src/lib/components/Sidebar.svelte`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components/Sidebar.svelte#L134-L148) и [строки 316–335](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components/Sidebar.svelte#L316-L335)
- **Проблема**:
  1. На вкладке `events` поле ввода показывает placeholder `"Search events…"`, но ввод текста меняет `app.searchQ`, а список `{#each app.events}` никак не фильтруется по `app.searchQ`.
  2. Нажатие клавиш `ArrowUp` / `ArrowDown` на элементах списка событий вызывает `itemKeydown(e, i)`, который выбирает слово из `app.filteredWords` вместо события из `app.events`.
- **Оптимизация**: Добавить `$derived` фильтр `filteredEvents` по `app.searchQ` и ветку `app.tab === 'events'` в `itemKeydown` и `searchKeydown`.

### P2-5. Некорректная сортировка дат (`NaN`) и мутация `$state`-массива `app.events` (`store.svelte.ts`)

- **Файл**: [`src/lib/store.svelte.ts`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/store.svelte.ts#L364-L373)
- **Проблема**:
  1. `const eventsWithDate = app.events.filter((e) => e.date !== null)` пропускает дефолтное событие `'Start'`, у которого `date === ""` (пустая строка из `init_schema`). При сортировке `new Date("").getTime()` возвращает `NaN`, что ломает компаратор `.sort()` и приводит к выбору `'Start'` (`id=1`) вместо самого свежего события.
  2. В ветке fallback `app.events.sort((a, b) => b.id - a.id)[0]` мутирует реактивный массив `app.events` на месте, меняя порядок событий в сайдбаре.
- **Оптимизация**: Фильтровать по `Boolean(e.date?.trim()) && !Number.isNaN(Date.parse(e.date))` и использовать копию `[...app.events].sort(...)`.

### P2-6. Очистка отладочных `println!` и `console.log` в горячих путях

- **Файлы**: `src-tauri/src/db.rs`, `src-tauri/src/commands/words.rs`, `src-tauri/src/commands/authors.rs`, `src-tauri/src/converter/converter.rs`, `src/lib/store.svelte.ts`.
- **Проблема**: Каждый вызов `get_words`, `get_word`, `applyFilter` и каждая строка при конвертации файлов пишет в `stdout` / `console.log`, замедляя импорт и фильтрацию и нарушая правило ESLint `no-console`.

---

## 5. Гигиена репозитория и безопасность (P2 / P3)

1. **Секреты и артефакты в корне проекта**:
   - В корне рабочей директории находится неотслеживаемый файл `keys signature.txt` (содержащий ключи подписи Tauri updater), лог падения JVM `hs_err_pid31460.log` (483 КБ), `affix_count.txt`, `src-tauri/build_rs_cov.profraw`, а также папка `.claude/worktrees/` с двумя полными копиями репозитория.
   - **Действие**: Немедленно добавить `keys signature.txt`, `hs_err_pid*.log`, `*.profraw`, `.claude/` в `.gitignore`, а `.claude/` и `.planning/` — в `.prettierignore` и `eslint.config.js`.
2. **Пустые директории и файлы-заглушки**:
   - Пустые папки: `src/lib/composables/`, `src/lib/repositories/`, `src/lib/services/`, `src/lib/stores/`, `.github/workflows/.github/`.
   - Неиспользуемые заглушки: `src/lib/components/DeleteModal.svelte`, `src/lib/components/_placeholder.ts`.
3. **Рассинхронизация версий**:
   - `package.json`: `1.6.10`
   - `src-tauri/tauri.conf.json`: `1.6.10`
   - `src-tauri/Cargo.toml`: `1.6.9` (необходимо обновить до `1.6.10`).

---

## 6. Сверка с эталонной архитектурой `torrua/loglan_core` (`export.db`)

Ниже приведена полная таблица сверки 11 канонических таблиц SQLAlchemy-модели [`torrua/loglan_core`](https://github.com/torrua/loglan_core) (и конвертера `loglan_convert`, генерирующего `export.db`) с текущей реализацией `LOD Manager`:

| Таблица в `loglan_core` | Канонические колонки (`loglan_core` / `export.db`)                                                                                                            | Статус в `LOD Manager` (`db.rs` / `import.rs` / `converter.rs`)                                                                           | Необходимое действие в `LOD Manager`                                                                                                         |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `authors`               | `id, abbreviation UNIQUE, full_name, notes, created, updated`                                                                                                 | Совпадает по колонкам, но `list_authors` возвращает `0` вместо `COUNT` по `connect_authors`                                               | Считать `word_count` через `LEFT JOIN connect_authors ca ON ca."AID" = a.id`                                                                 |
| `events`                | `id, event_id UNIQUE, name, date, definition, annotation, suffix, created, updated`                                                                           | Совпадает, но `migrate_event_columns_if_needed` ищет `notes`, а `save_word` путает `id` и `event_id`                                      | Использовать `events.event_id` для FK `words.event_start`/`event_end`; убрать миграцию `notes`                                               |
| `types`                 | `id, type, type_x, "group", parentable, description, created, updated`                                                                                        | `db.rs` использует `group_` вместо `"group"`; `save_type` обновляет несуществующую колонку `name`                                         | Перевести `db.rs`, `import.rs`, `export.rs` на `"group"` (и мигрировать `group_ -> "group"`)                                                 |
| `settings`              | `id, date UNIQUE, db_version, last_word_id, db_release, created, updated`                                                                                     | `init_schema` создает схему `loglan_core`, но `list_settings` / `upsert_setting` запрашивают `key, value`                                 | Читать/обновлять поля `date, db_version, last_word_id, db_release` из `settings`                                                             |
| `syllables`             | `id, name, type, allowed, created, updated`                                                                                                                   | Отсутствует в `db::init_schema`, из-за чего `converter::convert_syllables` молча падает                                                   | Добавить `CREATE TABLE IF NOT EXISTS syllables` в `db::init_schema`                                                                          |
| `words`                 | `id, name, origin, origin_x, "match", rank, year, notes (JSON), id_old, "TID_old", type, event_start, event_end, created, updated` (без `UNIQUE(name, type)`) | `db.rs` использует `match_` вместо `"match"`, навязывает `UNIQUE(name, type)` (теряя 11 слов), не парсит JSON в `notes` (`'null'`)        | Перевести на `"match"`, убрать `UNIQUE(name, type)`, обрабатывать JSON в `notes` (`'null'` → `None`)                                         |
| `definitions`           | `id, word_id, position, body, usage, grammar_code, slots, case_tags, language, notes, created, updated`                                                       | `LOD Manager` игнорирует `slots` при чтении (8 558 определений в `export.db`) и не заполняет `slots` при записи                           | Склеивать `slots \|\| grammar_code` при чтении и разделять на `(slots, grammar_code)` при записи                                             |
| `keys`                  | `id, word, language, created, updated, UNIQUE(word, language)`                                                                                                | Отсутствует в `db::init_schema`; `LOD Manager` использует FTS5 `def_kw_fts` вместо `keys`                                                 | Добавить `CREATE TABLE IF NOT EXISTS keys` и `connect_keys` в `init_schema` для 100% совместимости схемы                                     |
| `connect_authors`       | `"AID" REFERENCES authors(id), "WID" REFERENCES words(id), PRIMARY KEY ("AID", "WID")`                                                                        | Отсутствует в `db::init_schema`; не заполняется при импорте и не читается в `get_word` (`source: None`)                                   | Создать таблицу `connect_authors`, читать `source` из `connect_authors` + `notes.author`, заполнять при импорте                              |
| `connect_words`         | `parent_id REFERENCES words(id), child_id REFERENCES words(id), PRIMARY KEY (parent_id, child_id)`                                                            | В `export.db` хранит и аффиксы (`type_x='Affix'`), и комплексы (`"group"='Cpx'`). `LOD Manager` читает пустые `word_affixes`/`word_usage` | В `get_word` и `export::generate_html` извлекать `affixes`, `used_in` и `parents`/`children` из `connect_words` с fallback на legacy-таблицы |
| `connect_keys`          | `"KID" REFERENCES keys(id), "DID" REFERENCES definitions(id), PRIMARY KEY ("KID", "DID")`                                                                     | Отсутствует в `db::init_schema`                                                                                                           | Добавить `CREATE TABLE IF NOT EXISTS connect_keys` в `db::init_schema`                                                                       |

---

## 7. Внутренние ошибки и несоответствия в самом `torrua/loglan_core` (и `loglan_convert`)

В ходе аудита исходного кода `torrua/loglan_core` (`v0.3.2`) и `loglan_convert` выявлено **13 внутренних ошибок и несоответствий**, которые были эмпирически подтверждены на `export.db` и которые необходимо учитывать в `LOD Manager` (и рекомендуется исправить в upstream-репозиториях `torrua/loglan_core` и `loglan_convert`):

### LC-1. `BaseSelector.get_like_condition` (`loglan_core/addons/base_selector.py:193-201`) и `filter_key_by_word_cs` (`loglan_core/addons/filters.py:49-54`) — сломанный регистрозависимый поиск с wildcard в SQLite (`GLOB`) и отсутствие замены `?` → `_` для `LIKE`/`ILIKE`

- **Код в `loglan_core/addons/base_selector.py:193-201`** (и идентичный код в **`loglan_core/addons/filters.py:49-54`**):

  ```python
  value = value.replace("*", "%")

  if not self.case_sensitive:
      return column.ilike(value)

  if self.is_sqlite:
      return column.op("GLOB")(value)

  return column.like(value)
  ```

- **Суть ошибки**:
  1. И в `BaseSelector.get_like_condition`, и в `filter_key_by_word_cs` символ `*` безусловно заменяется на `%`, после чего при `is_sqlite=True` и `case_sensitive=True` вызывается оператор SQLite `GLOB` (`WHERE words.name GLOB 'pru%'` и `WHERE keys.word GLOB 'ab%'`). Однако оператор `GLOB` в SQLite использует синтаксис Unix-глобов (`*` и `?`), а символ `%` трактует как обычный литерал! В результате и `WordSelector(is_sqlite=True, case_sensitive=True).by_name("pru*")`, и `KeySelector(is_sqlite=True, case_sensitive=True).by_key("ab*")` / `DefinitionSelector.by_key` возвращают **0 записей**.
  2. На ветках `LIKE` / `ILIKE` (`case_sensitive=False` или PostgreSQL) заменяется только `*` → `%`, но **не заменяется** односимвольный wildcard `?` → `_` (`lower(words.name) LIKE lower('da?o')`), хотя docstring `get_like_condition` явно документирует поддержку `?` и `*`.

### LC-2. `DefinitionSelector.by_event` (`loglan_core/addons/definition_selector.py:72-80`) — потеря определений без ключевых слов из-за лишнего `INNER JOIN connect_keys`

- **Код в `loglan_core`**:
  ```python
  subquery = (
      select(self.model.id)
      .join(t_connect_keys)
      .join(BaseWord)
      .where(filter_word_by_event_id(event_id))
      .scalar_subquery()
  )
  ```
- **Суть ошибки**: Подзапрос фильтрации определений по событию делает `.join(t_connect_keys).join(BaseWord)`. Так как `BaseDefinition` уже имеет прямой внешний ключ `word_id -> words.id`, промежуточный `INNER JOIN connect_keys ON definitions.id = connect_keys."DID"` является избыточным и **отбрасывает все определения, не имеющие ключевых слов `«...»` в `connect_keys`** (в `export.db` теряется **40 из 18 766 определений**, включая определения аффиксов и служебные глоссы).

### LC-3. `BaseSelector.select_columns` (`loglan_core/addons/base_selector.py:64-80`) — сброс `JOIN`, `ORDER BY`, `LIMIT`, `OFFSET`

- **Суть ошибки**: Метод `select_columns` пересоздает `self._statement = select(*self._selected_columns).where(existing_conditions)`, сохраняя только `whereclause` и **полностью уничтожая** все ранее добавленные `.join(...)`, `.order_by(...)`, `.limit(...)`, `.offset(...)` и `.options(...)` (вместо использования стандартного метода SQLAlchemy `self._statement.with_only_columns(*columns)`).

### LC-4. `ExportWordConverter.e_rank` (`loglan_core/addons/export_word_converter.py:114-122`) — экспорт литеральной строки `"None"` вместо пустой строки

- **Код в `loglan_core`**:
  ```python
  @property
  def e_rank(self) -> str:
      notes: dict[str, str] = self.word.notes or {}
      return f"{self.word.rank} {notes.get('rank', str())}".strip()
  ```
- **Суть ошибки**: В отличие от `e_year` (где есть проверка `if self.word.year:`), в `e_rank` проверка на `None` отсутствует. Для всех слов, у которых `rank IS NULL` (в `export.db` это **47 слов**: `28 LW`, `10 Bor.`, `4 2-Cpx`, `2 Name`, `1 3-Cpx`, `1 4-Cpx`, `1 Afx`, например `Nihon` → `10073@Name@Name@@@RH@2014@None@J. Nihon@Japan@@`), `e_rank` возвращает строку `"None"`, записывая литерал `"None"` в колонку ранга при экспорте `Words.txt`.

### LC-5. `WordLinker.add_child` / `add_children` (`loglan_core/addons/word_linker.py:35-36, 58-59`) и семантика `types.parentable`

- **Суть ошибки**: В `Types.txt` и `export.db` флаг `parentable = True (1)` выставлен у типов `Afx`, `1-Cpx..4-Cpx`, `Cpd`, `LW` (т.е. у типов слов, которые **имеют родителей** / происходят от других слов), а у всех примитивов (`C-Prim`, `D-Prim`, `I-Prim`, `L-Prim`, `N-Prim`, `O-Prim`, `S-Prim`) и заимствований `parentable = False (0)`. При этом название поля `parentable` и документация в `WordLinker` вводят в заблуждение, а если попытаться связать производный примитив (`D-Prim`, например `humnu` от `humni`) через `WordLinker.add_child(humni, humnu)`, метод выбросит `TypeError: <BaseWord humnu> is not parentable`, так как у `D-Prim` флаг `parentable == False`.

### LC-6. `BaseWord.notes` (`loglan_core/word.py:264`) и сериализация JSON `'null'` в `export.db`

- **Суть ошибки**: Колонка `notes: Mapped[dict[str, str] | None] = mapped_column(JSON)` объявлена без `JSON(none_as_null=True)`. При пакетной вставке `session.bulk_insert_mappings(Word, words)` в `loglan_convert` значение `notes: None` записывается в SQLite не как `SQL NULL`, а как 4-символьная текстовая строка `'null'` (в `export.db` **9 879 из 10 173 строк** имеют `typeof(notes) = 'text'` и `notes = 'null'`).

### LC-7. `BaseWordSpell` и `T_NAME_WORD_SPELLS` (`loglan_core/word_spell.py` и `loglan_core/service/table_names.py:30`)

- **Суть ошибки**: В `table_names.py` объявлена константа `T_NAME_WORD_SPELLS = "word_spells"` (`"__tablename__ value for BaseWordSpell table"`), однако класс `BaseWordSpell(BaseWord)` в `word_spell.py` не задает `__tablename__ = T_NAME_WORD_SPELLS` и наследует `__tablename__ = "words"` от `BaseWord`. В результате таблица `word_spells` в схеме не существует, а `BaseWordSpell` является подклассом `BaseWord` на той же таблице `words`.

### LC-8. `WordSourcer._get_sources_c_prim` (`loglan_core/addons/word_sourcer.py:98`) и `Exporter.export_word_spell` (`loglan_core/addons/exporter.py:234`)

- **Суть ошибки**:
  1. `_get_sources_c_prim` выполняет `sources = str(word.origin).split(" | ")` без проверки `if not word.origin:`. Если у `C-Prim` поле `origin` пустое или `None`, `str(None)` превращается в `"None"`, и `WordSource("None")` падает с `ValueError: No compatible source found`.
  2. `Exporter.export_word_spell` проверяет `obj.event_end_id if obj.event_end else 9999` (обращаясь к ленивому relationship-объекту `obj.event_end`, что вызывает дополнительный `SELECT` к `events` или `DetachedInstanceError` вне сессии, вместо `if obj.event_end_id is not None:`).

### LC-9. Ошибки в docstring-примерах, ограничениях колонок и аннотациях типов (`loglan_core/*.py`)

- В docstring `BaseWord` (`loglan_core/word.py:62-69`) аргумент `origin=` передан дважды в одном вызове `Word(...)` (`SyntaxError: keyword argument repeated: origin`).
- В docstring `BaseWord.name` (`word.py:125, 201`) указано _"must be unique within the database"_, хотя `unique=True` отсутствует (и не может быть включен из-за омонимов и разных `WordSpell`).
- В `BaseType.type_` (`loglan_core/type.py:52`) и `BaseSetting.db_release` (`loglan_core/setting.py:45, 67`) пропущено ограничение `unique=True` в `mapped_column(...)`.
- В docstring `BaseSyllable.type_` (`syllable.py:60`) указано `max_length=8`, хотя колонка имеет тип `str_032` (`VARCHAR(32)`).
- Во всех конструкторах `__init__` моделей (`BaseAuthor`, `BaseDefinition`, `BaseEvent`, `BaseKey`, `BaseSetting`, `BaseSyllable`, `BaseType`, `BaseWord`) аргументы типизированы как `Mapped[...]` вместо рантайм-типов (`str`, `int`, `bool`, `datetime.date`).

### LC-10. Инвертированное условие проверки в `loglan_convert` (`app/models/postgres/checks.py:109`)

- **Код в `loglan_convert`**:
  ```python
  for source in sources:
      if not session.query(Word).filter(Word.name == source).count() == 0:
          print(f"Word '{source}' from {cpx.name}'s origin is not in the Dictionary")
  ```
- **Суть ошибки**: Условие `if not ... count() == 0:` инвертировано — скрипт печатает ошибку `"is not in the Dictionary"` для каждого слова-источника, которое **присутствует** в словаре, и молчит, когда слово действительно отсутствует!

### LC-11. Потеря целочисленного `0` в `Exporter.merge_by` (`loglan_core/addons/exporter.py:96`)

- **Код в `loglan_core`**:
  ```python
  @staticmethod
  def merge_by(items: Iterable[Any], separator: str) -> str:
      return separator.join([str(i or "") for i in items])
  ```
- **Суть ошибки**: Выражение `i or ""` в Python для числа `0` (`int`) возвращает `""` (так как `bool(0) is False`). В результате любое нулевое числовое значение (например, `Definition.position = 0`, `Event.event_id = 0`, `Setting.db_version = 0`) при экспорте через `Exporter.export(...)` превращается в пустую строку (`Exporter.merge_by([10, 0, "test"], "@") == "10@@test"` вместо `"10@0@test"`). Должно быть `"" if i is None else str(i)`.

### LC-12. Дублирование 46 строк `Words` и 58 строк `WordDefinition` при экспорте из БД в `loglan_convert` (`app/interface.py:41-49`) и `Exporter`

- **Суть ошибки**: При конвертации `.txt` → `export.db` таблицы `Words` (`10 127` записей) и `WordSpell` (`10 173` записи, включая 46 слов с несколькими историческими написаниями, например `id_old = 75` для `alkooli` и `alkoholi`) объединяются в одну таблицу `words` (`10 173` строки), а их определения клонируются (`18 766` строк вместо `18 708`). Когда `DatabaseInterface.default_export` вызывает `session.query(BaseWord).all()` и `session.query(BaseDefinition).all()` и передает каждую строку в `Exporter.export()`, для каждого слова с несколькими написаниями в `Words.txt` и `WordDefinition.txt` выгружаются **полные дубликаты строк с одинаковым `WID` (`id_old`)** (46 дубликатов в `Words.txt` и 58 дубликатов в `WordDefinition.txt`).

### LC-13. Отсутствие `ondelete="CASCADE"` у всех `ForeignKey` в `loglan_core` (`definition.py:55`, `relationships.py:21-51`)

- **Суть ошибки**: В `BaseDefinition.word_id` (`ForeignKey(f"{T_NAME_WORDS}.id")`) и во всех трех связующих таблицах `t_connect_authors`, `t_connect_words`, `t_connect_keys` в `relationships.py` не указан параметр `ondelete="CASCADE"`. В результате в DDL `export.db` все внешние ключи создаются без `ON DELETE CASCADE`, и при включенном `PRAGMA foreign_keys=ON` прямое выполнение `DELETE FROM words WHERE id=?`, `DELETE FROM definitions WHERE id=?` или `DELETE FROM authors WHERE id=?` падает с `sqlite3.IntegrityError: FOREIGN KEY constraint failed`.

---

## 8. Пошаговый план исправления ошибок и оптимизации (согласованный с `torrua/loglan_core`)

### Шаг 1: Приведение схемы БД и IPC-контракта `LOD Manager` в 100% соответствие с `torrua/loglan_core` (Блокеры P0)

1. **Каноническая схема `db::init_schema` и миграции (`src-tauri/src/db.rs`)**:
   - Использовать канонические имена колонок `loglan_core`: `types."group"` и `words."match"` во всех запросах `db.rs`, `import.rs`, `export.rs`, `converter.rs` (добавив безопасную миграцию при открытии БД: если в таблице `types` есть `group_`, переименовать в `"group"`; если в `words` есть `match_`, переименовать в `"match"`).
   - **Убрать** `UNIQUE(name, type)` из `words` в `init_schema` и удалить деструктивную миграцию `migrate_words_unique_if_needed`, чтобы не терять 11 омонимичных/обновленных по событиям слов из `export.db`.
   - Добавить в `init_schema` недостающие таблицы `loglan_core`: `syllables`, `keys`, `connect_keys`, `connect_authors`.
   - Исправить `add_missing_indexes` (`words(type)`, `words(event_start)`, `words(event_end)`).
2. **Поддержка `slots` + `grammar_code` и синхронизация `Definition` (`models.rs`, `db.rs`, `export.rs`, `import.rs`, `converter.rs`, `src/types.ts`, `WordDetail.svelte`)**:
   - При чтении определений объединять `COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, '')`.
   - При сохранении (`save_definition`) и импорте (`import.rs`, `converter.rs`) разделять строку грамматики (например, `"2a"`) на `slots = Some(2)` и `grammar_code = Some("a")`.
   - Выровнять имена полей `Definition` / `SaveDefinition` между Rust и TypeScript.
3. **Поддержка связей `connect_words`, `connect_authors`, формата `words.year` (`DATE`) и JSON `words.notes` (`db::get_word`, `db::list_authors`, `export::generate_html`)**:
   - В `db::get_word` и `export::generate_html`:
     - Извлекать `affixes` из `connect_words` (дочерние слова с `types.type_x = 'Affix'` или `types.type = 'Afx'`, удаляя дефисы `REPLACE(w.name, '-', '')` для случаев вроде `hei-`) + fallback на `word_affixes`.
     - Извлекать `used_in` из `connect_words` (дочерние слова с `types."group" = 'Cpx'`) + fallback на `word_usage`.
     - Для комплексов (`"group" = 'Cpx'`) и слов с родителями извлекать родительские слова (`connect_words.parent_id`), а для остальных — производные не-аффиксы.
     - Извлекать `source` из `connect_authors` (`GROUP_CONCAT(a.abbreviation, '/')`) + `json_extract(w.notes, '$.author')`.
     - Форматировать `w.year` (отрезая суффикс `'-01-01'` от `DATE` из `export.db` и добавляя `json_extract(w.notes, '$.year')`, как в `ExportWordConverter.e_year`) и `w.rank` (добавляя `json_extract(w.notes, '$.rank')`).
     - Нормализовать `w.notes`: если значение равно `'null'`, возвращать `None`; если валидный JSON-объект `{"year", "author", "rank"}` — форматировать читаемо без вывода сырого JSON.
   - В `db::list_authors` считать `word_count` через `LEFT JOIN connect_authors ca ON ca."AID" = a.id`.
4. **Исправление `delete_word`/`delete_definition`/`delete_author`, `save_type`/`delete_type`, `get_event_words`, `save_word`, `list_settings`/`upsert_setting`, `import.rs` и `converter.rs`**:
   - `delete_word`, `delete_definition`, `delete_author`: явно удалять дочерние записи из `connect_keys`, `definitions`, `connect_words`, `connect_authors` перед удалением основной строки (так как в `export.db` внешние ключи созданы без `ON DELETE CASCADE`).
   - `save_type`: `UPDATE types SET type=?1, type_x=?2, "group"=?3 WHERE id=?4`.
   - `get_event_words`: `JOIN events e ON e.event_id = w.event_start WHERE e.id = ?1`.
   - `save_word`: резолвить `events.event_id` по имени события и подставлять дефолтный `event_start = 1`.
   - `list_settings` / `upsert_setting`: работать с колонками `date, db_version, last_word_id, db_release` таблицы `settings`.
   - `import.rs` и `converter.rs`: исправить перепутанные колонки `annotation`/`suffix` в `events` и `parentable`/`description` в `types`, парсить `"True"`/`"False"` без учета регистра, сохранять `origin_x` и `"TID_old"`, обрабатывать `event_end >= 9999` как `NULL`, привязывать определения ко всем словам с данным `id_old` (`HashMap<i64, Vec<i64>>`), заполнять `connect_authors` и `connect_words`.

### Шаг 2: Восстановление тестов, линтеров и CI/CD (P1)

1. Обновить SQL в 15 тестах `src-tauri/src/lib.rs` под каноническую схему `loglan_core` и изолировать `test_convert_text_files` через `temp_dir()`.
2. Перевести `debug_update_check` в `src-tauri/src/lib.rs` на `async fn`.
3. Исправить генерацию `latest.json` в `.github/workflows/release.yml` и URL в `src-tauri/tauri.conf.json`.
4. Исправить все 56 предупреждений Clippy, конвертировать окончания строк `.rs` файлов в `LF` (`cargo fmt`), исключить `.claude/**` и `.planning/**` из ESLint/Prettier и отформатировать фронтенд (`npm run format`).

### Шаг 3: Оптимизация производительности и UX (P2)

1. Заменить `rebuild_fts` в `delete_word` на точечный `fts_update`.
2. Перевести `rebuild_fts` и `compact_db` на использование `with_db`.
3. Избавиться от промежуточной записи во временные файлы в `import_contents`.
4. Реализовать фильтрацию и клавиатурную навигацию для вкладки `Events` в `Sidebar.svelte` и убрать in-place `.sort()` в `autoSelectLatestEvent()`.
5. Удалить отладочные `println!` и `console.log` из горячих путей.
