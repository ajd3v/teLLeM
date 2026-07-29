mod attribute_cmd;
mod harvest;

use clap::{Parser, Subcommand, ValueEnum};
use std::io::Read;
use std::path::PathBuf;
use tellem_core::{Band, Engine, Pack};

#[derive(Parser)]
#[command(name = "tellem", version, about = "AI-text forensics with receipts")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Find AI tells, cite each one
    Lint {
        /// File to lint (stdin when omitted)
        file: Option<PathBuf>,
        /// Extra rule packs, applied after base (override by rule id)
        #[arg(long)]
        pack: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
        /// Exit 1 when the band reaches this level (for CI gates)
        #[arg(long, value_name = "BAND")]
        fail_on: Option<BandArg>,
    },
    /// Rewrite the tells away, deterministic and reviewable
    Fix {
        /// File to fix (stdin when omitted), fixed text goes to stdout
        file: Option<PathBuf>,
        #[arg(long)]
        pack: Vec<PathBuf>,
        /// Rewrite the file in place instead of printing (`fix f > f` truncates it)
        #[arg(long, short)]
        write: bool,
    },
    /// Learn per-family fingerprints from a labelled JSONL corpus
    Mine {
        corpus: PathBuf,
        #[arg(long, short, default_value = "catalog.toml")]
        out: PathBuf,
        /// Features kept per family, by |z|
        #[arg(long, default_value_t = 2000)]
        top_k: usize,
        #[arg(long, default_value_t = 12)]
        epochs: usize,
    },
    /// Closed-set model attribution, refuses below the margin
    Who {
        file: Option<PathBuf>,
        #[arg(long, short, default_value = "catalog.toml")]
        catalog: PathBuf,
        /// Confidence threshold. Derive it with `eval`, do not guess it.
        #[arg(long, default_value_t = 0.30)]
        margin: f32,
        #[arg(long)]
        json: bool,
    },
    /// Collect model output into the corpus. Resumable, and paced on purpose.
    Harvest {
        /// TOML listing the endpoint and the models to sample
        config: PathBuf,
        /// JSONL battery of {prompt_id, prompt}
        #[arg(long, default_value = "corpora/prompts.jsonl")]
        prompts: PathBuf,
        /// Append-only corpus. Existing samples are skipped, so restarts are free.
        #[arg(long, default_value = "corpora/harvest.jsonl")]
        corpus: PathBuf,
        /// Timestamp recorded on each sample (RFC3339). Defaults to unset.
        #[arg(long, default_value = "")]
        at: String,
        /// Print what would be asked without calling anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Held-out eval: derive the margin that holds the precision floor
    Eval {
        corpus: PathBuf,
        #[arg(long, default_value_t = 2000)]
        top_k: usize,
        #[arg(long, default_value_t = 12)]
        epochs: usize,
        /// Precision floor the threshold must hold (the invariant)
        #[arg(long, default_value_t = 0.95)]
        floor: f32,
        /// Percent of prompts held out
        #[arg(long, default_value_t = 20)]
        holdout: u64,
        /// Out-of-catalog text (human writing). Every call on it is a false positive.
        #[arg(long)]
        negatives: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum BandArg {
    Clean,
    Seasoned,
    Heavy,
}

impl From<Band> for BandArg {
    fn from(b: Band) -> BandArg {
        match b {
            Band::Clean => BandArg::Clean,
            Band::Seasoned => BandArg::Seasoned,
            Band::Heavy => BandArg::Heavy,
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("tellem: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), tellem_core::Error> {
    match Cli::parse().cmd {
        Cmd::Lint {
            file,
            pack,
            json,
            fail_on,
        } => {
            let text = read_input(&file)?;
            let report = engine(&pack)?.lint(&text);
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for f in &report.findings {
                    println!(
                        "{} {:<24} {:>6}..{:<6} {:?}",
                        f.rule_id, f.rule_name, f.start, f.end, f.excerpt
                    );
                    println!("     {}", f.rationale);
                }
                println!(
                    "{} findings, {:.1} tells/kword over {} words, band: {}",
                    report.findings.len(),
                    report.score,
                    report.words,
                    report.band
                );
            }
            if fail_on.is_some_and(|f| BandArg::from(report.band) >= f) {
                std::process::exit(1);
            }
        }
        Cmd::Mine {
            corpus,
            out,
            top_k,
            epochs,
        } => attribute_cmd::mine_cmd(&corpus, &out, top_k, epochs)?,
        Cmd::Who {
            file,
            catalog,
            margin,
            json,
        } => attribute_cmd::who_cmd(file, &catalog, margin, json)?,
        Cmd::Harvest {
            config,
            prompts,
            corpus,
            at,
            dry_run,
        } => harvest::harvest_cmd(&config, &prompts, &corpus, &at, dry_run)?,
        Cmd::Eval {
            corpus,
            top_k,
            floor,
            holdout,
            epochs,
            negatives,
        } => attribute_cmd::eval_cmd(&corpus, top_k, floor, holdout, epochs, negatives.as_deref())?,
        Cmd::Fix { file, pack, write } => {
            let text = read_input(&file)?;
            let fixed = engine(&pack)?.fix(&text);
            match (write, &file) {
                (true, Some(p)) => std::fs::write(p, fixed)?,
                (true, None) => return Err("--write needs a file, not stdin".into()),
                (false, _) => print!("{fixed}"),
            }
        }
    }
    Ok(())
}

fn read_input(file: &Option<PathBuf>) -> Result<String, tellem_core::Error> {
    match file {
        Some(p) => Ok(std::fs::read_to_string(p)?),
        None => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            Ok(s)
        }
    }
}

fn engine(extra: &[PathBuf]) -> Result<Engine, tellem_core::Error> {
    let mut packs = vec![Pack::parse(tellem_core::BASE_PACK)?];
    for p in extra {
        packs.push(Pack::parse(&std::fs::read_to_string(p)?)?);
    }
    Engine::from_packs(&packs)
}
