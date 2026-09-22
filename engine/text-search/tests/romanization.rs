use crate::{
    RomanizedMatch, RomanizedReading, find_romanized_match, romanized_query, romanized_readings,
};

fn form(full: &str, initials: &str) -> RomanizedReading {
    RomanizedReading {
        full: full.to_owned(),
        initials: initials.to_owned(),
    }
}

#[test]
fn phrase_readings_override_character_defaults() {
    assert_eq!(romanized_readings("音乐"), [form("yinyue", "yy")]);
    assert_eq!(romanized_readings("重庆"), [form("chongqing", "cq")]);
    assert_eq!(romanized_readings("银行"), [form("yinhang", "yh")]);
    assert_eq!(romanized_readings("人行道"), [form("renxingdao", "rxd")]);
    assert_eq!(
        romanized_readings("网易云音乐"),
        [form("wangyiyunyinyue", "wyyyy")]
    );
}

#[test]
fn ambiguous_phrases_keep_both_attested_readings() {
    assert_eq!(
        romanized_readings("朝阳"),
        [form("zhaoyang", "zy"), form("chaoyang", "cy")]
    );
    assert_eq!(
        romanized_readings("那些"),
        [form("naxie", "nx"), form("neixie", "nx")]
    );
    assert_eq!(romanized_readings("朝阳那些").len(), 4);
}

#[test]
fn ordinary_names_and_mixed_scripts_keep_useful_spellings() {
    assert_eq!(romanized_readings("同步"), [form("tongbu", "tb")]);
    assert_eq!(romanized_readings("同步 Sync"), [form("tongbusync", "tbs")]);
    assert_eq!(romanized_readings("音乐 Sync"), [form("yinyuesync", "yys")]);
    assert!(romanized_readings("Terminal").is_empty());
    assert_eq!(romanized_query("同bu"), [form("tongbu", "tb")]);
    assert_eq!(romanized_query("tong步"), [form("tongbu", "tb")]);
    assert_eq!(romanized_query("同bu sync"), [form("tongbusync", "tbs")]);
    assert_eq!(romanized_query("音yue"), [form("yinyue", "yy")]);
    assert_eq!(romanized_query("t步"), [form("tbu", "tb")]);
    assert!(romanized_query("tongbu").is_empty());
}

#[test]
fn spelling_matches_preserve_exact_prefix_and_infix_positions() {
    let query = romanized_query("同bu");
    assert_eq!(
        find_romanized_match(&romanized_readings("同步"), &query),
        Some(RomanizedMatch::Exact)
    );
    assert_eq!(
        find_romanized_match(&romanized_readings("同步工具"), &query),
        Some(RomanizedMatch::Prefix("gongju"))
    );
    assert_eq!(
        find_romanized_match(&romanized_readings("不同步"), &query),
        Some(RomanizedMatch::Infix("bu"))
    );
    assert_eq!(
        find_romanized_match(&[form("butongbu", "btb"), form("tongbu", "tb")], &query),
        Some(RomanizedMatch::Exact)
    );
    assert_eq!(
        find_romanized_match(&romanized_readings("音乐"), &query),
        None
    );
}
