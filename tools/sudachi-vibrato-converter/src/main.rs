use std::fs::File;
use std::io::{BufReader, BufWriter};

use anyhow::Result;
use clap::Parser;

use sudachi_vibrato_converter::cli::{Cli, Commands};
use sudachi_vibrato_converter::{
    convert_char_definition, convert_lexicon, convert_unknown_dictionary, ConversionStats,
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
            let unk_out = BufWriter::new(File::create(&args.unk_out)?);
            convert_unknown_dictionary(unk_in, unk_out)?;

            let char_in = BufReader::new(File::open(&args.char_in)?);
            let char_out = BufWriter::new(File::create(&args.char_out)?);
            convert_char_definition(char_in, char_out)?;

            stats.write_env_file(&args.stats_out)?;
        }
    }

    Ok(())
}
