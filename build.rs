fn main() {
    // Parse and dump the nice (std) toml-format config to a binary (no_std) format.
    let config: config::Config = toml::from_str(include_str!("deployment_config.toml")).unwrap();
    let config_bytes = postcard::to_allocvec(&config).unwrap();
    std::fs::write("deployment_config.postcard", config_bytes).unwrap();
}
