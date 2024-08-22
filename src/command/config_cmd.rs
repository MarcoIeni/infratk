use crate::config::{self, Config};

pub fn create_default_config() {
    let config_dir = config::create_config_dir();
    let default_config = Config {
        legacy_otp: Some("op item get <ITEM_ID> --otp".to_string()),
    };
    let default_config = toml::to_string(&default_config).unwrap();
    let config_file = config::config_file(&config_dir);
    fs_err::write(&config_file, default_config).unwrap();
    println!("{config_file:?}");
}
