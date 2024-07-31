use std::sync::atomic::{AtomicBool, Ordering};

use esp_idf_svc::nvs::{EspCustomNvsPartition, EspNvs, NvsCustom};
use pad::{Alignment, PadStr};

use crate::string_error::{StringError, StringEspError};

static IS_NVS_TAKEN: AtomicBool = AtomicBool::new(false);

const PARTITION_NAME: &str = "config";
const NAMESPACE: &str = "config";

const PAD_CHAR: char = 0x03 as char;

pub const KEY_STA_SSID: &str = "STASSID";
pub const KEY_STA_PASSPHRASE: &str = "STAPASS";
pub const KEY_AP_SSID: &str = "APSSID";
pub const KEY_AP_PASSPHRASE: &str = "APPASS";
pub const KEY_AP_SSID_HIDDEN: &str = "APHIDDEN";
pub const KEY_MQTT_SERVER: &str = "MQTTSRV";
pub const KEY_MQTT_PORT: &str = "MQTTPRT";

pub struct NvsConfiguration {
    nvs: EspNvs<NvsCustom>,
}

impl NvsConfiguration {
    pub fn take() -> Result<Self, StringError> {
        if IS_NVS_TAKEN.load(Ordering::Relaxed) {
            return Err(StringError("MainConfiguration NVS already taken"));
        }

        IS_NVS_TAKEN.store(true, Ordering::Relaxed);

        let nvs_custom = match EspCustomNvsPartition::take(PARTITION_NAME) {
            Ok(nvs) => nvs,
            Err(_) => return Err(StringError("Fail to take partition")),
        };

        match EspNvs::new(nvs_custom, NAMESPACE, true) {
            Ok(nvs) => Ok(Self { nvs }),
            Err(_) => Err(StringError("Failed to create EspNvs. Bad namespace ?")),
        }
    }

    pub fn get_sta_ssid(&self) -> String {
        self.read_string(KEY_STA_SSID, "")
    }

    pub fn get_sta_passphrase(&self) -> String {
        self.read_string(KEY_STA_PASSPHRASE, "")
    }

    pub fn get_ap_ssid(&self) -> String {
        self.read_string(KEY_AP_SSID, "ESP-WiFi Proxy")
    }

    pub fn get_ap_passphrase(&self) -> String {
        self.read_string(KEY_AP_PASSPHRASE, "")
    }

    pub fn get_ap_hidden_ssid(&self) -> bool {
        self.read_u8(KEY_AP_SSID_HIDDEN, 0) == 1
    }

    pub fn get_mqtt_server(&self) -> String {
        self.read_string(KEY_MQTT_SERVER, "")
    }

    pub fn get_mqtt_port(&self) -> u16 {
        self.read_u16(KEY_MQTT_PORT, 1883)
    }

    pub fn set_sta_ssid(&mut self, value: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_STA_SSID, value, 32)
    }

    pub fn set_sta_passphrase(&mut self, value: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_STA_PASSPHRASE, value, 63)
    }

    pub fn set_ap_ssid(&mut self, value: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_AP_SSID, value, 32)
    }

    pub fn set_ap_passphrase(&mut self, value: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_AP_PASSPHRASE, value, 63)
    }

    pub fn set_ap_hidden_ssid(&mut self, value: bool) -> Result<(), StringEspError> {
        self.store_u8(KEY_AP_SSID_HIDDEN, if value { 1 } else { 0 })
    }

    pub fn set_mqtt_server(&mut self, value: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_MQTT_SERVER, value, 128)
    }

    pub fn set_mqtt_port(&mut self, value: u16) -> Result<(), StringEspError> {
        self.store_u16(KEY_MQTT_PORT, value)
    }

    fn store_string(
        &mut self,
        key: &str,
        value: &str,
        max_size: usize,
    ) -> Result<(), StringEspError> {
        self.nvs
            .set_str(key, &Self::trunc_pad_string(value, max_size))
            .map_err(|e| StringEspError("Failed to store string", e))
    }

    fn read_string(&self, key: &str, default: &str) -> String {
        let size = self.nvs.str_len(key).unwrap_or(None).unwrap_or(0);
        let mut buf = vec![0; size];

        if size == 0 {
            return default.to_string();
        }

        let result = self
            .nvs
            .get_str(key, &mut buf)
            .unwrap_or(None)
            .unwrap_or(default)
            .to_string();

        result
            .split_once(PAD_CHAR)
            .unwrap_or((&result, ""))
            .0
            .to_owned()
    }

    fn store_u8(&mut self, key: &str, value: u8) -> Result<(), StringEspError> {
        self.nvs
            .set_u8(key, value)
            .map_err(|e| StringEspError("Failed to store U8", e))
    }

    fn read_u8(&self, key: &str, default: u8) -> u8 {
        self.nvs.get_u8(key).unwrap_or(None).unwrap_or(default)
    }

    fn store_u16(&mut self, key: &str, value: u16) -> Result<(), StringEspError> {
        self.nvs
            .set_u16(key, value)
            .map_err(|e| StringEspError("Failed to store U8", e))
    }

    fn read_u16(&self, key: &str, default: u16) -> u16 {
        self.nvs.get_u16(key).unwrap_or(None).unwrap_or(default)
    }

    fn trunc_pad_string(s: &str, max: usize) -> String {
        s.pad(max, PAD_CHAR, Alignment::Left, true)
    }
}

impl Drop for NvsConfiguration {
    fn drop(&mut self) {
        IS_NVS_TAKEN.store(false, Ordering::Relaxed);
    }
}
