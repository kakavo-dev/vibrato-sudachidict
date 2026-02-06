use once_cell::sync::Lazy;
use std::collections::HashSet;

static ALLOWED_CTYPE: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "*",
        "ラ変",
        "不変化型",
        "カ変・クル",
        "カ変・来ル",
        "サ変・スル",
        "サ変・−スル",
        "サ変・−ズル",
        "一段",
        "一段・病メル",
        "一段・クレル",
        "一段・得ル",
        "一段・ル",
        "下二・ア行",
        "下二・カ行",
        "下二・ガ行",
        "下二・サ行",
        "下二・ザ行",
        "下二・タ行",
        "下二・ダ行",
        "下二・ナ行",
        "下二・ハ行",
        "下二・バ行",
        "下二・マ行",
        "下二・ヤ行",
        "下二・ラ行",
        "下二・ワ行",
        "下二・得",
        "形容詞・アウオ段",
        "形容詞・イ段",
        "形容詞・イイ",
        "五段・カ行イ音便",
        "五段・カ行促音便",
        "五段・カ行促音便ユク",
        "五段・ガ行",
        "五段・サ行",
        "五段・タ行",
        "五段・ナ行",
        "五段・バ行",
        "五段・マ行",
        "五段・ラ行",
        "五段・ラ行アル",
        "五段・ラ行特殊",
        "五段・ワ行ウ音便",
        "五段・ワ行促音便",
        "四段・カ行",
        "四段・ガ行",
        "四段・サ行",
        "四段・タ行",
        "四段・バ行",
        "四段・マ行",
        "四段・ラ行",
        "四段・ハ行",
        "上二・ダ行",
        "上二・ハ行",
        "特殊・ナイ",
        "特殊・タイ",
        "特殊・タ",
        "特殊・ダ",
        "特殊・デス",
        "特殊・ドス",
        "特殊・ジャ",
        "特殊・マス",
        "特殊・ヌ",
        "特殊・ヤ",
        "文語・ベシ",
        "文語・ゴトシ",
        "文語・ナリ",
        "文語・マジ",
        "文語・シム",
        "文語・キ",
        "文語・ケリ",
        "文語・ル",
        "文語・リ",
    ]
    .into_iter()
    .collect()
});

static ALLOWED_CFORM: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "*",
        "ガル接続",
        "音便基本形",
        "仮定形",
        "仮定縮約１",
        "仮定縮約２",
        "基本形",
        "基本形-促音便",
        "現代基本形",
        "体言接続",
        "体言接続特殊",
        "体言接続特殊２",
        "文語基本形",
        "未然ウ接続",
        "未然ヌ接続",
        "未然レル接続",
        "未然形",
        "未然特殊",
        "命令ｅ",
        "命令ｉ",
        "命令ｒｏ",
        "命令ｙｏ",
        "連用ゴザイ接続",
        "連用タ接続",
        "連用テ接続",
        "連用デ接続",
        "連用ニ接続",
        "連用形",
    ]
    .into_iter()
    .collect()
});

pub fn normalize_pos(pos0: &str) -> [String; 4] {
    match pos0.trim() {
        "名詞" | "代名詞" | "形状詞" | "接尾辞" => [
            "名詞".to_string(),
            "一般".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "動詞" => [
            "動詞".to_string(),
            "自立".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "形容詞" => [
            "形容詞".to_string(),
            "自立".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "助詞" => [
            "助詞".to_string(),
            "格助詞".to_string(),
            "一般".to_string(),
            "*".to_string(),
        ],
        "助動詞" => [
            "助動詞".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "副詞" => [
            "副詞".to_string(),
            "一般".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "接続詞" => [
            "接続詞".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "連体詞" => [
            "連体詞".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "感動詞" => [
            "感動詞".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "接頭辞" | "接頭詞" => [
            "接頭詞".to_string(),
            "名詞接続".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "記号" | "補助記号" | "空白" => [
            "記号".to_string(),
            "一般".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        "フィラー" => [
            "フィラー".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
        _ => [
            "その他".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ],
    }
}

pub fn normalize_ctype(value: &str) -> (String, bool) {
    let src = normalize_text_or_star(value);

    let mut canonical = strip_spaces(&src);
    canonical = canonical
        .chars()
        .map(|c| match c {
            '-' | '－' | '−' => '・',
            _ => c,
        })
        .collect();

    if canonical == "五段・ワア行" {
        canonical = "五段・ワ行ウ音便".to_string();
    }

    canonical = canonical
        .replace("サ変・スル", "サ変・−スル")
        .replace("サ変・ズル", "サ変・−ズル")
        .replace("サ変・ｰスル", "サ変・−スル")
        .replace("サ変・ｰズル", "サ変・−ズル")
        .replace("サ変・ースル", "サ変・−スル")
        .replace("サ変・ーズル", "サ変・−ズル")
        .replace("サ変・・スル", "サ変・−スル")
        .replace("サ変・・ズル", "サ変・−ズル");

    if ALLOWED_CTYPE.contains(canonical.as_str()) {
        (canonical, false)
    } else {
        ("*".to_string(), src != "*")
    }
}

pub fn normalize_cform(value: &str) -> (String, bool) {
    let src = normalize_text_or_star(value);
    let mut canonical = strip_spaces(&src);

    canonical = if canonical == "終止連体形"
        || canonical.starts_with("終止形")
        || canonical.starts_with("連体形")
    {
        "基本形".to_string()
    } else if canonical.starts_with("連用形") {
        "連用形".to_string()
    } else if canonical.starts_with("未然形") {
        "未然形".to_string()
    } else if canonical.starts_with("仮定形") {
        "仮定形".to_string()
    } else if canonical.starts_with("命令形") {
        "命令ｙｏ".to_string()
    } else if canonical == "意志推量形" {
        "未然ウ接続".to_string()
    } else {
        canonical
    };

    if ALLOWED_CFORM.contains(canonical.as_str()) {
        (canonical, false)
    } else {
        ("*".to_string(), src != "*")
    }
}

pub fn normalize_text_or_star(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "*".to_string()
    } else {
        trimmed.to_string()
    }
}

fn strip_spaces(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{3000}')
        .collect()
}
