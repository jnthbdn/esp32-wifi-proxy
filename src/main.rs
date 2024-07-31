use std::{
    sync::{Arc, Mutex},
    time::SystemTime,
};

use esp_idf_svc::{
    hal::{
        delay::FreeRtos,
        gpio::{Output, OutputPin, PinDriver, Pull},
        peripherals::Peripherals,
    },
    http::server::EspHttpServer,
    mqtt::client::{EspMqttClient, MqttClientConfiguration},
    wifi::{BlockingWifi, EspWifi},
};

use http_server::{create_http_config_server, create_http_server};
use nvs_configuration::NvsConfiguration;
use on_board_led::OnBoardLed;
use wifi_helper::{create_ap_sta_wifi, create_ap_wifi};

mod http_server;
mod nvs_configuration;
mod on_board_led;
mod string_error;
mod template;
mod wifi_helper;

const MQTT_CLIENT_ID: &str = "SENSOR_WIFI_PROXY";

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let nvs_config = Arc::new(Mutex::new(NvsConfiguration::take().unwrap()));
    let wifi: Arc<Mutex<BlockingWifi<EspWifi>>>;

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;

    let _http_server: EspHttpServer;
    let is_config_mode: bool;
    let mut leds = OnBoardLed::new(pins.gpio3, pins.gpio4, pins.gpio5)?;

    let mut settings_button = PinDriver::input(pins.gpio9)?;
    settings_button.set_pull(Pull::Up)?;

    log::info!("Wait 5 seconds for activation of configuration web server...");
    leds.blue.set_high()?;
    FreeRtos::delay_ms(5000);
    is_config_mode = settings_button.is_low();
    leds.blue.set_low()?;

    if is_config_mode {
        log::info!("CONFIGURATION MODE");
        let ap_wifi = create_ap_wifi(peripherals.modem, &nvs_config.lock().unwrap());

        if ap_wifi.is_err() {
            log::error!("Failed to create AP !. Restart in 5 sec...");
            log::error!("{}", ap_wifi.as_ref().err().unwrap());

            flash_led_and_restart(&mut leds.red, 5);
        }

        wifi = Arc::new(Mutex::new(ap_wifi.unwrap()));

        _http_server = create_http_config_server(nvs_config.clone(), wifi.clone())?;
    } else {
        leds.red.set_high()?;

        let ap_sta_wifi = create_ap_sta_wifi(peripherals.modem, &nvs_config.lock().unwrap());

        if ap_sta_wifi.is_err() {
            log::error!("Failed to create AP !. Restart in 5 sec...");
            log::error!("{}", ap_sta_wifi.as_ref().err().unwrap());

            flash_led_and_restart(&mut leds.red, 5);
        }

        leds.red.set_low()?;

        //wifi = Arc::new(Mutex::new(ap_sta_wifi.unwrap()));

        _http_server = create_http_server()?;

        leds.green.set_high()?;

        let mqtt_client = EspMqttClient::new_cb(
            &make_mqtt_url(&nvs_config.lock().unwrap()),
            &MqttClientConfiguration {
                client_id: Some(MQTT_CLIENT_ID),
                ..Default::default()
            },
            |event| {
                log::info!("[MQTT Event]: {}", event.payload());
            },
        );

        if mqtt_client.is_err() {
            log::error!(
                "Failed to connect MQTT client (Error: {:?})",
                mqtt_client.as_ref().err().unwrap()
            );

            flash_led_and_restart(&mut leds.green, 5);
        }

        leds.green.set_low()?;
    }

    loop {
        FreeRtos::delay_ms(250);

        if is_config_mode {
            leds.blue.toggle()?;
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}

fn flash_led_and_restart<T: OutputPin>(led: &mut PinDriver<T, Output>, timeout_sec: u64) {
    let timeout = SystemTime::now();

    loop {
        let _ = led.set_high();
        FreeRtos::delay_ms(100);
        let _ = led.set_low();
        FreeRtos::delay_ms(100);

        if timeout.elapsed().unwrap().as_secs() >= timeout_sec {
            esp_idf_svc::hal::reset::restart();
        }
    }
}

fn make_mqtt_url(config: &NvsConfiguration) -> String {
    format!("{}:{}", config.get_mqtt_server(), config.get_mqtt_port())
}
