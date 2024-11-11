use anyhow::{bail, Result};
use config::HomeAssistantConfig;
use embedded_svc::http::{client::Client, Method};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use log::info;

/// Generic state struct that only captures the state, no other fields of interest.
/// More complex structs can be defined for entities with more interesting fields.
#[derive(serde::Deserialize, Debug)]
pub struct EntityState {
    pub state: String,
}

pub fn get_entity_state<S: serde::de::DeserializeOwned>(
    ha_config: &HomeAssistantConfig,
    entity_id: &str,
) -> Result<S> {
    let connection = EspHttpConnection::new(&Configuration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut client = Client::wrap(connection);

    let auth_header = format!("Bearer {}", ha_config.access_token);
    let headers = [
        ("content-type", "application/json"),
        ("Authorization", &auth_header),
    ];
    let request_url = format!("{}/api/states/{}", &ha_config.base_url, entity_id);
    info!("Connecting to {}", request_url);
    let request = client.request(Method::Get, &request_url, &headers)?;
    let response = request.submit()?;

    if !(200..=299).contains(&response.status()) {
        bail!(
            "HTTP request failed, error code {}: {}",
            response.status(),
            response.status_message().unwrap_or_default()
        );
    }

    let result: S = serde_json::from_reader(embedded_io_adapters::std::ToStd::new(response))?;
    Ok(result)
}
