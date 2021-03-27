extern crate bytes;
extern crate futures;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate warp;

type Result<T> = std::result::Result<T, std::boxed::Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() {
    println!("");
    println!("Pokémons in Shakespearese");
    println!("");
    println!("  Query format: /<pokemon name>");
    println!("  For example, try `curl http://<server address>:5000/charizard`");

    warp::serve(pokemon_name_filter())
        .run(([0, 0, 0, 0], 5000))
        .await;
}

fn pokemon_name_filter(
) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Copy {
    use warp::Filter;
    warp::path::param()
        .and(warp::path::end())
        .and(warp::get())
        .and_then(respond_with_pokemon_in_shakespearese)
}

#[derive(Debug)]
struct RequestError {
    status: http::StatusCode,
    description: String,
}

impl RequestError {
    fn new<S: std::convert::Into<String>>(
        status: http::StatusCode,
        description: S,
    ) -> RequestError {
        RequestError {
            status,
            description: description.into(),
        }
    }
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "RequestError with status {} and message \"{}\"",
            self.status, &self.description
        )
    }
}

impl std::error::Error for RequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(serde::Deserialize)]
struct PokeApiPokemonSpeciesInfo {
    url: String,
}

#[derive(serde::Deserialize)]
struct PokemonResponse {
    species: PokeApiPokemonSpeciesInfo,
}

#[derive(serde::Deserialize)]
struct PokeApiPokemonDescriptionLanguage {
    name: String,
}

#[derive(serde::Deserialize)]
struct PokeApiPokemonDescriptionVersion {
    name: String,
}

#[derive(serde::Deserialize)]
struct PokeApiPokemonDescription {
    version: PokeApiPokemonDescriptionVersion,
    flavor_text: String,
    language: PokeApiPokemonDescriptionLanguage,
}

#[derive(serde::Deserialize)]
struct PokemonDescriptionResponse {
    #[serde(rename(deserialize = "flavor_text_entries"))]
    descriptions: Vec<PokeApiPokemonDescription>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PokemonInShakespeareseResponse {
    name: String,
    description: String,
}

impl PokemonInShakespeareseResponse {
    fn new<Name: Into<String>, Desc: Into<String>>(name: Name, description: Desc) -> Self {
        PokemonInShakespeareseResponse {
            name: name.into(),
            description: description.into(),
        }
    }
}

// Results of Poke API queries can depend on presense or absense of trailing slash, so we better try
// both options. For example, see
// https://pokeapi.co/api/v2/pokemon/klink vs https://pokeapi.co/api/v2/pokemon/klink/
// and https://pokeapi.co/api/v2/pokemon/electrode vs https://pokeapi.co/api/v2/pokemon/electrode/
async fn query_pokemon_by_name(pokemon_name: &str) -> Result<reqwest::Response> {
    let pokemon_request_url = format!(
        "https://pokeapi.co/api/v2/pokemon/{}",
        &pokemon_name.to_ascii_lowercase()
    );
    let pokemon_response = reqwest::get(&pokemon_request_url).await?;
    if !pokemon_response.status().is_success() {
        let url_with_trailing_slash = pokemon_request_url + "/";
        let response_with_trailing_slash = reqwest::get(&url_with_trailing_slash).await?;
        Ok(response_with_trailing_slash)
    } else {
        Ok(pokemon_response)
    }
}

async fn describe_pokemon(pokemon_name: &str) -> Result<String> {
    let pokemon_response = query_pokemon_by_name(pokemon_name).await?;
    if !pokemon_response.status().is_success() {
        return Err(RequestError::new(
            pokemon_response.status(),
            format!(
                "Failed to get an id for pokemon {} by url {}",
                &pokemon_name,
                pokemon_response.url()
            ),
        )
        .into());
    }

    let pokemon_response: PokemonResponse = serde_json::from_str(&pokemon_response.text().await?)?;
    let description_response = reqwest::get(&pokemon_response.species.url).await?;

    if !description_response.status().is_success() {
        return Err(RequestError::new(
            description_response.status(),
            format!(
                "Failed to get a description for pokemon {} by url {}",
                &pokemon_name, &pokemon_response.species.url
            ),
        )
        .into());
    }
    let description_response_json: PokemonDescriptionResponse =
        serde_json::from_str(&description_response.text().await?)?;

    let descriptions = description_response_json
        .descriptions
        .into_iter()
        .filter(|entry| entry.language.name == "en")
        .collect::<Vec<_>>();

    descriptions
        .iter()
        .find(|entry| entry.version.name == "ruby")
        .or(descriptions
            .iter()
            .max_by_key(|entry| entry.flavor_text.len()))
        .map(|entry| entry.flavor_text.replace('\n', " "))
        .ok_or(
            RequestError::new(
                http::StatusCode::UNPROCESSABLE_ENTITY,
                format!(
                    "Couldn't find any information about {} in English",
                    &pokemon_name
                ),
            )
            .into(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_describe_pokemon() {
        let charizard_description = describe_pokemon("charizard").await;
        assert!(charizard_description.is_ok());
        let charizard_description = charizard_description.unwrap();
        assert!(charizard_description.len() > 20);
        assert!(charizard_description.contains("flies"));

        let banana_description = describe_pokemon("banana").await;
        assert!(banana_description.is_err());

        let empty_request_description = describe_pokemon("").await;
        assert!(empty_request_description.is_err());

        let charizard_by_number = describe_pokemon("6").await;
        assert!(charizard_by_number.is_ok());
        assert_eq!(charizard_by_number.unwrap(), charizard_description);
    }

    #[tokio::test]
    #[ignore] // This is flaky because of the low api limits of the service (5 requests per hour)
    async fn test_shakespearise() {
        let cat_phrase = shakespearise("Curiosity killed the cat").await;
        assert!(cat_phrase.is_ok());
        assert_eq!(cat_phrase.unwrap(), "Curiosity did kill the gib");

        let empty_phrase = shakespearise("").await;
        assert!(empty_phrase.is_ok());
        assert_eq!(empty_phrase.unwrap(), "");

        let rust_phrase = shakespearise(
            "Rust is a language empowering everyone to build reliable and efficient software.",
        )
        .await;
        assert!(rust_phrase.is_ok());
        assert_eq!(
            rust_phrase.unwrap(),
            "Rust is a language empowering everyone to buildeth reliable and efficient software."
        );
    }

    #[tokio::test]
    async fn test_warp_filter() {
        let filter = pokemon_name_filter();

        assert!(!warp::test::request().path("/").matches(&filter).await);
        assert_eq!(
            warp::test::request()
                .path("/banana")
                .reply(&filter)
                .await
                .status(),
            http::StatusCode::NOT_FOUND
        );

        assert_eq!(
            warp::test::request()
                .path("/ditto")
                .method("POST")
                .reply(&filter)
                .await
                .status(),
            http::StatusCode::METHOD_NOT_ALLOWED
        );

        let charizard_response = warp::test::request()
            .path("/charizard")
            .reply(&filter)
            .await;
        assert_eq!(charizard_response.status(), http::StatusCode::OK);
        let charizard_description = parse_response(&charizard_response);
        assert!(charizard_description.is_some());
        assert_eq!(charizard_description.as_ref().unwrap().name, "charizard");
        assert!(charizard_description
            .as_ref()
            .unwrap()
            .description
            .contains("charizard"));
        assert!(charizard_description
            .as_ref()
            .unwrap()
            .description
            .contains("flies"));

        let mixed_case_response = warp::test::request()
            .path("/CharIZard")
            .reply(&filter)
            .await;
        assert_eq!(mixed_case_response.status(), http::StatusCode::OK);
        // Can't expect this to be identical to charizard_description because by this time we may
        // hit shakespeare translation api rate limit
        let mixed_description = parse_response(&mixed_case_response);
        assert!(mixed_description.is_some());
        assert_eq!(mixed_description.as_ref().unwrap().name, "charizard");
        assert!(mixed_description
            .as_ref()
            .unwrap()
            .description
            .contains("charizard"));
        assert!(mixed_description
            .as_ref()
            .unwrap()
            .description
            .contains("flies"));

        let traliling_slash_response = warp::test::request()
            .path("/charizard/")
            .reply(&filter)
            .await;
        assert_eq!(traliling_slash_response.status(), http::StatusCode::OK);
        // Can't expect this to be identical to charizard_description because by this time we may
        // hit shakespeare translation api rate limit
        let trailing_slash_description = parse_response(&traliling_slash_response);
        assert!(trailing_slash_description.is_some());
        assert_eq!(
            trailing_slash_description.as_ref().unwrap().name,
            "charizard"
        );
        assert!(trailing_slash_description
            .as_ref()
            .unwrap()
            .description
            .contains("charizard"));
        assert!(trailing_slash_description
            .as_ref()
            .unwrap()
            .description
            .contains("flies"));

        assert_eq!(
            warp::test::request()
                .path("/electrode")
                .reply(&filter)
                .await
                .status(),
            http::StatusCode::OK
        );
        assert_eq!(
            warp::test::request()
                .path("/klink")
                .reply(&filter)
                .await
                .status(),
            http::StatusCode::OK
        );

        assert_eq!(
            warp::test::request()
                .path("/charizard/whatever")
                .reply(&filter)
                .await
                .status(),
            http::StatusCode::NOT_FOUND
        );
        assert_eq!(
            warp::test::request()
                .path("/charizard+ditto")
                .reply(&filter)
                .await
                .status(),
            http::StatusCode::NOT_FOUND
        );
        assert_eq!(
            warp::test::request()
                .path("/charizard,ditto")
                .reply(&filter)
                .await
                .status(),
            http::StatusCode::NOT_FOUND
        );
    }

    fn parse_response(
        response: &http::response::Response<bytes::Bytes>,
    ) -> Option<PokemonInShakespeareseResponse> {
        serde_json::from_slice(&response.body().iter().cloned().collect::<Vec<_>>())
            .ok()
            .map(
                |response: PokemonInShakespeareseResponse| PokemonInShakespeareseResponse {
                    name: response.name,
                    description: response.description.to_lowercase(),
                },
            )
    }

    #[derive(serde::Deserialize)]
    struct AllPokemonsResponse {
        count: usize,
        results: Vec<AllPokemonsResponseEntry>,
    }

    #[derive(serde::Deserialize)]
    struct AllPokemonsResponseEntry {
        name: String,
        url: String,
    }

    #[tokio::test]
    #[ignore]
    async fn test_examine_descriptions_of_all_pokemons() {
        use std::collections::HashMap;

        let response = reqwest::get("https://pokeapi.co/api/v2/pokemon?limit=10000")
            .await
            .unwrap();
        assert!(response.status().is_success());
        let all_pokemons: AllPokemonsResponse =
            serde_json::from_str(&response.text().await.unwrap()).unwrap();
        assert_eq!(all_pokemons.count, all_pokemons.results.len());

        use futures::stream::StreamExt;
        let _descriptions = all_pokemons
            .results
            .into_iter()
            .map(|entry| async move {
                let description = describe_pokemon(&entry.name).await.ok();
                println!(
                    "Name {}, url {}, description {:?}",
                    &entry.name, &entry.url, &description
                );
                (entry.name.clone(), description)
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<HashMap<_, _>>()
            .await;

        let description_count = _descriptions
            .values()
            .filter(|value| value.is_some())
            .count();
        println!(
            "Total pokemons: {}, pokemons with description: {}",
            all_pokemons.count, description_count
        );
    }
}

async fn shakespearise(input: &str) -> Result<String> {
    let request_url = reqwest::Url::parse_with_params(
        "https://api.funtranslations.com/translate/shakespeare.json",
        &[("text", input)],
    )?;
    let response = reqwest::get(request_url).await?;
    if !response.status().is_success() {
        return Err(RequestError::new(response.status(), "Failed to query Shakespeare API").into());
    }
    let response_json: serde_json::Value = serde_json::from_str(&response.text().await?)?;
    response_json["contents"]["translated"]
        .as_str()
        .map(str::to_string)
        .ok_or(
            RequestError::new(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to shakespearise the text",
            )
            .into(),
        )
}

async fn shakespearise_ignore_rate_limit_error(
    pokemon_name: &str,
    input_description: String,
) -> Result<String> {
    shakespearise(&input_description)
    .await
    .or_else(|err| match err.downcast_ref::<RequestError>() {
        Some(RequestError {
            status: http::StatusCode::TOO_MANY_REQUESTS,
            ..
        }) => {
            eprintln!(
                "Request \"{}\" to Shakespeare translation service hit the API rate limit and will be returned untranslated",
                pokemon_name
            );
            Ok(input_description)
        }
        _ => Err(err),
    })
}

async fn respond_with_pokemon_in_shakespearese(
    pokemon_name: String,
) -> std::result::Result<impl warp::Reply, warp::Rejection> {
    let request_start_time = std::time::Instant::now();
    use futures::future::TryFutureExt;
    let pokemon_name = &pokemon_name;
    let description_result = describe_pokemon(pokemon_name)
        .and_then(
            |desc| async move { shakespearise_ignore_rate_limit_error(pokemon_name, desc).await },
        )
        .await
        .and_then(|description| {
            Ok(serde_json::to_string_pretty(
                &PokemonInShakespeareseResponse::new(pokemon_name.to_lowercase(), description),
            )?)
        });
    let response = match description_result {
        Ok(json_response) => http::response::Builder::new()
            .header("Content-Type", "application/json; charset=UTF-8")
            .status(http::StatusCode::OK)
            .body(json_response)
            .unwrap(),
        Err(err) => {
            eprintln!("Request \"{}\" failed with error {:?}", pokemon_name, &err);
            let status_code = if let Some(response_error) = err.downcast_ref::<RequestError>() {
                response_error.status
            } else {
                http::StatusCode::INTERNAL_SERVER_ERROR
            };
            http::response::Builder::new()
                .status(status_code)
                .header("Content-Type", "text/plain; charset=UTF-8")
                .body(format!(
                    "Error {}: {}",
                    status_code.as_u16(),
                    status_code.canonical_reason().unwrap_or("Unknown reason")
                ))
                .unwrap()
        }
    };
    let request_duration = request_start_time.elapsed();
    eprintln!(
        "Request \"{}\" took {} ms",
        &pokemon_name,
        request_duration.as_millis()
    );
    Ok(response)
}
