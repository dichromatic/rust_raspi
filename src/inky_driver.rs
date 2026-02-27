use embedded_hal as hal;
use hal::digital::v2::{InputPin, OutputPin};
use hal::blocking::spi::Write;
use hal::blocking::delay::DelayMs;

// command constants for SSD1675 controller from datasheet
const DRIVER_OUTPUT_CONTROL: u8 = 0x01;
const BOOSTER_SOFT_START_CONTROL: u8 = 0x0C;
const GATE_SCAN_START_POSITION: u8 = 0x0F;
const DEEP_SLEEP_MODE: u8 = 0x10;
const DATA_ENTRY_MODE_SETTING: u8 = 0x11;
const SW_RESET: u8 = 0x12;
const TEMPERATURE_SENSOR_CONTROL: u8 = 0x1A;
const MASTER_ACTIVATION: u8 = 0x20;
const DISPLAY_UPDATE_CONTROL_1: u8 = 0x21;
const DISPLAY_UPDATE_CONTROL_2: u8 = 0x22;
const WRITE_RAM_BW: u8 = 0x24;
const WRITE_RAM_RED: u8 = 0x26;
const WRITE_VCOM_REGISTER: u8 = 0x2C;
const WRITE_LUT_REGISTER: u8 = 0x32;
const SET_DUMMY_LINE_PERIOD: u8 = 0x3A;
const SET_GATE_TIME: u8 = 0x3B;
const BORDER_WAVEFORM_CONTROL: u8 = 0x3C;
const SET_RAM_X_ADDRESS_START_END_POSITION: u8 = 0x44;
const SET_RAM_Y_ADDRESS_START_END_POSITION: u8 = 0x45;
const SET_RAM_X_ADDRESS_COUNTER: u8 = 0x4E;
const SET_RAM_Y_ADDRESS_COUNTER: u8 = 0x4F;

#[derive(Debug)]
pub enum InkyError<SPIE, GPIOE> {
    Spi(SPIE),
    Gpio(GPIOE),
}

pub struct InkyPhat<SPI, CS, BUSY, DC, RESET> {
    spi: SPI,
    cs: CS,
    busy: BUSY,
    dc: DC,
    reset: RESET,
}

// Inky pHAT pinout:
// 1: VCC
// 2: GND
// 3: SCK (SPI Clock) -> SPI
// 4: MOSI (SPI Data) -> SPI
// 5: CS (Chip Select) -> CS
// 6: DC (Data/Command) -> DC
// 7: RST (Reset) -> RESET

impl<SPI, CS, BUSY, DC, RESET, SPIE, GPIOE> InkyPhat<SPI, CS, BUSY, DC, RESET>
where
    SPI: Write<u8, Error = SPIE>,
    CS: OutputPin<Error = GPIOE>,
    BUSY: InputPin<Error = GPIOE>,
    DC: OutputPin<Error = GPIOE>,
    RESET: OutputPin<Error = GPIOE>,
{
    pub fn new(spi: SPI, cs: CS, busy: BUSY, dc: DC, reset: RESET) -> Self {
        InkyPhat {
            spi, 
            cs, 
            busy, 
            dc, 
            reset,
        }
    }

    pub fn reset<D: DelayMs<u8>>(&mut self, delay: &mut D) -> Result<(), InkyError<SPIE, GPIOE>> {
        // Reset sequence to wake up screen: pull RST low, wait, pull high, wait
        self.reset.set_low().map_err(InkyError::Gpio)?;
        delay.delay_ms(100);
        self.reset.set_high().map_err(InkyError::Gpio)?;
        delay.delay_ms(100);
        Ok(())
    }

    fn send_command(&mut self, command: u8) -> Result<(), InkyError<SPIE, GPIOE>> {
        // Set DC low for command, pull CS low, send command byte, then pull CS high to release
        self.dc.set_low().map_err(InkyError::Gpio)?;
        self.cs.set_low().map_err(InkyError::Gpio)?;
        self.spi.write(&[command]).map_err(InkyError::Spi)?;
        self.cs.set_high().map_err(InkyError::Gpio)?;
        Ok(())
    }

    fn send_data(&mut self, data: &[u8]) -> Result<(), InkyError<SPIE, GPIOE>> {
        // Set DC high for data, pull CS low, send data bytes, then pull CS high to release
        self.dc.set_high().map_err(InkyError::Gpio)?;
        self.cs.set_low().map_err(InkyError::Gpio)?;
        self.spi.write(data).map_err(InkyError::Spi)?;
        self.cs.set_high().map_err(InkyError::Gpio)?;
        Ok(())
    }

    fn send_command_data(&mut self, command: u8, data: Option<&[u8]>) -> Result<(), InkyError<SPIE, GPIOE>> {
        // Helper function to send a command followed by optional data
        self.send_command(command)?;
        if let Some(data) = data {
            self.send_data(data)?;
        }
        Ok(())
    }

    fn busy_wait<D: DelayMs<u8>>(&mut self, delay: &mut D) -> Result<(), InkyError<SPIE, GPIOE>> {
        // While the busy pin is high,
        while self.busy.is_high().map_err(InkyError::Gpio)? {
            // Wait 10ms 
            delay.delay_ms(10);
        }
        Ok(())
    }

    fn set_ram_address_counter(&mut self, x: u8, y: u16) -> Result<(), InkyError<SPIE, GPIOE>> {
        // Set RAM X address counter (0 to 12 for 104 pixels)
        self.send_command_data(SET_RAM_X_ADDRESS_COUNTER, Some(&[x]))?;
        // Set RAM Y address counter (split 16-bit y into two bytes)
        self.send_command_data(SET_RAM_Y_ADDRESS_COUNTER, Some(&[y as u8, (y >> 8) as u8]))?;
        Ok(())
    }

    pub fn init<D: DelayMs<u8>>(&mut self, delay: &mut D) -> Result<(), InkyError<SPIE, GPIOE>> {
        // Init sequence: 
        // call self.reset() to wake up the screen, then wait for busy to go low
        // Send SW_RESET command, then wait for busy to go low again
        // Send DRIVER_OUTPUT_CONTROL command with parameters to set resolution
        // Send DATA_ENTRY_MODE_SETTING command with parameters to set data entry mode

        self.reset(delay)?;
        self.busy_wait(delay)?;

        self.send_command(SW_RESET)?; // Software reset command 
        self.busy_wait(delay)?;
        // Set pixel height to 212 (0xD3) and width to 104 (0x00, 0x00 for 8-bit data)
        self.send_command_data(DRIVER_OUTPUT_CONTROL, Some(&[0xD3, 0x00, 0x00]))?; 
        // Set data entry mode to 0x03 (X increment, Y increment)
        self.send_command_data(DATA_ENTRY_MODE_SETTING, Some(&[0x03]))?; 
        // Set RAM X address start to 0 and end to 12 (for 104 pixels, 12 bytes)
        self.send_command_data(SET_RAM_X_ADDRESS_START_END_POSITION, Some(&[0x00, 0x0C]))?; 
        // Set RAM Y address start to 0 and end to 211 (0xD3) for 212 pixels
        self.send_command_data(SET_RAM_Y_ADDRESS_START_END_POSITION, Some(&[0x00, 0x00, 0xD3, 0x00]))?; 
        // Set border waveform control to set the colour of the very edge of the screen
        self.send_command_data(BORDER_WAVEFORM_CONTROL, Some(&[0x05]))?; 
        // Set display update control 1
        self.send_command_data(DISPLAY_UPDATE_CONTROL_1, Some(&[0x00, 0x80]))?; 
        // Set display update control 2
        self.send_command_data(DISPLAY_UPDATE_CONTROL_2, Some(&[0xC7]))?; 
        
       // set resolution, data entry modes, etc...
        Ok(())
    }   

    pub fn update_bw(&mut self, buffer: &[u8]) -> Result<(), InkyError<SPIE, GPIOE>> {
        // Set RAM address counter to (0,0)
        self.set_ram_address_counter(0, 0)?;
        // Send WRITE_RAM_BW command followed by the black/white buffer data
        self.send_command_data(WRITE_RAM_BW, Some(buffer))?;
        Ok(())
    }

    pub fn update_red(&mut self, buffer: &[u8]) -> Result<(), InkyError<SPIE, GPIOE>> {
        // Set RAM address counter to (0,0)
        self.set_ram_address_counter(0, 0)?;
        // Send WRITE_RAM_RED command followed by the red buffer data
        self.send_command_data(WRITE_RAM_RED, Some(buffer))?;
        Ok(())
    }

    pub fn display_refresh<D: DelayMs<u8>>(&mut self, delay: &mut D) -> Result<(), InkyError<SPIE, GPIOE>> {
        self.send_command(MASTER_ACTIVATION)?; // Trigger display refresh
        self.busy_wait(delay)?; // Wait for refresh to complete
        Ok(())
    }

}