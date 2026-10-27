//! A polite, rate-limited HTTP client for PokeAPI data.

use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use pokeapi::Source;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;

const USER_AGENT: &str = concat!(
    "pokeguess-dex-builder/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/lemon-gith/pokeguess)"
);
const MAX_ATTEMPTS: u32 = 5;

/// Fetches one request at a time, waiting at least `min_interval` between
/// requests and backing off on rate limits and server errors.
pub struct Fetcher {
    client: Client,
    pub source: Source,
    min_interval: Duration,
    last_request: Option<Instant>,
    pub requests: u32,
}

impl Fetcher {
    pub fn new(source: Source, min_interval: Duration) -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            source,
            min_interval,
            last_request: None,
            requests: 0,
        })
    }

    /// Fetches and deserialises an endpoint path such as `pokemon-species/25/`.
    pub fn get<T: DeserializeOwned>(&mut self, path: &str) -> Result<T> {
        let url = self.source.url(path);
        self.get_url(&url)
    }

    /// Fetches and deserialises a resource link found inside a response.
    pub fn follow<T: DeserializeOwned>(&mut self, link: &str) -> Result<T> {
        let url = self.source.resolve(link);
        self.get_url(&url)
    }

    fn get_url<T: DeserializeOwned>(&mut self, url: &str) -> Result<T> {
        let mut backoff = Duration::from_secs(1);
        for attempt in 1..=MAX_ATTEMPTS {
            self.throttle();
            let retry_after = match self.client.get(url).send() {
                Ok(response) if response.status().is_success() => {
                    return response
                        .json()
                        .with_context(|| format!("invalid JSON from {url}"));
                }
                Ok(response) => {
                    let status = response.status();
                    if status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error() {
                        bail!("{url} returned {status}");
                    }
                    eprintln!("{url} returned {status} (attempt {attempt}/{MAX_ATTEMPTS})");
                    retry_after_secs(&response).map(Duration::from_secs)
                }
                Err(err) => {
                    eprintln!("{url} failed: {err} (attempt {attempt}/{MAX_ATTEMPTS})");
                    None
                }
            };
            if attempt < MAX_ATTEMPTS {
                sleep(retry_after.unwrap_or(backoff));
                backoff *= 2;
            }
        }
        bail!("giving up on {url} after {MAX_ATTEMPTS} attempts")
    }

    fn throttle(&mut self) {
        if let Some(last) = self.last_request {
            let elapsed = last.elapsed();
            if elapsed < self.min_interval {
                sleep(self.min_interval - elapsed);
            }
        }
        self.last_request = Some(Instant::now());
        self.requests += 1;
    }
}

fn retry_after_secs(response: &reqwest::blocking::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}
