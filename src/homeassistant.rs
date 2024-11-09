use anyhow::{bail, Result};
use config::HomeAssistantConfig;
use embedded_svc::http::{client::Client, Method};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use log::info;

pub struct StateSnapshot {
    pub person_location: EntityState,
}
impl StateSnapshot {
    pub fn get(ha_config: &HomeAssistantConfig) -> Result<StateSnapshot> {
        let person_location = get_entity_state(ha_config, &ha_config.person_entity)?;
        Ok(StateSnapshot { person_location })
    }
}

/// Generic state struct that only captures the state, no other fields of interest.
#[derive(serde::Deserialize, Debug)]
pub struct EntityState {
    pub state: String,
}

pub fn get_entity_state<State: serde::de::DeserializeOwned>(
    ha_config: &HomeAssistantConfig,
    entity_id: &str,
) -> Result<State> {
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

    let result: State = serde_json::from_reader(embedded_io_adapters::std::ToStd::new(response))?;
    Ok(result)
}
