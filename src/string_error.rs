use core::fmt;

use esp_idf_svc::hal::sys::EspError;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct StringError(pub &'static str);

impl std::error::Error for StringError {}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct StringEspError(pub &'static str, pub EspError);

impl std::error::Error for StringEspError {}

impl fmt::Display for StringEspError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (EspError: {})", self.0, self.1)
    }
}
