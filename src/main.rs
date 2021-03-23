extern crate reqwest;
extern crate serde_json;
extern crate tokio;

type Result<T> = std::result::Result<T, std::boxed::Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    println!("Pokemon: {:#?}", describe_pokemon("ditto").await);
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
    let pokemon_response_json: serde_json::Value =
        serde_json::from_str(&pokemon_response.text().await?)?;
    let description_url = pokemon_response_json["id"]
        .as_u64()
        .map(|id| format!("https://pokeapi.co/api/v2/pokemon-species/{}/", id))
        .ok_or(Error::new(
            ErrorKind::InvalidData,
            format!("Didn't find any form of pokemon {}", &pokemon_name),
        ))?;

    let description_response = reqwest::get(description_url).await?;

    if !description_response.status().is_success() {
        return Err(RequestError::new(
            description_response.status(),
            format!("Failed to get a description for pokemon {}", &pokemon_name),
        )
        .into());
    }
    let description_response_json: serde_json::Value =
        serde_json::from_str(&description_response.text().await?)?;

    let descriptions = description_response_json["flavor_text_entries"]
        .as_array()
        .ok_or(Error::new(
            ErrorKind::InvalidData,
            format!("Failed to parse descriptions for pokemon {}", &pokemon_name),
        ))?
        .into_iter()
        .filter(|entry| entry["language"]["name"].as_str() == Some("en"))
        .collect::<Vec<_>>();

    descriptions
        .iter()
        .find(|entry| entry["version"]["name"].as_str() == Some("ruby"))
        .or(descriptions.iter().max_by_key(|entry| {
            entry["flavor_text"]
                .as_str()
                .map(|text| text.len())
                .unwrap_or(0)
        }))
        .and_then(|entry| entry["flavor_text"].as_str().map(str::to_string))
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
}

// fn shakespearise(input: &str) -> std::result::Result<String, RequestError> {}
