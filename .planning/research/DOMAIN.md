# Domain Research: Loglan and the LOD

## What is Loglan?

Loglan is a constructed logical language designed in the 1950s by James Cooke Brown. It was created to test hypotheses about linguistic relativity and to serve as a potential international auxiliary language. Key characteristics:

- **Logical structure**: Designed to be unambiguous and computationally parseable
- **Vocabulary**: Based primarily on English and other Indo-European roots
- **Grammar**: Uses particles (cmavo) and content words (gismu)

## Loglan Online Dictionary (LOD)

The LOD is a comprehensive dictionary of Loglan vocabulary maintained by the Loglan Institute. It contains:

- **Words** (gismu, lujvo, fu'ivla): ~10,000+ entries
- **Definitions**: Multiple definitions per word with grammar, usage notes
- **Types**: Word type classification (gismu, cvu, bangu, etc.)
- **Authors**: Attribution for word creation/modification
- **Events**: Lexical events tracking dictionary changes

## File Format (@ delimited)

The original LOD uses custom text files with `@` delimiters:

```
@Word:camgu
@Type:gismu
@Definition:grammar=GU,vorenhancement,vorenhancement
...
```

### Input Files

| File                 | Contents                                     |
| -------------------- | -------------------------------------------- |
| `Words.txt`          | Word metadata (type, rank, match %, origins) |
| `WordSpell.txt`      | Spellings and event references               |
| `WordDefinition.txt` | Definitions with grammar, usage              |
| `LexEvent.txt`       | Lexical events                               |
| `Author.txt`         | Authors                                      |
| `Type.txt`           | Word types                                   |

## Technical Context

### Database Schema

SQLite database with FTS5 (Full-Text Search) for English definitions:

- **words**: Core vocabulary entries
- **definitions**: Multiple definitions per word
- **types**: Word type classifications
- **events**: Lexical change events
- **authors**: Attribution
- **def_fts**: FTS index for definition body
- **def_kw_fts**: FTS index for keyword markers

### Search Features

- **Loglan → English**: Browse words by type, filter by event
- **English → Loglan**: Full-text search in definitions
- **Keyword search**: Search within «keyword» markers
- **FTS fallback**: LIKE search if FTS fails

## Application Capabilities

1. **Import**: Parse @ delimited LOD files into SQLite
2. **Browse**: Virtual scrolling for large word lists
3. **Search**: FTS5 full-text search with snippets
4. **Edit**: CRUD for words, definitions, events, types, authors
5. **Export**: Generate HTML dictionary

## Version History

- v1.0.0: Initial release (2026-03-09)
- v1.1.0: GitHub download, auto-create DB on Android
- v1.5.0: Settings button, mobile improvements
- v1.6.0: Performance improvements, FTS rebuild, indexes

## References

- Loglan Institute: https://loglan.org
- LOD Repository: https://github.com/loglan-org/Lojban-Dictionary
