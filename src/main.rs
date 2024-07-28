use std::{
    sync::{LazyLock, Mutex},
    time::SystemTime,
};

use esp_idf_svc::{
    hal::{
        delay::FreeRtos,
        gpio::{PinDriver, Pull},
        peripherals::Peripherals,
    },
    http::{self, server::EspHttpServer, Method},
    io::Write,
};
use nvs_configuration::NvsConfiguration;
use on_board_led::OnBoardLed;
use url_encoded_data::UrlEncodedData;
use wifi_helper::create_ap;

mod nvs_configuration;
mod on_board_led;
mod string_error;
mod template;
mod wifi_helper;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;
    static NVS_CONFIG: LazyLock<Mutex<NvsConfiguration>> =
        LazyLock::new(|| Mutex::new(NvsConfiguration::take().unwrap()));

    let mut http_server: Option<EspHttpServer> = None;
    let mut leds = OnBoardLed::new(pins.gpio3, pins.gpio4, pins.gpio5)?;

    let mut settings_button = PinDriver::input(pins.gpio9)?;
    settings_button.set_pull(Pull::Up)?;

    leds.blue.set_high()?;

    log::info!("Start AP...");
    let main_config_lock = NVS_CONFIG.lock().unwrap();
    let ap_wifi = create_ap(
        peripherals.modem,
        &main_config_lock.get_ssid(),
        &main_config_lock.get_passphrase(),
        main_config_lock.get_hidden_ssid(),
    );
    drop(main_config_lock);

    if ap_wifi.is_err() {
        let ap_wifi = ap_wifi.err().unwrap();
        let timeout = SystemTime::now();

        log::error!("Failed to create AP !. Restart in 10 sec...");
        log::error!("{}", ap_wifi);

        loop {
            leds.red.set_high()?;
            FreeRtos::delay_ms(100);
            leds.red.set_low()?;
            FreeRtos::delay_ms(100);

            if timeout.elapsed().unwrap().as_secs() == 10 {
                esp_idf_svc::hal::reset::restart();
            }
        }
    }

    log::info!("Wait 5 seconds for activation of configuration web server...");
    FreeRtos::delay_ms(5000);

    if settings_button.is_low() {
        log::info!("Configuration web server available...");
        http_server = Some(create_http_server(&NVS_CONFIG)?);
    }

    if http_server.is_none() {
        leds.blue.set_low()?;
    }

    loop {
        FreeRtos::delay_ms(250);

        if http_server.is_some() {
            leds.blue.toggle()?;
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn create_http_server(
    mutex_config: &'static Mutex<NvsConfiguration>,
) -> anyhow::Result<EspHttpServer> {
    let mut server = EspHttpServer::new(&http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    })?;

    server.fn_handler("/", Method::Get, |req| {
        req.into_ok_response()?
            .write_all(template::to_html(mutex_config, None).as_bytes())
            .map(|_| ())
    })?;

    server.fn_handler::<anyhow::Error, _>("/", Method::Post, |mut req| {
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
            let mut buffer = [0u8; 156];

            match req.read(&mut buffer) {
                Result::Ok(bytes_read) => {
                    let post_str = String::from_utf8(buffer[0..bytes_read].to_vec())?;
                    let post_data = UrlEncodedData::parse_str(&post_str);

                    let mut mainconfig_lock = mutex_config.lock().unwrap();

                    if post_data.exists("ssid") {
                        mainconfig_lock.set_ssid(&post_data.get_first("ssid").unwrap())?;
                    }

                    if post_data.exists("pass") {
                        let pass = post_data.get_first("pass").unwrap();

                        if pass.len() == 0 || pass.len() >= 8 {
                            mainconfig_lock.set_passphrase(pass)?;
                        } else {
                            error_message =
                                "Save warning: The passphrase minimum length is 8 characters...\n"
                                    .to_string();
                        }
                    }

                    mainconfig_lock.set_hidden_ssid(post_data.exists("is_hidden"))?;

                    error_message += "Save successfully!";
                }
                Err(_) => {
                    error_message = "Save error: Failed to read request.".to_string();
                }
            };
        }

        req.into_ok_response()?
            .write_all(template::to_html(mutex_config, Some(error_message)).as_bytes())?;

        Ok(())
    })?;

    Ok(server)
}
