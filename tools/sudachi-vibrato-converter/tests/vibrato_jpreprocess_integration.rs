use std::io::Cursor;

use anyhow::{anyhow, Result};
use jpreprocess_core::word_entry::WordEntry;
use sudachi_vibrato_converter::{
    convert_char_definition, convert_lexicon, convert_unknown_dictionary, ConversionStats,
};
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
        assert!(worker.num_tokens() > 0, "no tokens for sentence: {sentence}");

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
