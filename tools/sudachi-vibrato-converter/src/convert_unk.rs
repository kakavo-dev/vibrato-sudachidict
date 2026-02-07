use std::io::{Read, Write};

use anyhow::{anyhow, Context, Result};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};

use crate::normalize::{normalize_cform, normalize_ctype, normalize_pos};

pub fn convert_unknown_dictionary<R: Read, W: Write>(input: R, output: W) -> Result<()> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(input);
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(output);

    for (line_no, record) in reader.records().enumerate() {
        let record =
            record.with_context(|| format!("failed to read unk row at line {}", line_no + 1))?;

        if record.is_empty() {
            continue;
        }

        let first = record.get(0).unwrap_or("").trim_start();
        if first.starts_with('#') {
            continue;
        }

        if record.len() < 10 {
            return Err(anyhow!(
                "invalid unk row at line {}: expected >=10 columns, got {}",
                line_no + 1,
                record.len()
            ));
        }

        parse_i32(&record, 1, "left_id", line_no + 1)?;
        parse_i32(&record, 2, "right_id", line_no + 1)?;
        parse_i32(&record, 3, "cost", line_no + 1)?;

        let normalized_pos = normalize_pos(record.get(4).unwrap_or(""));
        let (ctype, _) = normalize_ctype(record.get(8).unwrap_or(""));
        let (cform, _) = normalize_cform(record.get(9).unwrap_or(""));

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
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
        ];

        writer
            .write_record(&output_row)
            .with_context(|| format!("failed to write unk row at line {}", line_no + 1))?;
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
