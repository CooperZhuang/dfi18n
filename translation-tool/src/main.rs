#![feature(trim_prefix_suffix)]

use std::{fs, path};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use flexi_logger::{LevelFilter, LogSpecBuilder, Logger};

use rule_based_translator::{ResultTree, Translator};

const AFTER_HELP: &'static str = color_print::cstr!(
  r#"<bold><underline>Examples:</underline></bold>
  Generate translation from single text input:
    <bold>translation-tool rulesets generate "pig iron bars"</bold>
  Generate translations from input CSV file:
    <bold>translation-tool rulesets generate input.csv</bold>
  Generate translations from input CSV file and write generated test to output CSV file:
    <bold>translation-tool rulesets generate input.csv --out-file output.csv</bold>
  Debug translations for a single text input:
    <bold>translation-tool rulesets debug "pig iron bars"</bold>
  Debug translations for a single text input (also show empty-matched nodes):
    <bold>translation-tool rulesets debug "pig iron bars" --verbose</bold>
  Dump the loaded rulesets:
    <bold>translation-tool rulesets dump</bold>
  Test translations for correctness from test CSV file:
    <bold>translation-tool rulesets test test.csv</bold>
  Show this help:
    <bold>translation-tool rulesets help</bold>
  Show help for generate command:
    <bold>translation-tool rulesets help generate</bold>

<bold><underline>Input CSV Format:</underline></bold> (first line is header, no need to indent)
  <bold><green>text</green></bold>
  <green>Snow-covered walnut tree twigs</green>
  <green>Sheep wool dress</green>
  <green>Pig iron</green>
  <green>Pig iron bars</green>

<bold><underline>Test CSV Format:</underline></bold> (first line is header, no need to indent)
  <bold><green>original</green></bold>,<bold><yellow>expected</yellow></bold>
  <green>Snow-covered walnut tree twigs</green>,<yellow>雪覆核桃树细枝</yellow>
  <green>Sheep wool dress</green>,<yellow>绵羊绒线裙装</yellow>
  <green>Pig iron</green>,<yellow>生铁</yellow>
  <green>Pig iron bars</green>,<yellow>生铁锭</yellow>"#
);

const GENERATE_AFTER_HELP: &'static str = color_print::cstr!(
  r#"<bold><underline>Examples:</underline></bold>
  Generate translation from single text input:
    <bold>translation-tool rulesets generate "pig iron bars"</bold>
  Generate translations from input CSV file:
    <bold>translation-tool rulesets generate input.csv</bold>
  Generate translations from input CSV file and write generated test to output CSV file:
    <bold>translation-tool rulesets generate input.csv --out-file output.csv</bold>

<bold><underline>Input CSV Format:</underline></bold> (first line is header, no need to indent)
  <bold><green>text</green></bold>
  <green>Snow-covered walnut tree twigs</green>
  <green>Sheep wool dress</green>
  <green>Pig iron</green>
  <green>Pig iron bars</green>"#
);

const DEBUG_AFTER_HELP: &'static str = color_print::cstr!(
  r#"<bold><underline>Examples:</underline></bold>
  Debug translations for a single text input:
    <bold>translation-tool rulesets debug "pig iron bars"</bold>
  Debug translations for a single text input (also show empty-matched nodes):
    <bold>translation-tool rulesets debug "pig iron bars" --verbose</bold>"#
);

const DUMP_AFTER_HELP: &'static str = color_print::cstr!(
  r#"<bold><underline>Examples:</underline></bold>
  Dump the loaded rulesets:
    <bold>translation-tool rulesets dump</bold>"#
);

const TEST_AFTER_HELP: &'static str = color_print::cstr!(
  r#"<bold><underline>Examples:</underline></bold>
  Test translations for correctness from test CSV file:
    <bold>translation-tool rulesets test test.csv</bold>

<bold><underline>Test CSV Format:</underline></bold> (first line is header, no need to indent)
  <bold><green>original</green></bold>,<bold><yellow>expected</yellow></bold>
  <green>Snow-covered walnut tree twigs</green>,<yellow>雪覆核桃树细枝</yellow>
  <green>Sheep wool dress</green>,<yellow>绵羊绒线裙装</yellow>
  <green>Pig iron</green>,<yellow>生铁</yellow>
  <green>Pig iron bars</green>,<yellow>生铁锭</yellow>"#
);

/// A tool for generating, debugging, and testing rule-based translations
#[derive(Debug, Parser)]
#[command(after_help = AFTER_HELP)]
struct Cli {
  /// Path to the rulesets directory
  rulesets_dir: String,

  #[command(subcommand)]
  command: Commands,

  /// Specify language tag when loading rulesets, default to the first available language in the rulesets directory
  #[arg(short, long)]
  lang_tag: Option<String>,

  /// Enable tracing for debugging (will produce a large amount of log output)
  #[arg(short, long)]
  trace: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
  /// Generate translations from input CSV or single text input
  #[command(after_help = GENERATE_AFTER_HELP)]
  Generate {
    /// Path to the input CSV file or a single text input for generating translations
    input: String,
    /// Optional output file to write generated test CSV
    #[arg(short, long)]
    out_file: Option<String>,
  },
  /// Debug translations for a single text input
  #[command(after_help = DEBUG_AFTER_HELP)]
  Debug {
    /// A single text input for debugging translations
    input: String,
    /// Enable verbose output (don't hide empty-matched nodes)
    #[arg(short, long)]
    verbose: bool,
  },
  /// Dump the loaded rulesets
  #[command(after_help = DUMP_AFTER_HELP)]
  Dump {},
  /// Test translations for correctness from test CSV
  #[command(after_help = TEST_AFTER_HELP)]
  Test {
    /// Path to the test CSV file for testing translations
    test_csv: String,
  },
}

const MOD_NAME: &str = "rule_based_translator";

fn main() -> Result<()> {
  let args = Cli::parse();

  // Configure logger, specifically for translator module (use debug by default, or trace level if -t or --trace is specified)
  let mod_level = if args.trace {
    LevelFilter::Trace
  } else {
    LevelFilter::Debug
  };
  let log_spec = LogSpecBuilder::new().default(LevelFilter::Info).module(MOD_NAME, mod_level).build();
  Logger::with(log_spec).log_to_stdout().start().unwrap();

  // Determine rulesets path based on lang_tag or first available language
  let rulesets_base_path = path::Path::new(&args.rulesets_dir);
  let rulesets_path = if let Some(lang_tag) = &args.lang_tag {
    rulesets_base_path.join(lang_tag)
  } else {
    fs::read_dir(rulesets_base_path)
      .context("failed to read rulesets directory")?
      .filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
          if path.join("index.toml").is_file() {
            return Some(path);
          }
        }

        None
      })
      .next()
      .ok_or(anyhow!(
        "No language directories with index.toml found in {:?}",
        rulesets_base_path
      ))?
  };

  // Validate rulesets path
  if !rulesets_path.is_dir() {
    return Err(anyhow!("Rulesets path {:?} is not a directory", rulesets_path));
  }
  if !rulesets_path.join("index.toml").is_file() {
    return Err(anyhow!("Rulesets path {:?} does not contain index.toml", rulesets_path));
  }
  log::info!("Using rulesets from {:?}", rulesets_path);

  // Register default replacers
  rule_based_translator::register_default_replacers();

  // Create a translator and load rulesets
  let mut translator = Translator::default();
  translator.load_from_dir(&rulesets_path).context("failed to load translator from rulesets")?;

  // Execute the specified command
  match args.command {
    Commands::Generate { input, out_file } => run_generate(&translator, input, out_file),
    Commands::Debug { input, verbose } => run_debug(&translator, input, verbose),
    Commands::Dump {} => run_dump(&translator),
    Commands::Test { test_csv } => run_test(&translator, test_csv),
  }
}

fn run_generate(translator: &Translator, input: String, out_file: Option<String>) -> Result<()> {
  // Prepare the input texts from either a path to the input CSV file or a single text input
  let mut texts = Vec::new();
  if input.ends_with(".csv") {
    log::info!("Reading input texts from CSV file: {input:?}");
    let p = path::Path::new(&input);
    let f = fs::File::open(p).with_context(|| format!("failed to open text CSV file {p:?}"))?;
    for entry in csv::Reader::from_reader(f).deserialize::<GenerateEntry>() {
      let GenerateEntry { text } = entry.context("failed to deserialize generate entry")?;
      texts.push(text);
    }
  } else {
    log::info!("Using single text input: {input:?}");
    texts.push(input);
  }

  // Prepare output CSV writer if out_file is specified
  let mut out_writer = if let Some(out_file) = out_file {
    log::info!("Writing generated test CSV to output file: {out_file:?}");
    let file = fs::File::create(out_file).with_context(|| "failed to create output file")?;
    let writer = csv::Writer::from_writer(file);
    Some(writer)
  } else {
    None
  };

  // for each text
  for text in texts {
    // generate translation
    let translation = if let Some(translation) = translator.translate(&text) {
      log::info!("{text:?} => {translation:?}");
      translation
    } else {
      log::warn!("Failed to translate {text:?}");
      String::new()
    };

    // write to output CSV if specified
    if let Some(writer) = &mut out_writer {
      let test_entry = TestEntry {
        original: text.clone(),
        expected: translation,
      };
      writer.serialize(test_entry).with_context(|| "failed to write test entry to output file")?;
    }
  }

  Ok(())
}

fn run_debug(translator: &Translator, input: String, verbose: bool) -> Result<()> {
  let results = translator.get_all_translations(&input, false);
  if results.is_empty() {
    log::warn!("No translations found for {input:?}");
  } else {
    log::info!("Translations results for {input:?} ({}):", results.len());
    for result in results {
      dump(&input, &result, verbose);
    }
  }

  Ok(())
}

fn run_dump(translator: &Translator) -> Result<()> {
  log::info!("{:#?}", translator.dump());
  Ok(())
}

fn run_test(translator: &Translator, test_csv: String) -> Result<()> {
  let p = path::Path::new(&test_csv);
  let f = fs::File::open(p).with_context(|| format!("failed to open test CSV file {p:?}"))?;
  for entry in csv::Reader::from_reader(f).deserialize::<TestEntry>() {
    let TestEntry { original, expected } = entry.context("failed to deserialize test entry")?;
    let translated = translator.translate(&original).unwrap_or_default();
    if translated == expected {
      log::info!("PASS: {original:?} => {translated:?}");
    } else {
      log::error!("FAIL: {original:?} => {translated:?}, expected: {expected:?}");
    }
  }

  Ok(())
}

// Dump ResultTree to stdout
pub fn dump(input: &str, result: &ResultTree, verbose: bool) {
  do_dump(input, result, 0, verbose);
}

// Recursive helper function to dump ResultTree
fn do_dump(input: &str, result: &ResultTree, level: usize, verbose: bool) {
  let indent = "  ".repeat(level);
  if level == 0 {
    log::info!("---")
  }

  // dump current level
  let weight = result.weight();
  let ResultTree {
    identifier,
    original,
    matched,
    translated,
    remaining,
    ..
  } = result;
  let already_matched = input.trim_suffix(remaining).trim_suffix(matched);
  let prefix = if already_matched == "" {
    String::new()
  } else {
    format!("{already_matched:?} ... ")
  };
  let suffix = if remaining == "" {
    String::new()
  } else {
    format!(" ... {remaining:?}")
  };
  log::info!("{indent} {level}> {translated:?} <W:{weight}> {prefix}({identifier}@{original} => {matched:?}){suffix}");

  // dump children
  for (_, child) in &result.children {
    // skip non-matching children in non-verbose mode
    if !verbose && child.matched.is_empty() {
      continue;
    }

    do_dump(&input, &child, level + 1, verbose);
  }
}

// Generate CSV entry
#[derive(Debug, serde::Deserialize)]
struct GenerateEntry {
  // Original text
  text: String,
}

// Test CSV entry
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TestEntry {
  // Original text
  original: String,
  // Expected translated text
  expected: String,
}
