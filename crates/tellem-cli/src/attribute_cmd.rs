//! mine / who / eval: the attribution side of the CLI.
//!
//! Corpus format is JSONL, one sample per line, because the harvest drip
//! appends to it over days and an append-only log is resumable for free.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tellem_core::mine::Catalog;
use tellem_core::train::fit;
use tellem_core::Error;

#[derive(Deserialize)]
pub struct Record {
    pub family: String,
    /// Same prompt answered by every model. Splits go by prompt, never by
    /// sample, or the same prompt lands on both sides and the eval flatters us.
    #[serde(default)]
    pub prompt_id: String,
    pub text: String,
}

pub fn read_corpus(path: &Path) -> Result<Vec<Record>, Error> {
    let mut out = Vec::new();
    for (i, line) in BufReader::new(std::fs::File::open(path)?)
        .lines()
        .enumerate()
    {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(
            serde_json::from_str(&line)
                .map_err(|e| format!("{}:{}: {e}", path.display(), i + 1))?,
        );
    }
    Ok(out)
}

/// Deterministic hash so a split is reproducible across runs and machines.
fn bucket(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

pub fn mine_cmd(corpus: &Path, out: &Path, top_k: usize, epochs: usize) -> Result<(), Error> {
    let records = read_corpus(corpus)?;
    let samples: Vec<(String, String)> = records
        .iter()
        .map(|r| (r.family.clone(), r.text.clone()))
        .collect();
    let mut catalog = fit(&samples, top_k, epochs);
    catalog.source = corpus.display().to_string();
    std::fs::write(out, toml::to_string(&catalog)?)?;
    println!(
        "mined {} families from {} samples -> {}",
        catalog.fingerprints.len(),
        records.len(),
        out.display()
    );
    Ok(())
}

pub fn who_cmd(
    file: Option<PathBuf>,
    catalog: &Path,
    margin: f32,
    json: bool,
) -> Result<(), Error> {
    let cat: Catalog = toml::from_str(&std::fs::read_to_string(catalog)?)?;
    let text = match file {
        Some(p) => std::fs::read_to_string(p)?,
        None => std::io::read_to_string(std::io::stdin())?,
    };
    let a = cat.who(&text, margin, 40);
    if json {
        println!("{}", serde_json::to_string_pretty(&a)?);
        return Ok(());
    }
    match &a.call {
        Some(f) => println!(
            "{f}  ({:.0}% vs {:.0}% {})",
            a.ranked[0].probability * 100.0,
            a.ranked[1].probability * 100.0,
            a.ranked[1].family
        ),
        None => println!(
            "insufficient signal, closest: {}",
            a.ranked
                .iter()
                .take(2)
                .map(|c| c.family.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
    for r in &a.receipts {
        println!(
            "  {:<28} +{:.3}  {:.2}/kw vs {:.2}/kw baseline",
            r.feature, r.contribution, r.rate, r.baseline
        );
    }
    println!(
        "{} features matched, among the {} families in this catalog",
        a.matched,
        a.catalog.len()
    );
    Ok(())
}

/// Derive the margin threshold from a held-out split instead of picking one.
/// The floor is the invariant, coverage is whatever it turns out to be.
pub fn eval_cmd(
    corpus: &Path,
    top_k: usize,
    floor: f32,
    holdout: u64,
    epochs: usize,
) -> Result<(), Error> {
    let records = read_corpus(corpus)?;
    let (train, test): (Vec<&Record>, Vec<&Record>) = records
        .iter()
        .partition(|r| bucket(&r.prompt_id) % 100 >= holdout);
    println!(
        "corpus {} samples: {} train, {} held out (split by prompt)",
        records.len(),
        train.len(),
        test.len()
    );

    let train_samples: Vec<(String, String)> = train
        .iter()
        .map(|r| (r.family.clone(), r.text.clone()))
        .collect();
    let catalog = fit(&train_samples, top_k, epochs);
    for f in &catalog.fingerprints {
        println!(
            "  {:<12} {:>6} samples, {:>9} tokens",
            f.family, f.samples, f.tokens
        );
    }

    // Score once, then sweep the posterior gap.
    let scored: Vec<(String, Option<String>, f32)> = test
        .iter()
        .map(|r| {
            let a = catalog.who(&r.text, 0.0, 0);
            (
                r.family.clone(),
                a.ranked.first().map(|c| c.family.clone()),
                a.confidence,
            )
        })
        .collect();

    let correct = scored
        .iter()
        .filter(|(t, p, _)| p.as_deref() == Some(t))
        .count();
    println!(
        "\nforced-choice accuracy (no abstention): {:.1}% over {} held-out samples",
        100.0 * correct as f32 / scored.len() as f32,
        scored.len()
    );

    let mut chosen: Option<(f32, f32, f32)> = None;
    println!("\n{:<9} {:>10} {:>10}", "conf", "precision", "coverage");
    let mut steps: Vec<f32> = (0..=99).map(|i| i as f32 * 0.01).collect();
    steps.dedup();
    for m in steps {
        let called: Vec<_> = scored.iter().filter(|(_, _, mg)| *mg >= m).collect();
        if called.is_empty() {
            break;
        }
        let hits = called
            .iter()
            .filter(|(t, p, _)| p.as_deref() == Some(t))
            .count();
        let precision = hits as f32 / called.len() as f32;
        let coverage = called.len() as f32 / scored.len() as f32;
        if ((m * 100.0) as u32).is_multiple_of(10) {
            println!(
                "{m:<9.2} {:>9.1}% {:>9.1}%",
                precision * 100.0,
                coverage * 100.0
            );
        }
        if chosen.is_none() && precision >= floor {
            chosen = Some((m, precision, coverage));
        }
    }

    match chosen {
        Some((m, p, c)) => println!(
            "\nthreshold {m:.2} is the lowest confidence holding {:.0}% precision: \
             {:.1}% precision at {:.1}% coverage",
            floor * 100.0,
            p * 100.0,
            c * 100.0
        ),
        None => println!(
            "\nNO threshold reaches {:.0}% precision on this corpus.",
            floor * 100.0
        ),
    }

    // Confusion at the derived threshold, or forced choice when none holds.
    let m = chosen.map(|c| c.0).unwrap_or(0.0);
    let mut confusion: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    let mut abstained: BTreeMap<&str, usize> = BTreeMap::new();
    for (truth, pred, mg) in &scored {
        if *mg >= m {
            *confusion
                .entry(truth)
                .or_default()
                .entry(pred.as_deref().unwrap_or("?"))
                .or_insert(0) += 1;
        } else {
            *abstained.entry(truth).or_insert(0) += 1;
        }
    }
    let families = catalog.families();
    println!("\nconfusion at confidence {m:.2} (rows are truth)");
    print!("{:<12}", "");
    for f in &families {
        print!("{f:>10}");
    }
    println!("{:>10}", "abstain");
    for t in &families {
        print!("{t:<12}");
        for p in &families {
            print!(
                "{:>10}",
                confusion
                    .get(t)
                    .and_then(|r| r.get(p))
                    .copied()
                    .unwrap_or(0)
            );
        }
        println!("{:>10}", abstained.get(t).copied().unwrap_or(0));
    }
    Ok(())
}
