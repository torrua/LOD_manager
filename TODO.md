# TODO.md — План исправления ошибок и оптимизации LOD Manager (согласованный с `torrua/loglan_core`)

> Приоритизированный чек-лист по результатам архитектурного аудита и сверки с [`torrua/loglan_core`](https://github.com/torrua/loglan_core) ([`AUDIT_REPORT.md`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/AUDIT_REPORT.md)).

---

## P0 — Критические ошибки времени выполнения и совместимость с `torrua/loglan_core` (`export.db`)

- [x] **[P0-1] Исправить рассинхронизацию полей `Definition` / `SaveDefinition` и поддержку `slots` + `grammar_code`**
  - Файлы: [`src-tauri/src/models.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/models.rs#L33-L69), [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L413-L436), [`src-tauri/src/export.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/export.rs#L183-L195), [`src/types.ts`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/types.ts#L7-L14), [`src/lib/components/WordDetail.svelte`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components/WordDetail.svelte#L129-L153)
  - Действие:
    - Выровнять имена полей между Rust/SQLite и TypeScript/Svelte (`grammar`/`grammar_code` и `tags`/`case_tags`), чтобы в `WordDetail.svelte` отображались грамматические коды и теги, а сохранение определения не затирало их в `NULL`.
    - При чтении определений склеивать `NULLIF(COALESCE(CAST(d.slots AS TEXT), '') || COALESCE(d.grammar_code, ''), '')` (в `export.db` у 8 558 определений число слотов хранится в `slots`).
    - При сохранении (`save_definition`) и импорте (`import.rs`, `converter.rs`) разделять строку вида `"2a"` на `slots = Some(2)` и `grammar_code = Some("a")` в соответствии с `loglan_core`.

- [x] **[P0-2] Исправить `db::save_type` (`SET type=?1, "group"=?3`) и `db::delete_type` (`NOT NULL` на `words.type`)**
  - Файл: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L681-L701)
  - Действие: Заменить `UPDATE types SET name=?1...` на `UPDATE types SET type=?1, type_x=?2, "group"=?3 WHERE id=?4`; в `delete_type` запретить обнуление `NOT NULL` колонки `words.type` при наличии связанных слов.

- [x] **[P0-3] Исправить `db::get_event_words` (`event_start` / `event_end`)**
  - Файл: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L1202-L1221)
  - Действие: Заменить несуществующие `w.event_start_id` и `w.event_end_id` на выборку через `w.event_start` и `w.event_end` с привязкой к `events.event_id`.

- [x] **[P0-4] Исправить сохранение событий (`event_start`/`event_end = 9999`), маппинг `id_old -> Vec<word_id>`, порядок колонок `events`/`types`, парсинг `"False"` и `NOT NULL` ограничения в `db::save_word`, `import.rs` и `converter.rs`**
  - Файлы: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L474-L535), [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs#L138-L362), [`src-tauri/src/converter/converter.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/converter/converter.rs#L135-L593)
  - Действие:
    - В `save_word` искать `SELECT event_id FROM events WHERE name=?1` (вместо `SELECT id FROM events WHERE event_id=?1`) и использовать дефолтный `event_start = 1`.
    - В `import.rs` исправить перепутанные колонки `annotation` (`r[4]`) и `suffix` (`r[5]`) в `events`, а также `parentable` (`r[3]`) и `description` (`r[4]`) в `types`; в `converter.rs:151,308` парсить булевы строки `"True"`/`"False"` без учета регистра (`eq_ignore_ascii_case("true")`).
    - В `import.rs` и `converter.rs` трактовать `event_end >= 9999` в `WordSpell.txt` как `NULL` (активное слово), обновлять предсозданное событие `event_id = 1` (`ON CONFLICT(event_id) DO UPDATE`), маппить `id_old -> Vec<word_id>` (чтобы все 46 слов с несколькими написаниями получали определения, как в `loglan_convert`) и сохранять `origin_x` и `"TID_old"`.

- [x] **[P0-5] Убрать ошибочное ограничение `UNIQUE(name, type)` на `words` и исправить миграции при открытии БД**
  - Файл: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L42-L58) и [строки 141–288](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L141-L288)
  - Действие:
    - Удалить `UNIQUE(name, type)` из `db::init_schema` и удалить деструктивную миграцию `migrate_words_unique_if_needed` (в `loglan_core` / `export.db` есть 11 пар слов с одинаковыми `(name, type)` в разных интервалах `event_start`/`event_end`, которые сейчас теряются при импорте).
    - В `add_missing_indexes` заменить `words(type_id)`, `words(event_start_id)`, `words(event_end_id)` на `words(type)`, `words(event_start)`, `words(event_end)`.
    - Убрать обращение к несуществующей колонке `events.notes` в `migrate_event_columns_if_needed`.

- [x] **[P0-6] Привести `list_settings` и `upsert_setting` к схеме `settings` из `torrua/loglan_core`**
  - Файлы: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L99-L108), [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L791-L820), [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs#L378-L403)
  - Действие: Читать и обновлять метаданные релиза (`date, db_version, last_word_id, db_release`) в таблице `settings` по схеме `loglan_core`, чтобы вкладка **Tools → Database** (`get_db_stats`) работала и на `export.db`, и на новых БД.

- [x] **[P0-7] Обеспечить 100% совместимость с канонической схемой `torrua/loglan_core` (`"group"`, `"match"`, `connect_words`, `connect_authors`, `words.year` `DATE`, JSON `notes`, `syllables`, `keys`)**
  - Файлы: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs), [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs), [`src-tauri/src/export.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/export.rs), [`src-tauri/src/converter/converter.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/converter/converter.rs)
  - Действие:
    - Использовать канонические имена колонок `loglan_core`: `types."group"` и `words."match"` во всех модулях (с безопасной миграцией `group_ -> "group"` и `match_ -> "match"` для промежуточных БД, не ломая совместимость `export.db` с `loglan_core`).
    - Добавить в `db::init_schema` таблицы `syllables`, `keys`, `connect_keys`, `connect_authors`.
    - В `db::get_word` и `export::generate_html` извлекать `affixes` (`type_x = 'Affix'`, удаляя дефисы `REPLACE(w.name, '-', '')` для `hei-`) и `used_in` (`"group" = 'Cpx'`) из `connect_words` (с fallback на `word_affixes` / `word_usage`), а для комплексов показывать родительские слова (`connect_words.parent_id`).
    - В `db::get_word` формировать `source` из `connect_authors` + `notes.author`, а в `db::list_authors` считать `word_count` по `connect_authors`.
    - В `db::get_word` и `export::generate_html` форматировать `words.year` (`'YYYY-01-01'` → `'YYYY'` + `notes.year`) и `words.rank` (+ `notes.rank`), а также корректно обрабатывать JSON в `words.notes` (превращать строку `'null'` из `export.db` в `None`).

- [x] **[P0-8] Исправить `FOREIGN KEY constraint failed` при удалении слов, определений и авторов в `export.db`**
  - Файл: [`src-tauri/src/db.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/db.rs#L564-L740) (`delete_word`, `delete_definition`, `delete_author`)
  - Действие: Поскольку в `export.db` (схема `loglan_core`) у внешних ключей `definitions`, `connect_words`, `connect_authors`, `connect_keys` отсутствует `ON DELETE CASCADE`, а `LOD Manager` включает `PRAGMA foreign_keys=ON`, перед удалением родительской записи явно удалять дочерние строки из связующих таблиц в транзакции.

---

## P1 — Тесты, сборка, линтеры и стабильность Desktop Updater

- [x] **[P1-1] Починить все 16 падающих тестов в `cargo test` и добавить тесты на схему `loglan_core`**
  - Файлы: [`src-tauri/src/lib.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/lib.rs#L145-L756), [`src-tauri/src/converter/tests.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/converter/tests.rs#L8-L60)
  - Действие: Обновить тестовые SQL-запросы под каноническую схему `loglan_core` (`types.type`, `types."group"`, `words.type`, `words."match"`, `words.id_old`, `words.event_start`, `events.definition`, `authors.abbreviation`), добавить тесты для `slots + grammar_code`, `connect_words`, `connect_authors`, JSON `notes` и перевести `test_convert_text_files` на временную папку в `std::env::temp_dir()`.

- [x] **[P1-2] Убрать панику `block_on` в `debug_update_check`**
  - Файл: [`src-tauri/src/lib.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/lib.rs#L93-L123)
  - Действие: Сделать `debug_update_check` асинхронной командой (`async fn`) и использовать `updater.check().await`.

- [x] **[P1-3] Исправить генерацию `latest.json` в CI и endpoint автообновления**
  - Файлы: [`.github/workflows/release.yml`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/.github/workflows/release.yml#L95), [`src-tauri/tauri.conf.json`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/tauri.conf.json#L54)
  - Действие: Добавить пропущенную кавычку `\"windows-x86_64-msi\"` в `release.yml` и обновить endpoint в `tauri.conf.json` на `https://github.com/torrua/LOD_manager/releases/latest/download/latest.json`.

- [x] **[P1-4] Устранить все ошибки Clippy (56), rustfmt (CRLF), ESLint и Prettier**
  - Файлы: `src-tauri/src/**/*.rs`, `eslint.config.js`, `.prettierignore`, `src/**/*`
  - Действие:
    - Исключить `.claude/**` и `.planning/**` из ESLint и Prettier (`✅ выполнено`).
    - Выполнить `npm run rust:fmt` (с конвертацией в `LF`) и исправить 56 замечаний `cargo clippy` (`✅ выполнено`).
    - Выполнить `npm run format` для фронтенда (`✅ выполнено`).

---

## P2 — Оптимизация производительности и UX

- [x] **[P2-1] Инкрементальное обновление FTS5 при удалении слова**
  - Файл: [`src-tauri/src/commands/words.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/commands/words.rs#L46-L52)
  - Действие: Заменить полный `db::rebuild_fts(conn)` при `delete_word` на удаление записей определений удаляемого слова через `db::fts_update(conn, def_id, "")`.

- [x] **[P2-2] Перевести `rebuild_fts` и `compact_db` на `with_db`**
  - Файл: [`src-tauri/src/commands/search.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/commands/search.rs#L41-L78)
  - Действие: Использовать существующее соединение из `AppState.db` вместо открытия второго `Connection::open(&path)`.

- [x] **[P2-3] Прямой in-memory импорт в `import_contents` и проверка `tx.commit()`**
  - Файл: [`src-tauri/src/import.rs`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs#L51-L87) и [строка 374](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src-tauri/src/import.rs#L374)
  - Действие: Избавиться от записи временных файлов в `temp_dir` при Android-импорте и обрабатывать ошибки `tx.commit()?`.

- [x] **[P2-4] Реализовать поиск и навигацию с клавиатуры для списка Events**
  - Файл: [`src/lib/components/Sidebar.svelte`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/components/Sidebar.svelte#L316-L335)
  - Действие: Фильтровать `app.events` по `app.searchQ` на вкладке `events` и исправить `itemKeydown` для переключения между событиями.

- [x] **[P2-5] Исправить `NaN`-сортировку дат (`date === ""`) и мутацию `$state`-массива в `autoSelectLatestEvent`**
  - Файл: [`src/lib/store.svelte.ts`](file:///c:/Users/User/OneDrive/JS/LOD%20Manager/src/lib/store.svelte.ts#L364-L373)
  - Действие: Отфильтровывать пустые строки `e.date?.trim()` перед `new Date(e.date).getTime()` и использовать `[...app.events].sort(...)` вместо `app.events.sort(...)`.

- [x] **[P2-6] Удалить отладочные `println!` и `console.log` из продакшн-кода**
  - Файлы: `src-tauri/src/db.rs`, `src-tauri/src/commands/*.rs`, `src-tauri/src/converter/converter.rs`, `src/lib/store.svelte.ts`

---

## P3 — Технический долг и гигиена репозитория

- [x] **[P3-1] Защитить секреты и временные артефакты в `.gitignore`, `.prettierignore`, `eslint.config.js`** (`выполнено`)
- [x] **[P3-2] Синхронизировать версию `1.7.0` в `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `CHANGELOG.md` и `README.md`** (`выполнено`)
- [x] **[P3-3] Удалить пустые директории (`src/lib/{composables,repositories,services,stores}`, `.github/workflows/.github`) и неиспользуемые заглушки (`DeleteModal.svelte`, `_placeholder.ts`)** (`выполнено`)
- [x] **[P3-4] Каскадное обновление `word_usage.used_in_word` при переименовании слова в `db::save_word`** (`выполнено`)
- [x] **[P3-5] Ввести типизированный `enum AppError` вместо строковых ошибок `Result<T, String>`** (`выполнено`)

---

## Upstream — Рекомендуемые исправления в репозиториях `torrua/loglan_core` и `loglan_convert`

- [ ] **[LC-1] `BaseSelector.get_like_condition` (`loglan_core/addons/base_selector.py:193-201`) и `filter_key_by_word_cs` (`loglan_core/addons/filters.py:49-54`)**: не заменять `*` на `%` перед вызовом `op("GLOB")` при `is_sqlite=True, case_sensitive=True` (сейчас `GLOB 'pru%'` и `GLOB 'ab%'` возвращают 0 строк) и заменять `?` на `_` для `LIKE`/`ILIKE`.
- [ ] **[LC-2] `DefinitionSelector.by_event` (`loglan_core/addons/definition_selector.py:72-80`)**: убрать лишний `.join(t_connect_keys)` перед `.join(BaseWord)`, который отбрасывает 40 определений без ключевых слов `«...»`.
- [ ] **[LC-3] `BaseSelector.select_columns` (`loglan_core/addons/base_selector.py:64-80`)**: использовать `self._statement.with_only_columns(*columns)` вместо пересоздания `select(...)`, чтобы не терять `JOIN`, `ORDER BY`, `LIMIT`, `OFFSET`.
- [ ] **[LC-4] `ExportWordConverter.e_rank` (`loglan_core/addons/export_word_converter.py:114-122`)**: проверять `self.word.rank` на `None`, чтобы не экспортировать литеральную строку `"None"` для 47 слов с `rank IS NULL` (например, `Nihon`).
- [ ] **[LC-5] `WordLinker.add_child` / `add_children` (`loglan_core/addons/word_linker.py:35-36, 58-59`)**: уточнить семантику `child.type.parentable` (сейчас не позволяет связывать производные примитивы `D-Prim`).
- [ ] **[LC-6] `BaseWord.notes` (`loglan_core/word.py:264`)**: использовать `mapped_column(JSON(none_as_null=True))`, чтобы `None` при `bulk_insert_mappings` сохранялся как `SQL NULL`, а не как строка `'null'`.
- [ ] **[LC-7] `BaseWordSpell` (`loglan_core/word_spell.py` и `service/table_names.py:30`)**: устранить противоречие между `T_NAME_WORD_SPELLS = "word_spells"` и наследованием `__tablename__ = "words"` от `BaseWord`.
- [ ] **[LC-8] `WordSourcer._get_sources_c_prim` (`loglan_core/addons/word_sourcer.py:98`) и `Exporter.export_word_spell` (`exporter.py:234`)**: добавить проверку `if not word.origin: return []` и `if obj.event_end_id is not None:`.
- [ ] **[LC-9] Докстринги, ограничения `unique=True` и аннотации типов (`loglan_core/*.py`)**: исправить дублирующийся аргумент `origin=` в docstring `BaseWord`, добавить пропущенный `unique=True` на `BaseType.type_` и `BaseSetting.db_release`, убрать ошибочное утверждение об уникальности `BaseWord.name`, исправить `max_length` в docstring `BaseSyllable.type_` и заменить `Mapped[...]` в сигнатурах `__init__` на рантайм-типы.
- [ ] **[LC-10] `loglan_convert` (`app/models/postgres/checks.py:109`)**: исправить инвертированное условие `if session.query(Word).filter(Word.name == source).count() == 0:`.
- [ ] **[LC-11] `Exporter.merge_by` (`loglan_core/addons/exporter.py:96`)**: заменить `[str(i or "") for i in items]` на `["" if i is None else str(i) for i in items]`, чтобы целочисленный `0` (`position=0`, `event_id=0`, `db_version=0`) не превращался в пустую строку `""`.
- [ ] **[LC-12] `DatabaseInterface.default_export` (`loglan_convert/app/interface.py:41-49`) и `Exporter`**: дедуплицировать выгрузку `Words` и `WordDefinition` по `id_old`, чтобы не порождать 46 дубликатов в `Words.txt` и 58 дубликатов в `WordDefinition.txt` для слов с несколькими историческими написаниями (`WordSpell`).
- [ ] **[LC-13] `BaseDefinition.word_id` (`definition.py:55`) и `relationships.py:21-51`**: добавить `ondelete="CASCADE"` во все `ForeignKey(...)`, чтобы удаление слов, определений, авторов и ключей в SQLite при `PRAGMA foreign_keys=ON` не падало с `FOREIGN KEY constraint failed`.
