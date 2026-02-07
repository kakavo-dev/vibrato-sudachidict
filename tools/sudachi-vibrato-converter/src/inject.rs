use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::convert_unk::convert_unknown_dictionary;

pub fn append_text_files_as_lines<W: Write>(output: &mut W, files: &[PathBuf]) -> Result<()> {
    for path in files {
        let file = File::open(path)?;
        append_text_as_lines(BufReader::new(file), output)?;
    }
    Ok(())
}

fn append_text_as_lines<R: BufRead, W: Write>(reader: R, output: &mut W) -> Result<()> {
    for line in reader.lines() {
        let mut line = line?;
        if line.ends_with('\r') {
            line.pop();
        }
        writeln!(output, "{line}")?;
    }
    Ok(())
}

pub fn append_unknown_definitions<W: Write>(output: &mut W, files: &[PathBuf]) -> Result<()> {
    for path in files {
        let input = BufReader::new(File::open(path)?);
        convert_unknown_dictionary(input, &mut *output)?;
    }
    Ok(())
}

pub fn write_rewrite_definition(
    rewrite_in: &Path,
    rewrite_out: &Path,
    rewrite_append: &[PathBuf],
) -> Result<()> {
    let mut writer = BufWriter::new(File::create(rewrite_out)?);

    let input = File::open(rewrite_in)?;
    append_text_as_lines(BufReader::new(input), &mut writer)?;

    for path in rewrite_append {
        let append_file = File::open(path)?;
        append_text_as_lines(BufReader::new(append_file), &mut writer)?;
    }

    writer.flush()?;
    Ok(())
}
