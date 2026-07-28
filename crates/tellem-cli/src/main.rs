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
