use std::io::{BufRead, BufReader, Read, Write};

use anyhow::Result;

pub fn convert_char_definition<R: Read, W: Write>(input: R, mut output: W) -> Result<()> {
    let reader = BufReader::new(input);

    for line in reader.lines() {
        let mut line = line?;
        if line.ends_with('\r') {
            line.pop();
        }

        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            writeln!(output, "{line}")?;
            continue;
        }

        match normalize_char_range_line(&line) {
            Some(Some(out_line)) => {
                writeln!(output, "{out_line}")?;
            }
            Some(None) => {
                // range line became category-less after stripping NOOOVBOW
            }
            None => {
                writeln!(output, "{line}")?;
            }
        }
    }

    Ok(())
}

// Some(Some(line)) => normalized range line
// Some(None) => range line removed
// None => not a range line
fn normalize_char_range_line(line: &str) -> Option<Option<String>> {
    let mut split = line.splitn(2, '#');
    let body = split.next().unwrap_or("");
    let comment = split.next();

    let tokens: Vec<&str> = body.split_whitespace().collect();
    if tokens.len() < 2 {
        return None;
    }

    if !is_codepoint_range(tokens[0]) {
        return None;
    }

    let mut out_tokens = vec![tokens[0]];
    for token in tokens.iter().skip(1) {
        if *token != "NOOOVBOW" {
            out_tokens.push(token);
        }
    }

    if out_tokens.len() == 1 {
        return Some(None);
    }

    let mut out = out_tokens.join(" ");
    if let Some(comment) = comment {
        out.push(' ');
        out.push('#');
        out.push_str(comment);
    }

    Some(Some(out))
}

fn is_codepoint_range(token: &str) -> bool {
    if let Some((start, end)) = token.split_once("..") {
        is_hex_codepoint(start) && is_hex_codepoint(end)
    } else {
        is_hex_codepoint(token)
    }
}

fn is_hex_codepoint(token: &str) -> bool {
    if token.len() <= 2 || !token.starts_with("0x") {
        return false;
    }
    token[2..].chars().all(|c| c.is_ascii_hexdigit())
}
