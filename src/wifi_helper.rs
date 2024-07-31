use esp_idf_svc::hal::sys::esp;
use esp_idf_svc::hal::sys::esp_wifi_set_country;
use esp_idf_svc::hal::{modem::Modem, peripheral::Peripheral, sys::wifi_country_t};
use esp_idf_svc::wifi::AccessPointConfiguration;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    ipv4::{self, Mask, Subnet},
    netif::{EspNetif, NetifConfiguration, NetifStack},
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, WifiDriver},
};
use lazy_static::lazy_static;

use std::{net::Ipv4Addr, str::FromStr};

use crate::nvs_configuration::NvsConfiguration;

lazy_static! {
    static ref AP_NETIF_CONFIG: NetifConfiguration = NetifConfiguration {
        ip_configuration: ipv4::Configuration::Router(ipv4::RouterConfiguration {
            subnet: Subnet {
                gateway: Ipv4Addr::from_str("192.168.70.1").unwrap(),
                mask: Mask(24),
            },
            ..Default::default()
        }),
        ..NetifConfiguration::wifi_default_router()
    };
    static ref WIFI_COUNTRY_SETTING: wifi_country_t = wifi_country_t {
        cc: [b'F' as i8, b'R' as i8, 0 as i8],
        schan: 1,
        nchan: 14,
        max_tx_power: 80,
        ..Default::default()
    };
}

pub fn create_ap_sta_wifi<'a>(
    modem: impl Peripheral<P = Modem> + 'a,
    main_config: &NvsConfiguration,
) -> anyhow::Result<BlockingWifi<EspWifi<'a>>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let wifi_drv = WifiDriver::new(modem, sys_loop.clone(), Some(nvs))?;
    let wifi_esp = EspWifi::wrap_all(
        wifi_drv,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new_with_conf(&AP_NETIF_CONFIG)?,
    )?;

    let mut wifi = BlockingWifi::wrap(wifi_esp, sys_loop)?;

    esp!(unsafe { esp_wifi_set_country(&*WIFI_COUNTRY_SETTING) })?;

    let wifi_configuration = Configuration::Mixed(
        generate_client_configuration(main_config),
        generate_accespoint_configuration(main_config),
    );

    wifi.set_configuration(&wifi_configuration)?;

    log::info!("Start WiFi...");
    wifi.start()?;

    log::info!("Connect WiFi...");
    wifi.connect()?;

    log::info!("Wait network interface...");
    wifi.wait_netif_up()?;

    Ok(wifi)
}

pub fn create_ap_wifi<'a>(
    modem: impl Peripheral<P = Modem> + 'a,
    main_config: &NvsConfiguration,
) -> anyhow::Result<BlockingWifi<EspWifi<'a>>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let wifi_drv = WifiDriver::new(modem, sys_loop.clone(), Some(nvs))?;
    let wifi_esp = EspWifi::wrap_all(
        wifi_drv,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new_with_conf(&AP_NETIF_CONFIG)?,
    )?;

    let mut wifi = BlockingWifi::wrap(wifi_esp, sys_loop)?;

    esp!(unsafe { esp_wifi_set_country(&*WIFI_COUNTRY_SETTING) })?;

    wifi.set_configuration(&Configuration::AccessPoint(
        generate_accespoint_configuration(main_config),
    ))?;

    log::info!("Start WiFi...");
    wifi.start()?;

    Ok(wifi)
}

fn generate_client_configuration(main_config: &NvsConfiguration) -> ClientConfiguration {
    ClientConfiguration {
        ssid: main_config.get_sta_ssid().as_str().try_into().unwrap(),
        bssid: None,
        auth_method: if main_config.get_sta_passphrase().is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        },
        password: main_config
            .get_sta_passphrase()
            .as_str()
            .try_into()
            .unwrap(),
        channel: None,
        ..Default::default()
    }
}

fn generate_accespoint_configuration(main_config: &NvsConfiguration) -> AccessPointConfiguration {
    AccessPointConfiguration {
        ssid: main_config.get_ap_ssid().as_str().try_into().unwrap(),
        ssid_hidden: main_config.get_ap_hidden_ssid(),
        auth_method: if main_config.get_ap_passphrase().is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        },
        password: main_config.get_ap_passphrase().as_str().try_into().unwrap(),
        max_connections: 10,
        channel: 11,
        ..Default::default()
    }
}
