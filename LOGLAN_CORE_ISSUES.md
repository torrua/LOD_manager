# Технический отчёт: Дефекты и архитектурные несоответствия в `torrua/loglan_core` (v0.3.2) и `loglan_convert`

> **Дата аудита**: 2026-09-30
> **Проверено**: Все 13 проблем подтверждены в актуальном коде на ветке `main` обоих репозиториев.
> **Автор аудита**: LOD Manager team (автоматический аудит + ручная верификация по исходному коду на GitHub).

---

## 1. Контекст проекта и архитектурная роль

- **Проект**: [`torrua/loglan_core`](https://github.com/torrua/loglan_core) — ядро данных и ORM-модели (Python 3.10+, SQLAlchemy 2.0) словаря искусственного языка Логлан (Loglan Online Dictionary, LOD).
- **Связанный проект**: [`torrua/loglan_convert`](https://github.com/torrua/loglan_convert) — ETL/конвертер текстовых файлов базы (`Words.txt`, `WordDefinition.txt` и др.) в SQLite (`export.db`) и обратно.
- **Потребители**: [`LOD Manager`](https://github.com/torrua/LOD_manager) (десктопный и мобильный клиент на Tauri v2 + Rust + Svelte 5), веб-сервисы и боты.
- **Каноническая база**: `export.db` (SQLite 3 + FTS5, 10 173 слова, 18 766 определений).

---

## 2. Сводка выявленных проблем по приоритетам

| ID        | Компонент / Файл                                    |    Серьёзность    | Краткое описание проблемы                                                                                        |
| :-------- | :-------------------------------------------------- | :---------------: | :--------------------------------------------------------------------------------------------------------------- |
| **LC-13** | `loglan_core/definition.py`, `relationships.py`     | **P0 (Critical)** | Отсутствие `ondelete="CASCADE"` у всех `ForeignKey` (падение `DELETE` при `foreign_keys=ON`)                     |
| **LC-2**  | `loglan_core/addons/definition_selector.py`         | **P0 (Critical)** | `DefinitionSelector.by_event`: потеря 40 определений без ключевых слов из-за лишнего `INNER JOIN connect_keys`   |
| **LC-6**  | `loglan_core/word.py`                               | **P0 (Critical)** | Колонка `words.notes` сериализует Python `None` в строку `'null'` (9 879 записей) вместо `SQL NULL`              |
| **LC-11** | `loglan_core/addons/exporter.py`                    | **P0 (Critical)** | `Exporter.merge_by` теряет целочисленный `0` (`i or ""` превращает `0` в пустую строку)                          |
| **LC-12** | `loglan_convert/app/interface.py`, `exporter.py`    | **P0 (Critical)** | Экспорт в `.txt` дублирует 46 строк `Words` и 58 строк `WordDefinition` из-за `id_old`                           |
| **LC-10** | `loglan_convert/app/models/postgres/checks.py`      |   **P1 (High)**   | Инвертированное условие проверки (`if not ... count() == 0`) — ругается на корректные слова                      |
| **LC-1**  | `loglan_core/addons/base_selector.py`, `filters.py` |   **P1 (High)**   | Сломан регистрозависимый поиск `GLOB` в SQLite (`*` заменяется на `%`); нет замены `?` на `_` для `LIKE`         |
| **LC-3**  | `loglan_core/addons/base_selector.py`               |   **P1 (High)**   | `BaseSelector.select_columns` стирает все `JOIN`, `ORDER BY`, `LIMIT`, `OFFSET` вместо `with_only_columns`       |
| **LC-4**  | `loglan_core/addons/export_word_converter.py`       |   **P1 (High)**   | `ExportWordConverter.e_rank` экспортирует литеральную строку `"None"` при пустом ранге (47 слов)                 |
| **LC-8**  | `loglan_core/addons/word_sourcer.py`, `exporter.py` |   **P1 (High)**   | Падение `WordSourcer` при `origin=None` (`"None"` → `ValueError`); lazy load `obj.event_end` вне сессии          |
| **LC-5**  | `loglan_core/addons/word_linker.py`                 |  **P2 (Medium)**  | Метод `add_child` падает с `TypeError` при попытке связать производные примитивы (`D-Prim`, `parentable=0`)      |
| **LC-7**  | `loglan_core/word_spell.py`, `table_names.py`       |  **P2 (Medium)**  | Несоответствие `T_NAME_WORD_SPELLS = "word_spells"`: класс `BaseWordSpell` наследует имя таблицы `words`         |
| **LC-9**  | `loglan_core/*.py`                                  |   **P3 (Low)**    | Повторный аргумент `origin` в docstring `BaseWord`; конструкторы аннотированы `Mapped[...]` вместо runtime-типов |

---

## 3. Детальное описание дефектов и инструкции по исправлению

### LC-13. Отсутствие `ondelete="CASCADE"` у связей ForeignKey

- **Файлы**:
  - `loglan_core/definition.py` (~строка 83)
  - `loglan_core/relationships.py` (~строки 21–51) — `t_connect_authors`, `t_connect_words`, `t_connect_keys`
- **Проблема**:
  В SQLAlchemy-определениях внешних ключей отсутствует параметр `ondelete="CASCADE"`:
  ```python
  # Сейчас в relationships.py:
  Column("AID", Integer, ForeignKey(f"{T_NAME_AUTHORS}.id"), primary_key=True),
  Column("WID", Integer, ForeignKey(f"{T_NAME_WORDS}.id"), primary_key=True),
  ```
  В сгенерированной DDL-схеме SQLite/PostgreSQL внешние ключи не каскадируются. Когда внешнее приложение (например, `LOD Manager` или SQLite-консоль с `PRAGMA foreign_keys=ON;`) удаляет слово, определение или автора, операция прерывается с ошибкой: `sqlite3.IntegrityError: FOREIGN KEY constraint failed`.
- **Решение**:
  Указать `ondelete="CASCADE"` во всех внешних ключах:
  ```python
  # loglan_core/definition.py:
  word_id: Mapped[int] = mapped_column(
      ForeignKey(f"{T_NAME_WORDS}.id", ondelete="CASCADE"), nullable=False
  )

  # loglan_core/relationships.py:
  Column("AID", Integer, ForeignKey(f"{T_NAME_AUTHORS}.id", ondelete="CASCADE"), primary_key=True),
  Column("WID", Integer, ForeignKey(f"{T_NAME_WORDS}.id", ondelete="CASCADE"), primary_key=True),
  # аналогично для parent_id, child_id в t_connect_words и KID, DID в t_connect_keys
  ```

---

### LC-2. `DefinitionSelector.by_event`: потеря определений без ключевых слов

- **Файл**: `loglan_core/addons/definition_selector.py` (~строка 92)
- **Проблема**:
  Метод фильтрации определений по событию содержит лишний `join`:
  ```python
  subquery = (
      select(self.model.id)
      .join(t_connect_keys)  # <--- ОШИБКА: лишний INNER JOIN
      .join(BaseWord)
      .where(filter_word_by_event_id(event_id))
      .scalar_subquery()
  )
  ```
  У модели `BaseDefinition` есть прямое поле `word_id`, ссылающееся на `BaseWord.id`. Промежуточный `INNER JOIN t_connect_keys` отбрасывает **все определения, у которых нет ключевых слов** (в `export.db` это 40 определений, включая определения аффиксов и служебные глоссы).
- **Решение**:
  Убрать `.join(t_connect_keys)`:
  ```python
  subquery = (
      select(self.model.id)
      .join(BaseWord, self.model.word_id == BaseWord.id)
      .where(filter_word_by_event_id(event_id))
      .scalar_subquery()
  )
  ```

---

### LC-6. Сериализация JSON `'null'` вместо `SQL NULL` в `words.notes`

- **Файл**: `loglan_core/word.py` (~строка 264) и скрипты вставки в `loglan_convert`
- **Проблема**:
  Колонка `notes` объявлена как:
  ```python
  notes: Mapped[dict[str, str] | None] = mapped_column(JSON)
  ```
  При пакетном импорте `bulk_insert_mappings` в SQLAlchemy значение `notes=None` по умолчанию сериализуется в строку `'null'` (4 байта текста). В `export.db` из 10 173 слов **9 879 строк** содержат текстовую строку `'null'` (`typeof(notes) = 'text'`), и лишь у 294 слов записан реальный JSON-словарь. Любой клиент вынужден писать специальные костыли, проверяя `if notes == 'null'`.
- **Решение**:
  1. В `loglan_core/word.py` использовать `JSON(none_as_null=True)`:
     ```python
     notes: Mapped[dict[str, str] | None] = mapped_column(JSON(none_as_null=True))
     ```
  2. В `loglan_convert` при формировании словарей для вставки явно передавать `None`, а в миграции выполнить:
     ```sql
     UPDATE words SET notes = NULL WHERE notes = 'null' OR notes = '';
     ```

---

### LC-11. `Exporter.merge_by` теряет `0` (ноль)

- **Файл**: `loglan_core/addons/exporter.py` (~строка 96)
- **Проблема**:
  ```python
  @staticmethod
  def merge_by(items: Iterable[Any], separator: str) -> str:
      return separator.join([str(i or "") for i in items])
  ```
  В Python `bool(0) is False`, поэтому выражение `0 or ""` возвращает пустую строку `""`. При экспорте моделей, содержащих целочисленный 0 (например, `Definition.position = 0`, `Event.event_id = 0`, `Setting.db_version = 0`), ноль превращается в пустоту: `"10@@test"` вместо `"10@0@test"`.
- **Решение**:
  ```python
  @staticmethod
  def merge_by(items: Iterable[Any], separator: str) -> str:
      return separator.join(["" if i is None else str(i) for i in items])
  ```

---

### LC-12. Дублирование записей при экспорте из БД в текстовые файлы

- **Файлы**: `loglan_convert/app/interface.py` (~строки 41–49), `loglan_core/addons/exporter.py`
- **Проблема**:
  При начальном импорте из `.txt` в SQLite таблицы `Words` (10 127) и `WordSpell` (10 173) схлопываются в единую таблицу `words`. У 46 слов есть несколько исторических написаний (например, `id_old = 75` для `alkooli` и `alkoholi`), при этом определения дублируются для каждого написания (18 766 определений вместо 18 708 исходных).
  Когда `DatabaseInterface.default_export` выгружает данные через `session.query(BaseWord).all()`, для каждого такого написания в `Words.txt` и `WordDefinition.txt` выгружаются **полные дубликаты строк с одинаковым `WID` (`id_old`)** (46 дубликатов в `Words.txt`, 58 в `WordDefinition.txt`).
- **Решение**:
  При экспорте в `Words.txt` группировать по `id_old` (выбирая актуальное написание или разделяя логику `Word` и `WordSpell`), а при экспорте `WordDefinition.txt` фильтровать уникальные `(id_old, position, body)`:
  ```python
  # Использовать DISTINCT или агрегацию по id_old перед отправкой в Exporter
  ```

---

### LC-10. Инвертированное условие валидации в `loglan_convert`

- **Файл**: `loglan_convert/app/models/postgres/checks.py` (~строка 109)
- **Проблема**:
  ```python
  for source in sources:
      if not session.query(Word).filter(Word.name == source).count() == 0:
          print(f"Word '{source}' from {cpx.name}'s origin is not in the Dictionary")
  ```
  Условие `if not ... == 0` истинно тогда, когда слово **найдено** в базе (`count > 0`). Скрипт выдает ложные предупреждения для валидных слов и пропускает реальные ошибки!
- **Решение**:
  ```python
  for source in sources:
      if session.query(Word).filter(Word.name == source).count() == 0:
          print(f"Word '{source}' from {cpx.name}'s origin is not in the Dictionary")
  ```

---

### LC-1. Сломанный регистрозависимый поиск `GLOB` в SQLite и неполная трансляция wildcard

- **Файлы**:
  - `loglan_core/addons/base_selector.py` (~строка 193)
  - `loglan_core/addons/filters.py` (~строка 49)
- **Проблема**:
  ```python
  value = value.replace("*", "%")

  if not self.case_sensitive:
      return column.ilike(value)

  if self.is_sqlite:
      return column.op("GLOB")(value)

  return column.like(value)
  ```
  1. Оператор SQLite `GLOB` чувствителен к регистру, но использует glob-шаблоны: `*` (любая подстрока) и `?` (один символ). Заменяя `*` на `%`, код формирует запрос `WHERE words.name GLOB 'pru%'`. В `GLOB` символ `%` является обычным литералом, поэтому поиск по шаблону `pru*` возвращает **0 результатов**!
  2. В ветках `LIKE` / `ILIKE` символ `?` не заменяется на `_` (одиночный символ в SQL LIKE).
- **Решение**:
  ```python
  if self.is_sqlite and self.case_sensitive:
      # Для SQLite GLOB '*' и '?' уже являются нативными подстановочными знаками
      return column.op("GLOB")(value)

  # Для SQL LIKE / ILIKE преобразуем glob в SQL wildcards
  sql_value = value.replace("*", "%").replace("?", "_")
  if not self.case_sensitive:
      return column.ilike(sql_value)
  return column.like(sql_value)
  ```

---

### LC-3. `BaseSelector.select_columns` стирает контекст запроса

- **Файл**: `loglan_core/addons/base_selector.py` (~строки 64–80)
- **Проблема**:
  ```python
  def select_columns(self, ...):
      # ...
      self._statement = select(*self._selected_columns).where(existing_conditions)
  ```
  Пересоздание через `select(...)` сохраняет только фильтры `where`, но **уничтожает** ранее добавленные `.join(...)`, `.order_by(...)`, `.limit(...)`, `.offset(...)` и `.options(...)`.
- **Решение**:
  Использовать штатный метод SQLAlchemy 2.0:
  ```python
  self._statement = self._statement.with_only_columns(*self._selected_columns)
  ```

---

### LC-4. `ExportWordConverter.e_rank` возвращает литерал `"None"`

- **Файл**: `loglan_core/addons/export_word_converter.py` (~строки 92–100)
- **Проблема**:
  ```python
  @property
  def e_rank(self) -> str:
      notes: dict[str, str] = self.word.notes or {}
      return f"{self.word.rank} {notes.get('rank', str())}".strip()
  ```
  Если `self.word.rank is None` (в `export.db` это 47 слов), форматированная f-строка подставляет строковый литерал `"None"`. В результате при экспорте в текстовый файл поле ранга заполняется строкой `"None"` вместо пустой строки.
- **Решение**:
  ```python
  @property
  def e_rank(self) -> str:
      notes: dict[str, str] = self.word.notes or {}
      rank_str = str(self.word.rank) if self.word.rank is not None else ""
      extra_rank = notes.get("rank", "")
      return f"{rank_str} {extra_rank}".strip()
  ```

---

### LC-8. Падение `WordSourcer` при отсутствии `origin` и lazy load в `Exporter`

- **Файлы**:
  - `loglan_core/addons/word_sourcer.py` (~строка 125)
  - `loglan_core/addons/exporter.py` (~строка 230)
- **Проблема**:
  1. В `_get_sources_c_prim`: вызов `str(word.origin).split(" | ")` при `origin=None` даёт `["None"]`, что приводит к `ValueError: No compatible source found`.
  2. В `export_word_spell`: проверка `obj.event_end_id if obj.event_end else 9999` обращается к lazy-relationship `obj.event_end`, вызывая скрытый SQL-запрос или падение `DetachedInstanceError`, если объект находится вне сессии.
- **Решение**:
  1. В `word_sourcer.py`:
     ```python
     if not word.origin:
         return []
     sources = word.origin.split(" | ")
     ```
  2. В `exporter.py`: проверять непосредственно скалярное поле:
     ```python
     event_end_id = obj.event_end_id if obj.event_end_id is not None else 9999
     ```

---

### LC-5. `WordLinker.add_child`: ложный `TypeError` на производных примитивах

- **Файл**: `loglan_core/addons/word_linker.py` (~строки 35–39)
- **Проблема**:
  В таблице `types` флаг `parentable = 1` выставлен у типов, которые **имеют родителей** (комплексы `1-Cpx..4-Cpx`, аффиксы `Afx`, составные слова `Cpd`, заимствования `LW`). У примитивов (`D-Prim`, `C-Prim` и др.) `parentable = 0`.
  При попытке связать производный примитив (например, `humnu` (D-Prim), образованный от `humni`) метод `WordLinker.add_child(humni, humnu)` проверяет `child.type.parentable` и выбрасывает исключение: `TypeError: <BaseWord humnu> is not parentable`.
- **Решение**:
  Пересмотреть логику проверки: либо разрешить связывание производных примитивов (`D-Prim`), либо уточнить документацию и название атрибута (`has_parents` vs `can_have_children`), чтобы валидация соответствовала морфологии Логлана.

---

### LC-7. Таблица `BaseWordSpell` и константа `T_NAME_WORD_SPELLS`

- **Файлы**:
  - `loglan_core/service/table_names.py` (~строка 29): `T_NAME_WORD_SPELLS = "word_spells"`
  - `loglan_core/word_spell.py`
- **Проблема**:
  Класс `BaseWordSpell` наследуется от `BaseWord`, но не переопределяет `__tablename__`, оставаясь на таблице `words`. Константа `T_NAME_WORD_SPELLS` в `table_names.py` не используется ни одной таблицей схемы.
- **Решение**:
  Либо явно задокументировать, что `BaseWordSpell` использует Single Table Inheritance / полиморфную модель на таблице `words`, либо удалить/скорректировать неиспользуемую константу `T_NAME_WORD_SPELLS`, чтобы не путать разработчиков сторонних клиентов.

---

### LC-9. Ошибки в аннотациях типов и docstrings

- **Файлы**: `loglan_core/*.py`
- **Проблема**:
  1. `loglan_core/word.py` (~строки 53–63): в примере docstring для вызова конструктора аргумент `origin=` передан дважды (`SyntaxError`).
  2. Во всех конструкторах `__init__` аргументы типизированы как `Mapped[str]` вместо `str | None`.
  3. В `BaseType.type_` (`type.py:52`) и `BaseSetting.db_release` (`setting.py:45`) не хватает `unique=True`.
  4. В docstring `BaseSyllable.type_` (`syllable.py:60`) указано `max_length=8`, хотя колонка имеет тип `str_032` (`VARCHAR(32)`).
- **Решение**:
  Исправить опечатки в примерах docstring и привести аннотации аргументов `__init__` к типам Python runtime (`str`, `int`, `bool`, `datetime.date`).

---

## 4. Рекомендуемый план действий

### Шаг 1: Подготовка окружения

1. Склонировать `torrua/loglan_core`, поднять виртуальное окружение (`poetry install` или `pip install -e ".[dev]"`).
2. Запустить `pytest` для фиксации текущего состояния тестов.

### Шаг 2: P0 — Data Integrity & Queries

1. Добавить `ondelete="CASCADE"` в `definition.py` и `relationships.py` (**LC-13**).
2. Убрать `.join(t_connect_keys)` из `DefinitionSelector.by_event` (**LC-2**).
3. Исправить `Exporter.merge_by` для сохранения `0` (**LC-11**).
4. Задать `JSON(none_as_null=True)` для `BaseWord.notes` (**LC-6**).
5. Исправить дублирование при экспорте в `loglan_convert` (**LC-12**).

### Шаг 3: P1 — Selectors & Converters

1. Исправить логику `GLOB` и wildcard в `BaseSelector` и `filters.py` (**LC-1**).
2. Заменить пересоздание запроса в `select_columns` на `.with_only_columns(...)` (**LC-3**).
3. Исправить `ExportWordConverter.e_rank` (**LC-4**) и `word_sourcer.py` (**LC-8**).
4. Исправить инвертированную проверку в `loglan_convert` (**LC-10**).

### Шаг 4: P2 / P3 — Валидация и документация

1. Исправить `WordLinker` (**LC-5**).
2. Разобраться с `BaseWordSpell` / `T_NAME_WORD_SPELLS` (**LC-7**).
3. Исправить docstrings и аннотации (**LC-9**).

### Шаг 5: Верификация

1. Написать unit-тесты в `tests/` на каждый исправленный кейс:
   - `GLOB` wildcard-поиск по SQLite и PostgreSQL
   - `by_event` на определениях без ключевых слов в `connect_keys`
   - Каскадное удаление слов, определений и авторов
   - Сохранение целочисленного `0` при экспорте (`merge_by`)
   - Корректная сериализация `notes=None` как `SQL NULL`
   - Экспорт `e_rank` для слов с `rank=None`
   - `WordSourcer` для слов с `origin=None`
2. Прогнать полный цикл `pytest` и `flake8` / `ruff` / `mypy`.
