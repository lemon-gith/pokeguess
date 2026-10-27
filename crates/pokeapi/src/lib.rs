//! A small, typed wrapper around the parts of PokeAPI that Pokeguess uses.
//!
//! Only the fields we read are modelled; serde ignores the rest. This crate
//! does no I/O, so it works both natively and in the browser.

use serde::Deserialize;

/// Where PokeAPI data is fetched from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The live API at `pokeapi.co`.
    PokeApi,
    /// PokeAPI's static JSON dump on GitHub (`PokeAPI/api-data`), which is
    /// what the live API serves, without loading PokeAPI's servers.
    ApiData,
}

impl Source {
    const POKEAPI_BASE: &str = "https://pokeapi.co/api/v2/";
    const API_DATA_BASE: &str =
        "https://raw.githubusercontent.com/PokeAPI/api-data/master/data/api/v2/";

    /// URL of an endpoint path such as `pokemon-species/25/`. A query string
    /// (e.g. `?limit=2000`) is only kept for the live API; the static dump's
    /// lists are never paginated.
    pub fn url(self, path: &str) -> String {
        let (path, query) = path
            .split_once('?')
            .map_or((path, None), |(p, q)| (p, Some(q)));
        let path = path.trim_start_matches('/');
        let path = path.strip_prefix("api/v2/").unwrap_or(path);
        let dir = path.trim_end_matches('/');
        match self {
            Source::PokeApi => match query {
                Some(query) => format!("{}{dir}/?{query}", Self::POKEAPI_BASE),
                None => format!("{}{dir}/", Self::POKEAPI_BASE),
            },
            Source::ApiData => format!("{}{dir}/index.json", Self::API_DATA_BASE),
        }
    }

    /// URL of a resource link found inside a response. The live API returns
    /// absolute links and the dump returns relative ones; both work here.
    pub fn resolve(self, link: &str) -> String {
        let path = link.split_once("/api/v2/").map_or(link, |(_, path)| path);
        self.url(path)
    }
}

/// The trailing numeric ID of a resource link, e.g. `25` for
/// `/api/v2/pokemon-species/25/`.
pub fn resource_id(link: &str) -> Option<u32> {
    link.trim_end_matches('/').rsplit('/').next()?.parse().ok()
}

#[derive(Debug, Clone, Deserialize)]
pub struct NamedResource {
    pub name: String,
    pub url: String,
}

impl NamedResource {
    pub fn id(&self) -> Option<u32> {
        resource_id(&self.url)
    }
}

/// A paginated list, e.g. `pokemon-species/`.
#[derive(Debug, Clone, Deserialize)]
pub struct ResourceList {
    pub count: u32,
    pub results: Vec<NamedResource>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PokemonSpecies {
    pub id: u32,
    pub name: String,
    pub generation: NamedResource,
    pub color: NamedResource,
    pub shape: Option<NamedResource>,
    pub habitat: Option<NamedResource>,
    pub is_baby: bool,
    pub is_legendary: bool,
    pub is_mythical: bool,
    pub names: Vec<Name>,
    pub genera: Vec<Genus>,
    pub flavor_text_entries: Vec<FlavorText>,
    pub varieties: Vec<Variety>,
}

impl PokemonSpecies {
    /// The species' default form.
    pub fn default_variety(&self) -> Option<&NamedResource> {
        self.varieties
            .iter()
            .find(|v| v.is_default)
            .map(|v| &v.pokemon)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Name {
    pub name: String,
    pub language: NamedResource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Genus {
    pub genus: String,
    pub language: NamedResource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlavorText {
    pub flavor_text: String,
    pub language: NamedResource,
    pub version: NamedResource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Variety {
    pub is_default: bool,
    pub pokemon: NamedResource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pokemon {
    pub id: u32,
    pub name: String,
    /// In decimetres.
    pub height: u32,
    /// In hectograms.
    pub weight: u32,
    pub types: Vec<PokemonType>,
    pub stats: Vec<PokemonStat>,
    pub sprites: Sprites,
    pub cries: Cries,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PokemonType {
    pub slot: u8,
    #[serde(rename = "type")]
    pub kind: NamedResource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PokemonStat {
    pub base_stat: u32,
    pub stat: NamedResource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sprites {
    pub front_default: Option<String>,
    pub other: OtherSprites,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OtherSprites {
    #[serde(rename = "official-artwork")]
    pub official_artwork: OfficialArtwork,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OfficialArtwork {
    pub front_default: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cries {
    pub latest: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_urls_for_both_sources() {
        assert_eq!(
            Source::PokeApi.url("pokemon-species/25/"),
            "https://pokeapi.co/api/v2/pokemon-species/25/"
        );
        assert_eq!(
            Source::ApiData.url("pokemon-species"),
            "https://raw.githubusercontent.com/PokeAPI/api-data/master/data/api/v2/pokemon-species/index.json"
        );
    }

    #[test]
    fn keeps_queries_for_the_live_api_only() {
        assert_eq!(
            Source::PokeApi.url("pokemon-species/?limit=2000"),
            "https://pokeapi.co/api/v2/pokemon-species/?limit=2000"
        );
        assert_eq!(
            Source::ApiData.url("pokemon-species/?limit=2000"),
            "https://raw.githubusercontent.com/PokeAPI/api-data/master/data/api/v2/pokemon-species/index.json"
        );
    }

    #[test]
    fn resolves_relative_and_absolute_links() {
        let expected = "https://pokeapi.co/api/v2/pokemon/25/";
        assert_eq!(Source::PokeApi.resolve("/api/v2/pokemon/25/"), expected);
        assert_eq!(
            Source::PokeApi.resolve("https://pokeapi.co/api/v2/pokemon/25/"),
            expected
        );
    }

    #[test]
    fn parses_resource_ids() {
        assert_eq!(resource_id("/api/v2/generation/9/"), Some(9));
        assert_eq!(
            resource_id("https://pokeapi.co/api/v2/pokemon-species/1025/"),
            Some(1025)
        );
        assert_eq!(resource_id("/api/v2/generation/"), None);
    }
}
