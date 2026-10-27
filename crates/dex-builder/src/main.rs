//! Builds (and keeps up to date) the `pokedex.json` dataset served alongside the web app.

mod convert;
mod fetch;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use pokeapi::{Pokemon, PokemonSpecies, ResourceList, Source};
use shared::Language;
use shared::pokedex::{Pokedex, PokedexEntry, SCHEMA_VERSION};

use fetch::Fetcher;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Where to fetch PokeAPI data from.
    #[arg(long, value_enum, global = true, default_value_t = SourceArg::ApiData)]
    source: SourceArg,

    /// Minimum time between requests, in milliseconds.
    #[arg(long, global = true, default_value_t = 100)]
    delay_ms: u64,

    /// Path of the dataset file.
    #[arg(long, global = true, default_value = "apps/web/data/pokedex.json")]
    out: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Fetch every species and (over)write the dataset.
    Build {
        /// Only fetch the first N species (for quick local runs).
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Re-fetch every species and only rewrite the dataset if anything changed.
    Update {
        /// Also write a Markdown summary of the changes to this file.
        #[arg(long)]
        summary: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum SourceArg {
    /// PokeAPI's static data dump on GitHub (no load on PokeAPI's servers).
    ApiData,
    /// The live API at pokeapi.co.
    Pokeapi,
}

impl From<SourceArg> for Source {
    fn from(arg: SourceArg) -> Self {
        match arg {
            SourceArg::ApiData => Source::ApiData,
            SourceArg::Pokeapi => Source::PokeApi,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut fetcher = Fetcher::new(cli.source.into(), Duration::from_millis(cli.delay_ms))?;

    match cli.command {
        Command::Build { limit } => {
            let pokedex = fetch_pokedex(&mut fetcher, limit)?;
            write_pokedex(&cli.out, &pokedex)?;
            eprintln!(
                "wrote {} Pokémon to {} ({} requests)",
                pokedex.pokemon.len(),
                cli.out.display(),
                fetcher.requests
            );
        }
        Command::Update { summary } => {
            let old = read_pokedex(&cli.out)?;
            let new = fetch_pokedex(&mut fetcher, None)?;
            let changes = Changes::between(old.as_ref(), &new);
            if changes.is_empty() {
                eprintln!("{} is up to date", cli.out.display());
                return Ok(());
            }
            write_pokedex(&cli.out, &new)?;
            let report = changes.to_markdown();
            print!("{report}");
            if let Some(path) = summary {
                fs::write(&path, &report).with_context(|| format!("writing {}", path.display()))?;
            }
        }
    }
    Ok(())
}

fn fetch_pokedex(fetcher: &mut Fetcher, limit: Option<usize>) -> Result<Pokedex> {
    let list: ResourceList = fetcher.get("pokemon-species/?limit=100000")?;
    let mut links = list.results;
    links.sort_by_key(|r| r.id());
    if let Some(limit) = limit {
        links.truncate(limit);
    }

    let total = links.len();
    let mut pokemon = Vec::with_capacity(total);
    for (i, link) in links.iter().enumerate() {
        let species: PokemonSpecies = fetcher
            .follow(&link.url)
            .with_context(|| format!("fetching species {}", link.name))?;
        let variety = species
            .default_variety()
            .ok_or_else(|| anyhow!("{} has no default variety", species.name))?;
        let form: Pokemon = fetcher
            .follow(&variety.url)
            .with_context(|| format!("fetching pokemon {}", variety.name))?;
        pokemon.push(convert::entry(&species, &form, Language::English)?);

        if (i + 1) % 50 == 0 || i + 1 == total {
            eprintln!("fetched {}/{total}", i + 1);
        }
    }
    pokemon.sort_by_key(|p| p.id);

    Ok(Pokedex {
        schema_version: SCHEMA_VERSION,
        pokemon,
    })
}

fn read_pokedex(path: &Path) -> Result<Option<Pokedex>> {
    match fs::read_to_string(path) {
        Ok(json) => serde_json::from_str(&json)
            .map(Some)
            .with_context(|| format!("parsing {}", path.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("reading {}", path.display())),
    }
}

fn write_pokedex(path: &Path, pokedex: &Pokedex) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut json = serde_json::to_string_pretty(pokedex)?;
    json.push('\n');
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))
}

/// What an update changed, for the log and the update PR's description.
#[derive(Debug, Default, PartialEq)]
struct Changes {
    schema_changed: bool,
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
}

impl Changes {
    fn between(old: Option<&Pokedex>, new: &Pokedex) -> Self {
        let label = |p: &PokedexEntry| format!("#{} {}", p.id, p.display_name);
        let old_by_id: BTreeMap<u16, &PokedexEntry> = old
            .map(|old| old.pokemon.iter().map(|p| (p.id, p)).collect())
            .unwrap_or_default();
        let new_by_id: BTreeMap<u16, &PokedexEntry> =
            new.pokemon.iter().map(|p| (p.id, p)).collect();

        let mut changes = Changes {
            schema_changed: old.is_some_and(|old| old.schema_version != new.schema_version),
            ..Default::default()
        };
        for (id, entry) in &new_by_id {
            match old_by_id.get(id) {
                None => changes.added.push(label(entry)),
                Some(previous) if previous != entry => changes.changed.push(label(entry)),
                Some(_) => {}
            }
        }
        for (id, entry) in &old_by_id {
            if !new_by_id.contains_key(id) {
                changes.removed.push(label(entry));
            }
        }
        changes
    }

    fn is_empty(&self) -> bool {
        *self == Changes::default()
    }

    fn to_markdown(&self) -> String {
        let mut out = String::from("## Pokédex changes\n\n");
        if self.schema_changed {
            out.push_str("- Schema version changed\n");
        }
        for (heading, names) in [
            ("Added", &self.added),
            ("Changed", &self.changed),
            ("Removed", &self.removed),
        ] {
            if !names.is_empty() {
                out.push_str(&format!(
                    "- {heading} ({}): {}\n",
                    names.len(),
                    names.join(", ")
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::pokedex::BaseStats;

    fn entry(id: u16, name: &str) -> PokedexEntry {
        PokedexEntry {
            id,
            name: name.to_lowercase(),
            display_name: name.into(),
            genus: None,
            generation: 1,
            types: vec!["normal".into()],
            stats: BaseStats {
                hp: 1,
                attack: 1,
                defense: 1,
                special_attack: 1,
                special_defense: 1,
                speed: 1,
            },
            height: 1,
            weight: 1,
            color: "red".into(),
            shape: None,
            habitat: None,
            is_baby: false,
            is_legendary: false,
            is_mythical: false,
            flavor_text: None,
            sprite: None,
            artwork: None,
            cry: None,
        }
    }

    fn pokedex(pokemon: Vec<PokedexEntry>) -> Pokedex {
        Pokedex {
            schema_version: SCHEMA_VERSION,
            pokemon,
        }
    }

    #[test]
    fn no_changes_when_identical() {
        let dex = pokedex(vec![entry(1, "Bulbasaur")]);
        assert!(Changes::between(Some(&dex), &dex).is_empty());
    }

    #[test]
    fn reports_added_changed_and_removed() {
        let old = pokedex(vec![entry(1, "Bulbasaur"), entry(2, "Ivysaur")]);
        let mut renamed = entry(1, "Bulbasaur");
        renamed.genus = Some("Seed Pokémon".into());
        let new = pokedex(vec![renamed, entry(3, "Venusaur")]);

        let changes = Changes::between(Some(&old), &new);
        assert_eq!(changes.changed, ["#1 Bulbasaur"]);
        assert_eq!(changes.added, ["#3 Venusaur"]);
        assert_eq!(changes.removed, ["#2 Ivysaur"]);
        assert_eq!(
            changes.to_markdown(),
            "## Pokédex changes\n\n\
             - Added (1): #3 Venusaur\n\
             - Changed (1): #1 Bulbasaur\n\
             - Removed (1): #2 Ivysaur\n"
        );
    }

    #[test]
    fn everything_is_added_without_an_existing_file() {
        let new = pokedex(vec![entry(1, "Bulbasaur")]);
        assert_eq!(Changes::between(None, &new).added, ["#1 Bulbasaur"]);
    }
}
