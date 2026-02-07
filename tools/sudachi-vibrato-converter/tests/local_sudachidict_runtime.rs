use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Result;
use vibrato::{Dictionary, Tokenizer};

#[test]
fn local_sudachidict_runtime_smoke_test_if_installed() -> Result<()> {
    let dict_path = local_dict_path();
    if !dict_path.exists() {
        eprintln!(
            "skip: local SudachiDict is not installed at {}. run ../../scripts/prepare-local-sudachidict-runtime-test.sh first.",
            dict_path.display()
        );
        return Ok(());
    }

    let dict = load_dictionary(&dict_path)?;
    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();

    assert_has_tokens(&mut worker, "東京都に行く");
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

    let scientific = token_surfaces(&mut worker, "1e-3");
    assert_ne!(scientific, vec!["1e-3"]);
    assert!(scientific.len() > 1, "1e-3 should remain non-merged");

    Ok(())
}

fn load_dictionary(path: &Path) -> Result<Dictionary> {
    if let Ok(dict) = Dictionary::read(File::open(path)?) {
        return Ok(dict);
    }

    let mut decoder = zstd::stream::read::Decoder::new(File::open(path)?)?;
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    let dict = Dictionary::read(decompressed.as_slice())?;
    Ok(dict)
}

fn local_dict_path() -> PathBuf {
    if let Some(path) = std::env::var_os("SUDACHI_VIBRATO_LOCAL_DIC") {
        return PathBuf::from(path);
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("local-sudachidict")
        .join("system.dic.zst")
}

fn assert_token_surfaces(
    worker: &mut vibrato::tokenizer::worker::Worker<'_>,
    sentence: &str,
    expected: &[&str],
) {
    let actual = token_surfaces(worker, sentence);
    println!("actual: {:?}", actual);
    assert_eq!(actual, expected);
}

fn assert_has_tokens(worker: &mut vibrato::tokenizer::worker::Worker<'_>, sentence: &str) {
    worker.reset_sentence(sentence);
    worker.tokenize();
    assert!(
        worker.num_tokens() > 0,
        "no tokens for sentence: {sentence}"
    );
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
