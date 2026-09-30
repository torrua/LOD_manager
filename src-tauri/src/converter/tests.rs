#[cfg(test)]
mod converter_tests {
    use crate::convert_text_to_sqlite;
    use rusqlite::Connection;
    use std::fs;

    #[test]
    fn test_convert_text_files() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();

        let tmp_dir =
            std::env::temp_dir().join(format!("lod_test_converter_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();

        fs::write(
            tmp_dir.join("type.txt"),
            "C-Prim@Composite Primitive@Prim@False@Composite primitive predicate\n\
             2-Cpx@Two-Term Complex@Cpx@True@Two-term complex\n\
             Afx@Affix@Affix@True@CvV/CCV affix\n\
             LW@Little Word@LW@True@Structure word\n",
        )
        .unwrap();

        fs::write(
            tmp_dir.join("author.txt"),
            "JCB@James Cooke Brown@Founder\n\
             L4@Loglan 4@1975 dictionary\n",
        )
        .unwrap();

        fs::write(
            tmp_dir.join("lexevent.txt"),
            "1@Initial@01.01.1975@The initial vocabulary@INIT@init\n\
             2@Update@01.01.1989@Second event@UPD@upd\n",
        )
        .unwrap();

        fs::write(
            tmp_dir.join("syllable.txt"),
            "ba@CV@True\n\
             zz@CC@False\n",
        )
        .unwrap();

        fs::write(
            tmp_dir.join("words.txt"),
            "75@C-Prim@@alk@50%@JCB/L4@1975@1.0@3/6E alcohol@alcohol@alko@10\n\
             100@2-Cpx@@ek@80%@JCB@1975@2.0@ekti + act@do@ekta@20\n",
        )
        .unwrap();

        fs::write(
            tmp_dir.join("wordspell.txt"),
            "75@alkooli@@1@1@2\n\
             75@alkoholi@@2@2@9999\n\
             100@ekti@@3@1@9999\n",
        )
        .unwrap();

        fs::write(
            tmp_dir.join("worddefinition.txt"),
            "75@1@%@2a@«alcohol» drink@\n\
             75@2@lo %@n@mass of «alcohol»@\n\
             100@1@%@2v@«act» on something@B-K\n",
        )
        .unwrap();

        fs::write(
            tmp_dir.join("setting.txt"),
            "22.08.2024 05:08:00@1@10150@4.5.9\n",
        )
        .unwrap();

        let test_dir = tmp_dir.to_string_lossy().into_owned();
        let result = convert_text_to_sqlite(&mut conn, &test_dir).unwrap();

        assert_eq!(result.types, 4, "Should import 4 types");
        assert_eq!(result.authors, 2, "Should import 2 authors");
        assert_eq!(result.events, 2, "Should import 2 events");
        assert_eq!(result.words, 3, "Should import 3 word spellings");
        assert_eq!(
            result.definitions, 5,
            "Should attach definitions to all spellings sharing old_id=75 (2*2 + 1 = 5)"
        );
        assert_eq!(result.settings, 1, "Should import 1 setting row");

        // Verify "False" boolean parsing in types and syllables
        let cprim_parentable: bool = conn
            .query_row(
                "SELECT parentable FROM types WHERE type='C-Prim'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!cprim_parentable, "C-Prim parentable should be false");

        let zz_allowed: bool = conn
            .query_row("SELECT allowed FROM syllables WHERE name='zz'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(!zz_allowed, "zz syllable allowed should be false");

        // Verify event_end = 9999 becomes NULL
        let alkoholi_end: Option<i64> = conn
            .query_row(
                "SELECT event_end FROM words WHERE name='alkoholi'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            alkoholi_end, None,
            "event_end 9999 should be stored as NULL"
        );

        // Verify slots + grammar_code split
        let (slots, gcode): (Option<i64>, Option<String>) = conn
            .query_row(
                "SELECT slots, grammar_code FROM definitions d JOIN words w ON w.id = d.word_id WHERE w.name='ekti'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(slots, Some(2));
        assert_eq!(gcode.as_deref(), Some("v"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_convert_nonexistent_directory() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();

        let result = convert_text_to_sqlite(&mut conn, "nonexistent_dir");
        assert!(result.is_err(), "Should fail for nonexistent directory");
    }

    #[test]
    fn test_convert_empty_directory() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();

        let empty_dir = std::env::temp_dir().join(format!("lod_empty_dir_{}", std::process::id()));
        let _ = fs::remove_dir_all(&empty_dir);
        fs::create_dir_all(&empty_dir).unwrap();

        let result = convert_text_to_sqlite(&mut conn, &empty_dir.to_string_lossy());
        assert!(result.is_err(), "Should fail for empty directory");

        let _ = fs::remove_dir_all(&empty_dir);
    }
}
