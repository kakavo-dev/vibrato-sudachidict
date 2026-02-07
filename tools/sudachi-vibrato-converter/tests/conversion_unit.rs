use std::fs;
use std::io::Cursor;

use anyhow::Result;
use csv::ReaderBuilder;
use sudachi_vibrato_converter::{
    append_text_files_as_lines, append_unknown_definitions, convert_char_definition,
    convert_lexicon, convert_unknown_dictionary, write_rewrite_definition, ConversionStats,
};
use tempfile::tempdir;

#[test]
fn lex_uses_pos_5_to_8_and_reading_11() -> Result<()> {
    let input = "語,1,2,3,原形,動詞,普通,*,*,五段-ワア行,終止形-一般,ヨミ,余剰\n";

    let mut output = Vec::new();
    let mut stats = ConversionStats::default();
    convert_lexicon(Cursor::new(input.as_bytes()), &mut output, &mut stats)?;

    let rows = parse_csv_rows(&output)?;
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    assert_eq!(row.len(), 13);
    assert_eq!(row[0], "語");
    assert_eq!(row[1], "1");
    assert_eq!(row[2], "2");
    assert_eq!(row[3], "3");

    assert_eq!(row[4], "動詞");
    assert_eq!(row[5], "自立");
    assert_eq!(row[6], "*");
    assert_eq!(row[7], "*");
    assert_eq!(row[8], "五段・ワ行ウ音便");
    assert_eq!(row[9], "基本形");
    assert_eq!(row[10], "原形");
    assert_eq!(row[11], "ヨミ");
    assert_eq!(row[12], "ヨミ");

    assert_eq!(stats.written, 1);
    assert_eq!(stats.normalized_pos_rows, 1);
    Ok(())
}

#[test]
fn lex_skips_negative_connection_ids_and_fills_missing_reading() -> Result<()> {
    let input = concat!(
        "捨てる,-1,0,1,捨てる,名詞,普通名詞,一般,*,*,*,ステル\n",
        "採用,0,0,1,採用,名詞,普通名詞,一般,*,*,*\n"
    );

    let mut output = Vec::new();
    let mut stats = ConversionStats::default();
    convert_lexicon(Cursor::new(input.as_bytes()), &mut output, &mut stats)?;

    let rows = parse_csv_rows(&output)?;
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    assert_eq!(row[0], "採用");
    assert_eq!(row[11], "*");
    assert_eq!(row[12], "*");

    assert_eq!(stats.written, 1);
    assert_eq!(stats.skipped_negative_conn_ids, 1);
    Ok(())
}

#[test]
fn unk_is_converted_to_mecab_minimum_schema() -> Result<()> {
    let input = "ALPHA,0,0,100,名詞,普通名詞,一般,*,*,*\n";

    let mut output = Vec::new();
    convert_unknown_dictionary(Cursor::new(input.as_bytes()), &mut output)?;

    let rows = parse_csv_rows(&output)?;
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    assert_eq!(row.len(), 13);
    assert_eq!(row[0], "ALPHA");
    assert_eq!(row[4], "名詞");
    assert_eq!(row[5], "一般");
    assert_eq!(row[6], "*");
    assert_eq!(row[7], "*");
    assert_eq!(row[8], "*");
    assert_eq!(row[9], "*");
    assert_eq!(row[10], "*");
    assert_eq!(row[11], "*");
    assert_eq!(row[12], "*");
    Ok(())
}

#[test]
fn char_definition_strips_nooovbow_entries() -> Result<()> {
    let input = concat!(
        "# comment\n",
        "0x0041..0x005A ALPHA NOOOVBOW #A-Z\n",
        "0x0030..0x0039 NOOOVBOW #DIGIT\n",
        "DEFAULT 0 1 0\n"
    );

    let mut output = Vec::new();
    convert_char_definition(Cursor::new(input.as_bytes()), &mut output)?;
    let output = String::from_utf8(output)?;

    assert!(output.contains("# comment\n"));
    assert!(output.contains("0x0041..0x005A ALPHA #A-Z\n"));
    assert!(!output.contains("0x0030..0x0039"));
    assert!(output.contains("DEFAULT 0 1 0\n"));
    Ok(())
}

#[test]
fn char_definition_can_append_custom_rules() -> Result<()> {
    let input = "DEFAULT 0 1 0\n0x0030..0x0039 NUMERIC NOOOVBOW\n";
    let append = "0x0030..0x0039 NUMERIC ALPHA\n0xFF10..0xFF19 NUMERIC ALPHA\n";
    let dir = tempdir()?;
    let append_path = dir.path().join("char.append.def");
    fs::write(&append_path, append)?;

    let mut output = Vec::new();
    convert_char_definition(Cursor::new(input.as_bytes()), &mut output)?;
    append_text_files_as_lines(&mut output, &[append_path])?;

    let output = String::from_utf8(output)?;
    assert!(!output.contains("NOOOVBOW"));
    assert!(output.contains("0x0030..0x0039 NUMERIC ALPHA\n"));
    assert!(output.contains("0xFF10..0xFF19 NUMERIC ALPHA\n"));
    Ok(())
}

#[test]
fn unk_definition_can_append_custom_rows_with_normalization() -> Result<()> {
    let base = "ALPHA,0,0,100,名詞,普通名詞,一般,*,*,*\n";
    let append = "NUMERIC,0,0,100,名詞,数,*,*,*,*\n";
    let dir = tempdir()?;
    let append_path = dir.path().join("unk.append.def");
    fs::write(&append_path, append)?;

    let mut output = Vec::new();
    convert_unknown_dictionary(Cursor::new(base.as_bytes()), &mut output)?;
    append_unknown_definitions(&mut output, &[append_path])?;

    let rows = parse_csv_rows(&output)?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1][0], "NUMERIC");
    assert_eq!(rows[1][4], "名詞");
    assert_eq!(rows[1][5], "数");
    assert_eq!(rows[1][10], "*");
    Ok(())
}

#[test]
fn lex_numeric_pos_is_normalized_to_meishi_kazu() -> Result<()> {
    let input = "123,1,2,3,123,名詞,数詞,*,*,*,*,ヒャクニジュウサン\n";

    let mut output = Vec::new();
    let mut stats = ConversionStats::default();
    convert_lexicon(Cursor::new(input.as_bytes()), &mut output, &mut stats)?;

    let rows = parse_csv_rows(&output)?;
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    assert_eq!(row[0], "123");
    assert_eq!(row[4], "名詞");
    assert_eq!(row[5], "数");
    assert_eq!(row[6], "*");
    assert_eq!(row[7], "*");
    Ok(())
}

#[test]
fn rewrite_definition_is_copied_and_appended() -> Result<()> {
    let dir = tempdir()?;
    let rewrite_in = dir.path().join("rewrite.in.def");
    let rewrite_out = dir.path().join("rewrite.out.def");
    let append = dir.path().join("rewrite.append.def");

    fs::write(
        &rewrite_in,
        "[unigram rewrite]\nfoo,*,*,* bar,*,*,*\n[left rewrite]\n",
    )?;
    fs::write(&append, "# custom\n[right rewrite]\na,b,c d,e,f\n")?;

    write_rewrite_definition(&rewrite_in, &rewrite_out, &[append])?;

    let output = fs::read_to_string(&rewrite_out)?;
    assert!(output.contains("[unigram rewrite]\n"));
    assert!(output.contains("[left rewrite]\n"));
    assert!(output.contains("# custom\n"));
    assert!(output.contains("[right rewrite]\n"));
    assert!(output.contains("a,b,c d,e,f\n"));
    Ok(())
}

#[test]
fn rewrite_definition_copy_works_without_append_files() -> Result<()> {
    let dir = tempdir()?;
    let rewrite_in = dir.path().join("rewrite.in.def");
    let rewrite_out = dir.path().join("rewrite.out.def");

    let input = "[unigram rewrite]\nx,*,* y,*,*\n";
    fs::write(&rewrite_in, input)?;

    write_rewrite_definition(&rewrite_in, &rewrite_out, &[])?;
    let output = fs::read_to_string(&rewrite_out)?;
    assert_eq!(output, input);
    Ok(())
}

fn parse_csv_rows(bytes: &[u8]) -> Result<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes);

    for record in reader.records() {
        let record = record?;
        rows.push(record.iter().map(ToOwned::to_owned).collect());
    }

    Ok(rows)
}
