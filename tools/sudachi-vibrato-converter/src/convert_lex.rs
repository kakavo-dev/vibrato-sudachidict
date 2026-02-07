use std::io::{Read, Write};

use anyhow::{anyhow, Context, Result};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};

use crate::normalize::{normalize_cform, normalize_ctype, normalize_pos, normalize_text_or_star};
use crate::stats::ConversionStats;

pub fn convert_lexicon<R: Read, W: Write>(
    input: R,
    output: W,
    stats: &mut ConversionStats,
) -> Result<()> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(input);
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(output);

    for (line_no, record) in reader.records().enumerate() {
        let record =
            record.with_context(|| format!("failed to read lex row at line {}", line_no + 1))?;

        if record.is_empty() {
            continue;
        }
        if record.len() < 11 {
            return Err(anyhow!(
                "invalid lex row at line {}: expected >=11 columns, got {}",
                line_no + 1,
                record.len()
            ));
        }

        let left = parse_i32(&record, 1, "left_id", line_no + 1)?;
        let right = parse_i32(&record, 2, "right_id", line_no + 1)?;
        let _cost = parse_i32(&record, 3, "cost", line_no + 1)?;

        if left < 0 || right < 0 {
            stats.skipped_negative_conn_ids += 1;
            continue;
        }

        let original_pos = [
            normalize_text_or_star(record.get(5).unwrap_or("")),
            normalize_text_or_star(record.get(6).unwrap_or("")),
            normalize_text_or_star(record.get(7).unwrap_or("")),
            normalize_text_or_star(record.get(8).unwrap_or("")),
        ];

        let normalized_pos = normalize_pos(record.get(5).unwrap_or(""));
        if original_pos != normalized_pos {
            stats.normalized_pos_rows += 1;
        }

        let (ctype, ctype_fallback) = normalize_ctype(record.get(9).unwrap_or(""));
        if ctype_fallback {
            stats.fallback_ctype_rows += 1;
        }

        let (cform, cform_fallback) = normalize_cform(record.get(10).unwrap_or(""));
        if cform_fallback {
            stats.fallback_cform_rows += 1;
        }

        let base = normalize_text_or_star(record.get(4).unwrap_or(""));
        let read = normalize_text_or_star(record.get(11).unwrap_or(""));
        let pron = read.clone();

        let output_row = vec![
            record.get(0).unwrap_or("").to_string(),
            record.get(1).unwrap_or("").to_string(),
            record.get(2).unwrap_or("").to_string(),
            record.get(3).unwrap_or("").to_string(),
            normalized_pos[0].clone(),
            normalized_pos[1].clone(),
            normalized_pos[2].clone(),
            normalized_pos[3].clone(),
            ctype,
            cform,
            base,
            read,
            pron,
        ];

        writer
            .write_record(&output_row)
            .with_context(|| format!("failed to write lex row at line {}", line_no + 1))?;
        stats.written += 1;
    }

    writer.flush()?;
    Ok(())
}

fn parse_i32(record: &StringRecord, index: usize, name: &str, line_no: usize) -> Result<i32> {
    let value = record
        .get(index)
        .ok_or_else(|| anyhow!("missing {} at line {}", name, line_no))?
        .trim();
    value
        .parse::<i32>()
        .with_context(|| format!("failed to parse {}='{}' at line {}", name, value, line_no))
}
