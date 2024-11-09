fn main() {
    // Verify the config can be parsed at build-time, to prevent config parsing errors at runtime.
    config::Config::load_or_panic();

    embuild::espidf::sysenv::output();
}
