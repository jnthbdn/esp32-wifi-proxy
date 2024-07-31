use std::str::FromStr;
use std::sync::{Arc, Mutex};

use esp_idf_svc::{
    http::{self, server::EspHttpServer, Method},
    io::Write,
    wifi::{BlockingWifi, EspWifi},
};
use serde_json::Value;
use url_encoded_data::UrlEncodedData;

use crate::{nvs_configuration::NvsConfiguration, template};

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

pub fn create_http_server<'a>() -> anyhow::Result<EspHttpServer<'a>> {
    log::info!("Creating HTTP server.");
    let mut server = EspHttpServer::new(&http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    })?;

    server.fn_handler::<anyhow::Error, _>("/send_json", Method::Post, |mut req| {
        let len_body = req
            .header("Content-Length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let mut error_message = String::new();

        if len_body == 0 {
            error_message = "No body or no content-length".to_string();
        } else if len_body >= 256 {
            error_message = "Content-length too long.".to_string();
        } else {
            let mut buffer = [0u8; 256];

            match req.read(&mut buffer) {
                Result::Ok(bytes_read) => {
                    let post_str = String::from_utf8(buffer[0..bytes_read].to_vec())?;
                    let json_data = serde_json::from_str::<Value>(&post_str);

                    if json_data.is_err() {
                        error_message = "Invalid JSON".to_string();
                        log::error!("Invalid JSON ({})", json_data.err().unwrap());
                    } else {
                        let json_data = json_data.unwrap();
                        let json_data = json_data.as_object();

                        if json_data.is_none() {
                            error_message = "Invalid JSON format".to_string();
                        } else {
                            let json_data = json_data.unwrap();
                            error_message += "Save successfully!";
                            log::info!("Json data: {:?}", json_data);
                        }
                    }
                }
                Err(_) => {
                    error_message = "Save error: Failed to read request.".to_string();
                }
            };
        }

        req.into_ok_response()?
            .write_all(error_message.as_bytes())?;

        Ok(())
    })?;

    Ok(server)
}
