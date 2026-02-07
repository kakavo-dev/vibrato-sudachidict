use std::fs;
use std::io::Cursor;

use anyhow::{anyhow, Result};
use jpreprocess_core::word_entry::WordEntry;
use sudachi_vibrato_converter::{
    append_text_files_as_lines, append_unknown_definitions, convert_char_definition,
    convert_lexicon, convert_unknown_dictionary, ConversionStats,
};
use tempfile::tempdir;
use vibrato::dictionary::{LexType, SystemDictionaryBuilder};
use vibrato::Tokenizer;

#[test]
fn vibrato_and_jpreprocess_can_consume_converted_outputs() -> Result<()> {
    let lex_input = concat!(
        "東京都,0,0,100,東京都,名詞,固有名詞,地名,一般,*,*,トウキョウト,東京都\n",
        "に,0,0,100,に,助詞,格助詞,一般,*,*,*,ニ,に\n",
        "行く,0,0,100,行く,動詞,一般,*,*,*,*,イク,行く\n"
    );
    let unk_input = concat!(
        "DEFAULT,0,0,100,補助記号,一般,*,*,*,*\n",
        "ALPHA,0,0,100,名詞,普通名詞,一般,*,*,*\n",
        "KANJI,0,0,100,名詞,普通名詞,一般,*,*,*\n"
    );
    let char_input = concat!(
        "DEFAULT 0 1 0\n",
        "ALPHA 1 1 0\n",
        "KANJI 1 1 0\n",
        "SPACE 0 1 0\n",
        "0x0020 SPACE\n",
        "0xFF21..0xFF3A ALPHA\n",
        "0xFF41..0xFF5A ALPHA\n",
        "0x4E00..0x9FFF KANJI\n"
    );
    let matrix_def = "1 1\n0 0 0\n";

    let mut stats = ConversionStats::default();

    let mut lex_out = Vec::new();
    convert_lexicon(Cursor::new(lex_input.as_bytes()), &mut lex_out, &mut stats)?;

    let mut unk_out = Vec::new();
    convert_unknown_dictionary(Cursor::new(unk_input.as_bytes()), &mut unk_out)?;

    let mut char_out = Vec::new();
    convert_char_definition(Cursor::new(char_input.as_bytes()), &mut char_out)?;

    let dict = SystemDictionaryBuilder::from_readers(
        lex_out.as_slice(),
        matrix_def.as_bytes(),
        char_out.as_slice(),
        unk_out.as_slice(),
    )?;

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();

    for sentence in ["東京都に行く", "ＡＩ"] {
        worker.reset_sentence(sentence);
        worker.tokenize();
        assert!(
            worker.num_tokens() > 0,
            "no tokens for sentence: {sentence}"
        );

        for i in 0..worker.num_tokens() {
            let token = worker.token(i);
            let feature = token.feature();
            let details = mecab9_feature_to_jpreprocess12(feature)?;
            let details_ref: Vec<&str> = details.iter().map(String::as_str).collect();
            WordEntry::load(&details_ref)?;

            if token.lex_type() == LexType::Unknown {
                let fields: Vec<&str> = feature.split(',').collect();
                assert_eq!(fields[7], "*");
                assert_eq!(fields[8], "*");
            }
        }
    }

    Ok(())
}

fn mecab9_feature_to_jpreprocess12(feature: &str) -> Result<Vec<String>> {
    let fields: Vec<&str> = feature.split(',').collect();
    if fields.len() != 9 {
        return Err(anyhow!(
            "expected mecab9 feature with 9 columns, got {}: {}",
            fields.len(),
            feature
        ));
    }

    Ok(vec![
        fields[0].to_string(),
        fields[1].to_string(),
        fields[2].to_string(),
        fields[3].to_string(),
        fields[4].to_string(),
        fields[5].to_string(),
        fields[6].to_string(),
        fields[7].to_string(),
        fields[8].to_string(),
        "*/*".to_string(),
        "*".to_string(),
        "*".to_string(),
    ])
}

#[test]
fn ipadic_numeric_merge_rules_prioritize_numeric_and_split_alpha_numeric() -> Result<()> {
    let lex_input = "既知語,0,0,100,既知語,名詞,普通名詞,一般,*,*,*,キチゴ,既知語\n";
    let unk_input = concat!(
        "DEFAULT,0,0,100,補助記号,一般,*,*,*,*\n",
        "SYMBOL,0,0,100,記号,一般,*,*,*,*\n",
        "ALPHA,0,0,100,名詞,普通名詞,一般,*,*,*\n",
        "NUMERIC,0,0,100,名詞,数,*,*,*,*,*\n"
    );
    let char_input = concat!(
        "DEFAULT 0 1 0\n",
        "ALPHA 1 1 0\n",
        "NUMERIC 1 1 0\n",
        "SYMBOL 1 1 0\n",
        "SPACE 0 1 0\n",
        "0x0020 SPACE\n",
        "0x002E SYMBOL\n",
        "0xFF0E SYMBOL\n",
        "0x0030..0x0039 NUMERIC\n",
        "0x0061..0x007A ALPHA\n",
        "0x0041..0x005A ALPHA\n",
        "0xFF21..0xFF3A ALPHA\n",
        "0xFF41..0xFF5A ALPHA\n"
    );
    let char_append = concat!(
        "NUMERIC 1 0 0\n",
        "0x0030..0x0039 NUMERIC\n",
        "0xFF10..0xFF19 NUMERIC\n"
    );
    let unk_append = "# empty on purpose\n";
    let matrix_def = "1 1\n0 0 0\n";

    let mut stats = ConversionStats::default();
    let mut lex_out = Vec::new();
    convert_lexicon(Cursor::new(lex_input.as_bytes()), &mut lex_out, &mut stats)?;

    let mut unk_out = Vec::new();
    convert_unknown_dictionary(Cursor::new(unk_input.as_bytes()), &mut unk_out)?;

    let mut char_out = Vec::new();
    convert_char_definition(Cursor::new(char_input.as_bytes()), &mut char_out)?;

    let dir = tempdir()?;
    let char_append_path = dir.path().join("char.append.def");
    let unk_append_path = dir.path().join("unk.append.def");
    fs::write(&char_append_path, char_append)?;
    fs::write(&unk_append_path, unk_append)?;
    append_text_files_as_lines(&mut char_out, &[char_append_path])?;
    append_unknown_definitions(&mut unk_out, &[unk_append_path])?;

    let dict = SystemDictionaryBuilder::from_readers(
        lex_out.as_slice(),
        matrix_def.as_bytes(),
        char_out.as_slice(),
        unk_out.as_slice(),
    )?;
    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();

    assert_token_surfaces(&mut worker, "123", &["1", "2", "3"]);
    assert_token_pos12(&mut worker, "123", 0, "名詞", "数");
    assert_token_pos12(&mut worker, "123", 1, "名詞", "数");
    assert_token_pos12(&mut worker, "123", 2, "名詞", "数");
    assert_token_surfaces(&mut worker, "１２３", &["１", "２", "３"]);
    assert_token_pos12(&mut worker, "１２３", 0, "名詞", "数");
    assert_token_pos12(&mut worker, "１２３", 1, "名詞", "数");
    assert_token_pos12(&mut worker, "１２３", 2, "名詞", "数");
    assert_token_surfaces(&mut worker, "1.234", &["1", ".", "2", "3", "4"]);
    assert_token_pos12(&mut worker, "1.234", 0, "名詞", "数");
    assert_token_pos12(&mut worker, "1.234", 2, "名詞", "数");
    assert_token_pos12(&mut worker, "1.234", 3, "名詞", "数");
    assert_token_pos12(&mut worker, "1.234", 4, "名詞", "数");
    assert_token_surfaces(&mut worker, "１．２３４", &["１", "．", "２", "３", "４"]);
    assert_token_pos12(&mut worker, "１．２３４", 0, "名詞", "数");
    assert_token_pos12(&mut worker, "１．２３４", 2, "名詞", "数");
    assert_token_pos12(&mut worker, "１．２３４", 3, "名詞", "数");
    assert_token_pos12(&mut worker, "１．２３４", 4, "名詞", "数");
    assert_token_surfaces(&mut worker, "AI2026", &["AI", "2", "0", "2", "6"]);
    assert_token_pos12(&mut worker, "AI2026", 1, "名詞", "数");
    assert_token_pos12(&mut worker, "AI2026", 2, "名詞", "数");
    assert_token_pos12(&mut worker, "AI2026", 3, "名詞", "数");
    assert_token_pos12(&mut worker, "AI2026", 4, "名詞", "数");
    assert_token_surfaces(&mut worker, "ＡＩ2026", &["ＡＩ", "2", "0", "2", "6"]);
    assert_token_pos12(&mut worker, "ＡＩ2026", 1, "名詞", "数");
    assert_token_pos12(&mut worker, "ＡＩ2026", 2, "名詞", "数");
    assert_token_pos12(&mut worker, "ＡＩ2026", 3, "名詞", "数");
    assert_token_pos12(&mut worker, "ＡＩ2026", 4, "名詞", "数");
    assert_token_surfaces(&mut worker, "k8s", &["k", "8", "s"]);
    assert_token_pos12(&mut worker, "k8s", 1, "名詞", "数");
    assert_token_surfaces(&mut worker, "abc123def", &["abc", "1", "2", "3", "def"]);
    assert_token_pos12(&mut worker, "abc123def", 1, "名詞", "数");
    assert_token_pos12(&mut worker, "abc123def", 2, "名詞", "数");
    assert_token_pos12(&mut worker, "abc123def", 3, "名詞", "数");

    let scientific = token_surfaces(&mut worker, "1e-3");
    assert_ne!(scientific, vec!["1e-3"]);
    assert!(scientific.len() > 1);

    Ok(())
}

fn assert_token_surfaces(
    worker: &mut vibrato::tokenizer::worker::Worker<'_>,
    sentence: &str,
    expected: &[&str],
) {
    let actual = token_surfaces(worker, sentence);
    assert_eq!(actual, expected);
}

fn token_surfaces(
    worker: &mut vibrato::tokenizer::worker::Worker<'_>,
    sentence: &str,
) -> Vec<String> {
    worker.reset_sentence(sentence);
    worker.tokenize();
    (0..worker.num_tokens())
        .map(|i| worker.token(i).surface().to_string())
        .collect()
}

fn assert_token_pos12(
    worker: &mut vibrato::tokenizer::worker::Worker<'_>,
    sentence: &str,
    token_index: usize,
    expected_pos1: &str,
    expected_pos2: &str,
) {
    worker.reset_sentence(sentence);
    worker.tokenize();
    assert!(
        token_index < worker.num_tokens(),
        "token index {token_index} out of range for sentence: {sentence}"
    );
    let feature = worker.token(token_index).feature();
    let fields: Vec<&str> = feature.split(',').collect();
    assert!(fields.len() >= 2, "unexpected feature format: {feature}");
    assert_eq!(fields[0], expected_pos1, "pos1 mismatch for {sentence}");
    assert_eq!(fields[1], expected_pos2, "pos2 mismatch for {sentence}");
}
