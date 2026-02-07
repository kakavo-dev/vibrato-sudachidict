use std::fs::File;
use std::io::{BufReader, BufWriter};

use anyhow::Result;
use clap::Parser;

use sudachi_vibrato_converter::cli::{Cli, Commands};
use sudachi_vibrato_converter::{
    append_text_files_as_lines, append_unknown_definitions, convert_char_definition,
    convert_lexicon, convert_unknown_dictionary, write_rewrite_definition, ConversionStats,
};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert(args) => {
            let mut stats = ConversionStats::default();

            let lex_in = BufReader::new(File::open(&args.lex_in)?);
            let lex_out = BufWriter::new(File::create(&args.lex_out)?);
            convert_lexicon(lex_in, lex_out, &mut stats)?;

            let unk_in = BufReader::new(File::open(&args.unk_in)?);
            let mut unk_out = BufWriter::new(File::create(&args.unk_out)?);
            convert_unknown_dictionary(unk_in, &mut unk_out)?;
            append_unknown_definitions(&mut unk_out, &args.unk_append)?;

            let char_in = BufReader::new(File::open(&args.char_in)?);
            let mut char_out = BufWriter::new(File::create(&args.char_out)?);
            convert_char_definition(char_in, &mut char_out)?;
            append_text_files_as_lines(&mut char_out, &args.char_append)?;

            if let (Some(rewrite_in), Some(rewrite_out)) =
                (args.rewrite_in.as_deref(), args.rewrite_out.as_deref())
            {
                write_rewrite_definition(rewrite_in, rewrite_out, &args.rewrite_append)?;
            }

            stats.write_env_file(&args.stats_out)?;
        }
    }

    Ok(())
}
