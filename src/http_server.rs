use std::str::FromStr;
use std::sync::{Arc, Mutex};

use esp_idf_svc::http::server::{EspHttpConnection, Request};
use esp_idf_svc::mqtt::client::{EspMqttClient, QoS};
use esp_idf_svc::{
    http::{self, server::EspHttpServer, Method},
    io::Write,
    wifi::{BlockingWifi, EspWifi},
};
use serde_json::{json, Map, Value};
use url_encoded_data::UrlEncodedData;

use crate::{nvs_configuration::NvsConfiguration, template};

const JSON_MANDATORY_KEYS: &[&str] = &["id"];

pub fn create_http_config_server<'a>(
    mutex_config: Arc<Mutex<NvsConfiguration>>,
    mutex_wifi: Arc<Mutex<BlockingWifi<EspWifi<'static>>>>,
) -> anyhow::Result<EspHttpServer<'a>> {
    log::info!("Creating configuration HTTP server.");
    let mut server = EspHttpServer::new(&http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    })?;

    let handler_config = mutex_config.clone();
    let handler_wifi = mutex_wifi.clone();
    server.fn_handler("/", Method::Get, move |req| {
        req.into_ok_response()?
            .write_all(
                template::to_html(
                    &handler_config.lock().unwrap(),
                    handler_wifi.lock().unwrap().scan().ok(),
                    None,
                )
                .as_bytes(),
            )
            .map(|_| ())
    })?;

    let handler_config = mutex_config.clone();
    let handler_wifi = mutex_wifi.clone();
    server.fn_handler::<anyhow::Error, _>("/", Method::Post, move |mut req| {
        let len_body = req
            .header("Content-Length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let mut error_message = String::new();

        if len_body == 0 {
            error_message = "Save error: No body or no content-length".to_string();
        } else if len_body >= 256 {
            error_message = "Save error: Content-length too long.".to_string();
        } else {
            let mut buffer = [0u8; 256];

            match req.read(&mut buffer) {
                Result::Ok(bytes_read) => {
                    let post_str = String::from_utf8(buffer[0..bytes_read].to_vec())?;
                    let post_data = UrlEncodedData::parse_str(&post_str);

                    let mut config_mut = handler_config.lock().unwrap();

                    if post_data.exists("mqttsrv") {
                        config_mut.set_mqtt_server(&post_data.get_first("mqttsrv").unwrap())?;
                    }

                    if post_data.exists("mqttprt") {
                        config_mut.set_mqtt_port(u16::from_str(
                            &post_data.get_first("mqttprt").unwrap(),
                        )?)?;
                    }

                    if post_data.exists("apssid") {
                        config_mut.set_ap_ssid(&post_data.get_first("apssid").unwrap())?;
                    }

                    if post_data.exists("appass") {
                        let pass = post_data.get_first("appass").unwrap();

                        if pass.len() == 0 || pass.len() >= 8 {
                            config_mut.set_ap_passphrase(pass)?;
                        } else {
                            error_message =
                                "Save warning: The passphrase minimum length is 8 characters...\n"
                                    .to_string();
                        }
                    }

                    if post_data.exists("stassid") {
                        config_mut.set_sta_ssid(&post_data.get_first("stassid").unwrap())?;
                    }

                    if post_data.exists("stapass") {
                        let pass = post_data.get_first("stapass").unwrap();

                        if pass.len() == 0 || pass.len() >= 8 {
                            config_mut.set_sta_passphrase(pass)?;
                        } else {
                            error_message =
                                "Save warning: The passphrase minimum length is 8 characters...\n"
                                    .to_string();
                        }
                    }

                    config_mut.set_ap_hidden_ssid(post_data.exists("apishidden"))?;

                    error_message += "Save successfully!";
                }
                Err(_) => {
                    error_message = "Save error: Failed to read request.".to_string();
                }
            };
        }

        req.into_ok_response()?.write_all(
            template::to_html(
                &handler_config.lock().unwrap(),
                handler_wifi.lock().unwrap().scan().ok(),
                Some(error_message),
            )
            .as_bytes(),
        )?;

        Ok(())
    })?;

    Ok(server)
}

pub fn create_http_server<'a>(
    mutex_mqtt: Arc<Mutex<EspMqttClient<'static>>>,
) -> anyhow::Result<EspHttpServer<'a>> {
    log::info!("Creating HTTP server.");
    let mut server = EspHttpServer::new(&http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    })?;

    let mqtt = mutex_mqtt.clone();
    server.fn_handler::<anyhow::Error, _>(
        "/send_soil_moisture",
        Method::Post,
        move |mut req| {
            let json = extract_json_from_request(&mut req);

            if json.is_err() {
                req.into_status_response(400)?
                    .write_all(json.as_ref().err().unwrap().as_bytes())?;
                return Ok(());
            }

            let json = json.unwrap();

            if !object_contains_keys(&json, &["level", "battery"]) {
                req.into_status_response(400)?
                    .write_all("Missing keys".as_bytes())?;
                return Ok(());
            }

            mqtt.lock().unwrap().publish(
                &format!(
                    "sensor/soil_moisture/{}",
                    json["id"].as_str().ok_or(anyhow::Error::msg("Bad ID"))?
                ),
                QoS::AtLeastOnce,
                false,
                json!({
                    "level": json["level"],
                    "battery": json["battery"]
                })
                .to_string()
                .as_bytes(),
            )?;

            req.into_status_response(200)?;
            Ok(())
        },
    )?;

    let mqtt = mutex_mqtt.clone();
    server.fn_handler::<anyhow::Error, _>("/send_water_level", Method::Post, move |mut req| {
        let json = extract_json_from_request(&mut req);

        if json.is_err() {
            req.into_status_response(400)?
                .write_all(json.as_ref().err().unwrap().as_bytes())?;
            return Ok(());
        }

        let json = json.unwrap();

        if !object_contains_keys(&json, &["level", "measure", "battery"]) {
            req.into_status_response(400)?
                .write_all("Missing keys".as_bytes())?;
            return Ok(());
        }

        mqtt.lock().unwrap().publish(
            &format!(
                "sensor/water_level/{}",
                json["id"].as_str().ok_or(anyhow::Error::msg("Bad ID"))?
            ),
            QoS::AtLeastOnce,
            false,
            json!({
                "level": json["level"],
                "raw": json["measure"],
                "battery": json["battery"]
            })
            .to_string()
            .as_bytes(),
        )?;

        req.into_status_response(200)?;
        Ok(())
    })?;

    Ok(server)
}

fn object_contains_keys(json: &Map<String, Value>, additional_key: &[&str]) -> bool {
    JSON_MANDATORY_KEYS.iter().all(|&x| json.contains_key(x))
        && additional_key.iter().all(|&x| json.contains_key(x))
}

fn extract_json_from_request(
    req: &mut Request<&mut EspHttpConnection>,
) -> Result<Map<String, Value>, &'static str> {
    let len_body = req
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    if len_body == 0 {
        return Err("No body or no content-length");
    } else if len_body >= 256 {
        return Err("Content-length too long.");
    } else {
        let mut buffer = [0u8; 256];

        match req.read(&mut buffer) {
            Ok(bytes_read) => {
                let post_str = String::from_utf8(buffer[0..bytes_read].to_vec()).unwrap();

                match serde_json::from_str::<Value>(&post_str) {
                    Ok(json_value) => match json_value {
                        Value::Object(obj) => Ok(obj),
                        _ => Err("Invalid JSON (no object)"),
                    },
                    Err(e) => {
                        log::error!("Invalid JSON (Error: {}).", e);
                        Err("Invalid JSON")
                    }
                }
            }
            Err(e) => {
                log::error!("Read error: {}", e);
                Err("Save error: Failed to read request.")
            }
        }
    }
}
