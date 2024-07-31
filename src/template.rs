use esp_idf_svc::wifi::AccessPointInfo;

use crate::nvs_configuration::NvsConfiguration;

const BASE_HTML: &str = include_str!("html/base.html");

pub fn to_html(
    config: &NvsConfiguration,
    aps: Option<Vec<AccessPointInfo>>,
    error_message: Option<String>,
) -> String {
    let mut template = BASE_HTML.to_string();

    template = template.replace("{ERROR_MSG}", &error_message.unwrap_or("".to_string()));
    template = template.replace("{AP_LIST}", &accespoint_to_template(aps));
    template = template.replace("{MQTTSRV}", &config.get_mqtt_server());
    template = template.replace("{MQTTPRT}", &format!("{}", config.get_mqtt_port()));
    template = template.replace("{STASSID}", &config.get_sta_ssid());
    template = template.replace("{STAPASS}", &config.get_sta_passphrase());
    template = template.replace("{APSSID}", &config.get_ap_ssid());
    template = template.replace("{APPASS}", &config.get_ap_passphrase());
    template = template.replace(
        "{APHIDDEN_CHECKED}",
        if config.get_ap_hidden_ssid() {
            "checked"
        } else {
            ""
        },
    );

    template
}

fn accespoint_to_template(aps: Option<Vec<AccessPointInfo>>) -> String {
    let mut result = String::new();

    if aps.is_none() {
        return result;
    } else {
        let aps = aps.unwrap();

        result += "[";
        for ap in aps {
            result += &format!("{{ssid:\"{}\",rssi:{}}},", ap.ssid, ap.signal_strength);
        }
        result += "]";
    }

    result
}
