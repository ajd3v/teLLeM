use clap::{Parser, Subcommand};
use std::io::Read;
use std::path::PathBuf;
use tellem_core::{Engine, Pack};

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
    },
    /// Rewrite the tells away, deterministic and reviewable
    Fix {
        /// File to fix (stdin when omitted), fixed text goes to stdout
        file: Option<PathBuf>,
        #[arg(long)]
        pack: Vec<PathBuf>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("tellem: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), tellem_core::Error> {
    match Cli::parse().cmd {
        Cmd::Lint { file, pack, json } => {
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
        }
        Cmd::Fix { file, pack } => {
            let text = read_input(&file)?;
            print!("{}", engine(&pack)?.fix(&text));
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
