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
    /// Seconds to wait for one response. Reasoning models think before they
    /// answer and 145s has been measured on a real one, so the default that
    /// suits a chat model retires them as failures when they were working.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// How long a rate-limited model sits out before its next turn. The
    /// gateway limits under sustained load, and a limit is transient, so the
    /// model is skipped rather than retired.
    #[serde(default = "default_backoff")]
    pub backoff_secs: f32,
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
fn default_timeout() -> u64 {
    420
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

    // Round-robin, not one model at a time. Two reasons, both from a live run.
    // A rate-limited model used to block every model below it in the list, so
    // one busy provider starved the whole battery. And consecutive requests to
    // the same provider are what trigger the limit in the first place, so
    // alternating is gentler on the endpoint as well as faster overall.
    let mut queues: Vec<Vec<&Prompt>> = cfg
        .models
        .iter()
        .map(|m| pending(&battery, &done, &m.family, m.target))
        .collect();
    let mut strikes = vec![0usize; cfg.models.len()];
    let mut cooling: Vec<Option<Instant>> = vec![None; cfg.models.len()];
    for (spec, q) in cfg.models.iter().zip(&queues) {
        println!("{:<20} {} owed", spec.family, q.len());
    }

    'outer: loop {
        let mut asked = false;
        let mut waiting = false;
        for i in 0..cfg.models.len() {
            if collected >= cfg.per_run {
                println!("\nper-run budget of {} reached", cfg.per_run);
                break 'outer;
            }
            if queues[i].is_empty() || strikes[i] >= cfg.strikes {
                continue;
            }
            // A cooling model is not a failed one, it just is not this turn's.
            if cooling[i].is_some_and(|t| Instant::now() < t) {
                waiting = true;
                continue;
            }
            let spec = &cfg.models[i];
            let p = queues[i].remove(0);
            asked = true;

            if dry_run {
                println!("would ask {} for {}", spec.model, p.prompt_id);
                collected += 1;
                continue;
            }

            let started = Instant::now();
            match ask(&cfg, spec, &p.prompt) {
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
                    collected += 1;
                    strikes[i] = 0;
                    cooling[i] = None;
                    print!(".");
                    std::io::stdout().flush().ok();
                }
                Err(e) if is_rate_limit(&e) => {
                    // Put the prompt back and come round again later. Not a
                    // strike: a rate limit says "later", not "broken".
                    queues[i].insert(0, p);
                    cooling[i] = Some(Instant::now() + Duration::from_secs_f32(cfg.backoff_secs));
                    eprintln!(
                        "\n{} rate limited, cooling {:.0}s",
                        spec.model, cfg.backoff_secs
                    );
                }
                Ok(_) => {
                    strikes[i] += 1;
                    eprintln!("\n{}: empty response", spec.model);
                }
                Err(e) => {
                    strikes[i] += 1;
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
        if !asked && !waiting {
            break; // everything is done or retired
        }
        if !asked {
            // Every remaining model is cooling. Wait rather than spin.
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    for (i, spec) in cfg.models.iter().enumerate() {
        if strikes[i] >= cfg.strikes {
            println!("{:<20} retired after {} failures", spec.family, strikes[i]);
        }
    }
    println!("\ncollected {collected} samples this run");
    Ok(())
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
    .timeout(Duration::from_secs(cfg.timeout_secs));
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
