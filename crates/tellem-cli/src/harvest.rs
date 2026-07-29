//! harvest: collect model output into the corpus, slowly and resumably.
//!
//! Shape is set by two facts. The endpoint is a personal gateway in front of
//! subscription-backed tools, so the run must be gentle and must never hammer
//! after an error. And a corpus is built over days, so the run must survive
//! being killed: the corpus is an append-only JSONL and every start reads what
//! is already there and skips it. Interrupting this is safe by construction.
//!
//! The battery reuses the prompts the reference corpus already used. Same
//! prompts against new models means topic is controlled across both, so a
//! fingerprint difference is the model changing rather than the subject.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tellem_core::Error;

use crate::attribute_cmd::read_corpus;

#[derive(Deserialize)]
pub struct Config {
    /// OpenAI-compatible endpoint, for example http://127.0.0.1:20128/v1
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    /// Seconds to wait between requests. The drip, not a burst.
    #[serde(default = "default_delay")]
    pub delay_secs: f32,
    /// Stop after this many samples in one run, so a cron slice is bounded.
    #[serde(default = "default_budget")]
    pub per_run: usize,
    /// Give up on a model for this run after this many consecutive failures.
    #[serde(default = "default_strikes")]
    pub strikes: usize,
    /// How long to wait out a 429 before retrying the same prompt, and how
    /// many times. The gateway rate limits under sustained load, and a rate
    /// limit is transient, so retiring the model over one is throwing away
    /// samples that were about to be available.
    #[serde(default = "default_backoff")]
    pub backoff_secs: f32,
    #[serde(default = "default_retries")]
    pub retries: usize,
    pub models: Vec<ModelSpec>,
}

fn default_delay() -> f32 {
    6.0
}
fn default_budget() -> usize {
    120
}
fn default_strikes() -> usize {
    3
}
fn default_backoff() -> f32 {
    60.0
}
fn default_retries() -> usize {
    4
}

#[derive(Deserialize, Clone)]
pub struct ModelSpec {
    /// Catalog label. Versions collapse into a family until the eval shows
    /// they separate, so several model ids can share one family.
    pub family: String,
    pub provider: String,
    /// The id to send, for example cx/gpt-5.5
    pub model: String,
    #[serde(default = "default_target")]
    pub target: usize,
}

fn default_target() -> usize {
    300
}

#[derive(Deserialize)]
struct Prompt {
    prompt_id: String,
    prompt: String,
}

/// What is still owed for one model, in battery order. The whole resumability
/// story is this function: it is the reason a killed run costs nothing and a
/// restart never double-charges the endpoint for a sample already on disk.
fn pending<'a>(
    battery: &'a [Prompt],
    done: &HashSet<(String, String)>,
    family: &str,
    target: usize,
) -> Vec<&'a Prompt> {
    let have = done.iter().filter(|(f, _)| f == family).count();
    battery
        .iter()
        .filter(|p| !done.contains(&(family.to_string(), p.prompt_id.clone())))
        .take(target.saturating_sub(have))
        .collect()
}

/// One corpus line. Provenance travels with the sample: a gateway alias can be
/// re-pointed upstream without notice, so the label alone is not evidence.
#[derive(Serialize)]
struct Sample<'a> {
    family: &'a str,
    provider: &'a str,
    model: &'a str,
    prompt_id: &'a str,
    source: String,
    /// What the endpoint said it actually ran, when it says anything.
    upstream: String,
    harvested_at: String,
    text: String,
}

pub fn harvest_cmd(
    config: &Path,
    prompts: &Path,
    corpus: &Path,
    now: &str,
    dry_run: bool,
) -> Result<(), Error> {
    let cfg: Config = toml::from_str(&std::fs::read_to_string(config)?)?;
    let battery: Vec<Prompt> = std::fs::read_to_string(prompts)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;

    // Everything already collected, so a restart is a no-op on done work.
    let mut done: HashSet<(String, String)> = HashSet::new();
    if corpus.exists() {
        for r in read_corpus(corpus)? {
            done.insert((r.family.clone(), r.prompt_id.clone()));
        }
    }
    println!(
        "{} prompts in the battery, {} samples already collected",
        battery.len(),
        done.len()
    );

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(corpus)?;
    let mut collected = 0usize;

    for spec in &cfg.models {
        let have = done.iter().filter(|(f, _)| *f == spec.family).count();
        if have >= spec.target {
            println!("{:<14} {have}/{} done", spec.family, spec.target);
            continue;
        }
        let mut strikes = 0usize;
        for p in pending(&battery, &done, &spec.family, spec.target) {
            if collected >= cfg.per_run {
                println!(
                    "\nper-run budget of {} reached, stopping cleanly",
                    cfg.per_run
                );
                return Ok(());
            }
            if strikes >= cfg.strikes {
                println!(
                    "{:<14} {} failures in a row, skipping for this run",
                    spec.family, strikes
                );
                break;
            }

            if dry_run {
                println!("would ask {} for {}", spec.model, &p.prompt_id);
                collected += 1;
                done.insert((spec.family.clone(), p.prompt_id.clone()));
                continue;
            }

            let started = Instant::now();
            match ask_with_backoff(&cfg, spec, &p.prompt) {
                Ok((text, upstream)) if !text.trim().is_empty() => {
                    let line = serde_json::to_string(&Sample {
                        family: &spec.family,
                        provider: &spec.provider,
                        model: &spec.model,
                        prompt_id: &p.prompt_id,
                        source: format!("harvest/{}", cfg.base_url),
                        upstream,
                        harvested_at: now.to_string(),
                        text,
                    })?;
                    writeln!(file, "{line}")?;
                    file.flush()?;
                    done.insert((spec.family.clone(), p.prompt_id.clone()));
                    collected += 1;
                    strikes = 0;
                    print!(".");
                    std::io::stdout().flush().ok();
                }
                Ok(_) => {
                    strikes += 1;
                    eprintln!("\n{}: empty response", spec.model);
                }
                Err(e) => {
                    strikes += 1;
                    eprintln!("\n{}: {e}", spec.model);
                }
            }
            // Pace from the START of the request, so a slow call does not also
            // pay the full delay on top.
            let elapsed = started.elapsed();
            let target = Duration::from_secs_f32(cfg.delay_secs);
            if elapsed < target {
                std::thread::sleep(target - elapsed);
            }
        }
    }
    println!("\ncollected {collected} samples this run");
    Ok(())
}

/// Ask, waiting out rate limits. Only a non-429 failure counts as a strike:
/// a 429 means "later", not "broken", and the whole point of a slow drip is
/// that it can afford to wait.
fn ask_with_backoff(
    cfg: &Config,
    spec: &ModelSpec,
    prompt: &str,
) -> Result<(String, String), Error> {
    for attempt in 0..=cfg.retries {
        match ask(cfg, spec, prompt) {
            Err(e) if is_rate_limit(&e) && attempt < cfg.retries => {
                // Linear, not exponential. The limit is a refill window rather
                // than congestion, so doubling just wastes the window.
                let wait = cfg.backoff_secs * (attempt + 1) as f32;
                eprintln!("\n{} rate limited, waiting {wait:.0}s", spec.model);
                std::thread::sleep(Duration::from_secs_f32(wait));
            }
            other => return other,
        }
    }
    Err("retries exhausted".into())
}

fn is_rate_limit(e: &Error) -> bool {
    let s = e.to_string();
    s.contains("429") || s.to_lowercase().contains("rate limit")
}

fn ask(cfg: &Config, spec: &ModelSpec, prompt: &str) -> Result<(String, String), Error> {
    // stream:false is not optional. Several gateway backends stream by
    // default and answer with SSE chunks, which parse as neither JSON nor an
    // error, so a harvester without this silently records every one of them as
    // a failure.
    let body = serde_json::json!({
        "model": spec.model,
        "stream": false,
        "messages": [{ "role": "user", "content": prompt }],
        "max_tokens": 1024,
    });
    let mut req = ureq::post(&format!(
        "{}/chat/completions",
        cfg.base_url.trim_end_matches('/')
    ))
    .timeout(Duration::from_secs(180));
    if !cfg.api_key.is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", cfg.api_key));
    }
    let res = req.send_json(body).map_err(|e| e.to_string())?;
    let v: serde_json::Value = res.into_json()?;
    if let Some(err) = v.get("error") {
        return Err(format!("{err}").into());
    }
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let upstream = v["model"].as_str().unwrap_or(&spec.model).to_string();
    Ok((text, upstream))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battery(n: usize) -> Vec<Prompt> {
        (0..n)
            .map(|i| Prompt {
                prompt_id: format!("p{i}"),
                prompt: format!("prompt {i}"),
            })
            .collect()
    }

    fn done(family: &str, ids: &[&str]) -> HashSet<(String, String)> {
        ids.iter()
            .map(|i| (family.to_string(), i.to_string()))
            .collect()
    }

    #[test]
    fn skips_what_is_already_on_disk() {
        let b = battery(5);
        let d = done("gpt-5", &["p0", "p2"]);
        let got: Vec<&str> = pending(&b, &d, "gpt-5", 5)
            .iter()
            .map(|p| p.prompt_id.as_str())
            .collect();
        assert_eq!(got, ["p1", "p3", "p4"]);
    }

    #[test]
    fn another_family_does_not_count_as_done() {
        let b = battery(3);
        let d = done("claude-4", &["p0", "p1", "p2"]);
        assert_eq!(pending(&b, &d, "gpt-5", 3).len(), 3);
    }

    #[test]
    fn stops_at_the_target_counting_what_exists() {
        let b = battery(10);
        let d = done("gpt-5", &["p0", "p1"]);
        // target 4, two already collected, so only two more are owed
        assert_eq!(pending(&b, &d, "gpt-5", 4).len(), 2);
        assert_eq!(pending(&b, &done("gpt-5", &["p0"]), "gpt-5", 1).len(), 0);
    }

    #[test]
    fn a_finished_family_asks_for_nothing() {
        let b = battery(3);
        let d = done("gpt-5", &["p0", "p1", "p2"]);
        assert!(pending(&b, &d, "gpt-5", 3).is_empty());
    }
}
