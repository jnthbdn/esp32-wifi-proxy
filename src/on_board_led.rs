use esp_idf_svc::hal::gpio::*;

pub struct OnBoardLed<'a> {
    pub red: PinDriver<'a, Gpio3, Output>,
    pub green: PinDriver<'a, Gpio4, Output>,
    pub blue: PinDriver<'a, Gpio5, Output>,
}

impl<'a> OnBoardLed<'a> {
    pub fn new(pin_3: Gpio3, pin_4: Gpio4, pin_5: Gpio5) -> anyhow::Result<Self> {
        let mut s = Self {
            red: PinDriver::output(pin_3)?,
            green: PinDriver::output(pin_4)?,
            blue: PinDriver::output(pin_5)?,
        };

        s.red.set_low()?;
        s.green.set_low()?;
        s.blue.set_low()?;

        Ok(s)
    }
}
