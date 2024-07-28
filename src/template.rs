use std::sync::Mutex;

use crate::nvs_configuration::NvsConfiguration;

const BASE_HTML: &str = include_str!("html/base.html");

pub fn to_html(main_config: &Mutex<NvsConfiguration>, error_message: Option<String>) -> String {
    let mut template = BASE_HTML.to_string();
    let config = main_config.lock().unwrap();

    template = template.replace("{ERROR_MSG}", &error_message.unwrap_or("".to_string()));
    template = template.replace("{SSID}", &config.get_ssid());
    template = template.replace("{PASS}", &config.get_passphrase());
    template = template.replace(
        "{HIDDEN_CHECKED}",
        if config.get_hidden_ssid() {
            "checked"
        } else {
            ""
        },
    );

    template
}
