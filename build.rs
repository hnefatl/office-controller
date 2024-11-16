const CONFIG: &str = include_str!("deployment_config.toml");

fn main() {
    // Only rerun this build script if these files change: avoids
    // rebuilds for binary code.
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=config/");
    println!("cargo::rerun-if-changed=deployment_config.toml");
    embuild::espidf::sysenv::output();

    // Verify the config can be parsed at build-time, to prevent config parsing errors at runtime.
    config::Config::parse_or_panic(CONFIG);
}
