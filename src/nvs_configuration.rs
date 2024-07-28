use std::sync::atomic::{AtomicBool, Ordering};

use esp_idf_svc::nvs::{EspCustomNvsPartition, EspNvs, NvsCustom};
use pad::{Alignment, PadStr};

use crate::string_error::{StringError, StringEspError};

static IS_NVS_TAKEN: AtomicBool = AtomicBool::new(false);

const PARTITION_NAME: &str = "config";
const NAMESPACE: &str = "config";

const PAD_CHAR: char = 0x03 as char;

pub const KEY_SSID: &str = "SSID";
pub const KEY_PASSPHRASE: &str = "PASS";
pub const KEY_SSID_HIDDEN: &str = "HIDDEN";

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

    pub fn get_ssid(&self) -> String {
        self.read_string(KEY_SSID, "ESP-Nrf Proxy")
    }

    pub fn get_passphrase(&self) -> String {
        self.read_string(KEY_PASSPHRASE, "")
    }

    pub fn get_hidden_ssid(&self) -> bool {
        self.read_u8(KEY_SSID_HIDDEN, 0) == 1
    }

    pub fn set_ssid(&mut self, value: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_SSID, value, 32)
    }

    pub fn set_passphrase(&mut self, value: &str) -> Result<(), StringEspError> {
        self.store_string(KEY_PASSPHRASE, value, 63)
    }

    pub fn set_hidden_ssid(&mut self, value: bool) -> Result<(), StringEspError> {
        self.store_u8(KEY_SSID_HIDDEN, if value { 1 } else { 0 })
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

    fn trunc_pad_string(s: &str, max: usize) -> String {
        s.pad(max, PAD_CHAR, Alignment::Left, true)
    }
}

impl Drop for NvsConfiguration {
    fn drop(&mut self) {
        IS_NVS_TAKEN.store(false, Ordering::Relaxed);
    }
}
