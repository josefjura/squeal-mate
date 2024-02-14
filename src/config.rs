use std::collections::HashMap;

use config::Config;

pub fn setup_config() -> HashMap<String, String> {
    let settings = Config::builder()
        // Add in `./Settings.toml`
        .add_source(config::File::with_name("aeq-cac"))
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("AEQ-CAC"))
        .build();

    match settings {
        Ok(content) => match content.try_deserialize::<HashMap<String, String>>() {
            Ok(parsed) => parsed,
            Err(_) => HashMap::<String, String>::new(),
        },
        Err(_) => HashMap::<String, String>::new(),
    }
}
