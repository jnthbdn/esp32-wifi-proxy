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

use std::{net::Ipv4Addr, str::FromStr};

pub fn create_ap<'a>(
    modem: impl Peripheral<P = Modem> + 'a,
    ap_name: &str,
    password: &str,
    is_hidden: bool,
) -> anyhow::Result<BlockingWifi<EspWifi<'a>>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let wifi_drv = WifiDriver::new(modem, sys_loop.clone(), Some(nvs))?;
    let wifi_esp = EspWifi::wrap_all(
        wifi_drv,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: ipv4::Configuration::Router(ipv4::RouterConfiguration {
                subnet: Subnet {
                    gateway: Ipv4Addr::from_str("192.168.70.1")?,
                    mask: Mask(24),
                },
                ..Default::default()
            }),
            ..NetifConfiguration::wifi_default_router()
        })?,
    )?;

    let mut wifi = BlockingWifi::wrap(wifi_esp, sys_loop)?;

    let cc = wifi_country_t {
        cc: [b'F' as i8, b'R' as i8, 0 as i8],
        schan: 1,
        nchan: 14,
        max_tx_power: 80,
        ..Default::default()
    };

    esp!(unsafe { esp_wifi_set_country(&cc) })?;

    let wifi_configuration = Configuration::Mixed(
        ClientConfiguration {
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: ap_name.try_into().unwrap(),
            ssid_hidden: is_hidden,
            auth_method: if password.is_empty() {
                AuthMethod::None
            } else {
                AuthMethod::WPA2Personal
            },
            password: password.try_into().unwrap(),
            max_connections: 10,
            channel: 11,
            ..Default::default()
        },
    );

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;

    Ok(wifi)
}
