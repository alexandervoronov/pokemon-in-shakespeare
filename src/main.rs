extern crate futures;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate warp;

type Result<T> = std::result::Result<T, std::boxed::Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    use warp::Filter;
    let pokemon_name = warp::path::param()
        .and(warp::get())
        .and_then(respond_with_pokemon_in_shakespearese);
    warp::serve(pokemon_name).run(([127, 0, 0, 1], 5000)).await;
}

#[derive(Debug)]
struct RequestError {
    status: reqwest::StatusCode,
    description: String,
}

impl RequestError {
    fn new<S: std::convert::Into<String>>(
        status: reqwest::StatusCode,
        description: S,
    ) -> RequestError {
        RequestError {
            status,
            description: description.into(),
        }
    }
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
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
struct PokemonResponse {
    id: u64,
}

#[derive(serde::Deserialize)]
struct PokemonDescriptionLanguage {
    name: String,
}

#[derive(serde::Deserialize)]
struct PokemonDescriptionVersion {
    name: String,
}

#[derive(serde::Deserialize)]
struct PokemonDescription {
    version: PokemonDescriptionVersion,
    flavor_text: String,
    language: PokemonDescriptionLanguage,
}

#[derive(serde::Deserialize)]
struct PokemonDescriptionResponse {
    #[serde(rename(deserialize = "flavor_text_entries"))]
    descriptions: Vec<PokemonDescription>,
}

async fn describe_pokemon(pokemon_name: &str) -> Result<String> {
    let pokemon_response = reqwest::get(format!(
        "https://pokeapi.co/api/v2/pokemon/{}",
        &pokemon_name.to_ascii_lowercase()
    ))
    .await?;
    if !pokemon_response.status().is_success() {
        return Err(RequestError::new(
            pokemon_response.status(),
            format!("Failed to get an id for pokemon {}", &pokemon_name),
        )
        .into());
    }

    use std::io::{Error, ErrorKind};
    let pokemon_response: PokemonResponse = serde_json::from_str(&pokemon_response.text().await?)?;
    let description_url = format!(
        "https://pokeapi.co/api/v2/pokemon-species/{}/",
        pokemon_response.id
    );

    let description_response = reqwest::get(description_url).await?;

    if !description_response.status().is_success() {
        return Err(RequestError::new(
            description_response.status(),
            format!("Failed to get a description for pokemon {}", &pokemon_name),
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
            Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Failed to extract a description for pokemon {}",
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
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to shakespearise the text",
            )
            .into(),
        )
}

async fn respond_with_pokemon_in_shakespearese(
    pokemon_name: String,
) -> std::result::Result<impl warp::Reply, warp::Rejection> {
    use futures::future::TryFutureExt;
    let description_result = describe_pokemon(&pokemon_name)
        .and_then(|desc| async move {
            shakespearise(&desc)
                .await
                .or_else(|err| match err.downcast_ref::<RequestError>() {
                    Some(RequestError {
                        status: reqwest::StatusCode::TOO_MANY_REQUESTS,
                        ..
                    }) => Ok(desc),
                    _ => Err(err),
                })
        })
        .await;
    match description_result {
        Ok(description) => Ok(http::response::Builder::new()
            .status(200)
            .body(description)
            .unwrap()),
        Err(err) => {
            let status_code = if let Some(response_error) = err.downcast_ref::<RequestError>() {
                response_error.status
            } else {
                reqwest::StatusCode::INTERNAL_SERVER_ERROR
            };
            Ok(http::response::Builder::new()
                .status(status_code)
                .body(format!(
                    "Error {}: {}",
                    status_code.as_u16(),
                    status_code.canonical_reason().unwrap_or("Unknown reason")
                ))
                .unwrap())
        }
    }
}
