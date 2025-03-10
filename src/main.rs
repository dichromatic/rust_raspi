// taken from ssd1675 driver example

extern crate linux_embedded_hal;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::sysfs_gpio::Direction;
use linux_embedded_hal::Delay;
use linux_embedded_hal::{Pin, Spidev};

extern crate ssd1675;
use ssd1675::{display, Builder, Color, Dimensions, Display, GraphicDisplay, Rotation};

// Graphics
#[macro_use]
extern crate embedded_graphics;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;

// Font
extern crate profont;
use profont::{PROFONT_12_POINT, PROFONT_14_POINT, PROFONT_24_POINT, PROFONT_9_POINT};

use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};

// Activate SPI, GPIO in raspi-config needs to be run with sudo because of some sysfs_gpio
// permission problems and follow-up timing problems
// see https://github.com/rust-embedded/rust-sysfs-gpio/issues/5 and follow-up issues

const ROWS: u16 = 212;
const COLS: u8 = 104;

#[rustfmt::skip]
const LUT: [u8; 70] = [
    // Phase 0     Phase 1     Phase 2     Phase 3     Phase 4     Phase 5     Phase 6
    // A B C D     A B C D     A B C D     A B C D     A B C D     A B C D     A B C D
    0b01001000, 0b10100000, 0b00010000, 0b00010000, 0b00010011, 0b00000000, 0b00000000,  // LUT0 - Black
    0b01001000, 0b10100000, 0b10000000, 0b00000000, 0b00000011, 0b00000000, 0b00000000,  // LUTT1 - White
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,  // IGNORE
    0b01001000, 0b10100101, 0b00000000, 0b10111011, 0b00000000, 0b00000000, 0b00000000,  // LUT3 - Red
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,  // LUT4 - VCOM

    // Duration            |  Repeat
    // A   B     C     D   |
    64,   12,   32,   12,    6,   // 0 Flash
    16,   8,    4,    4,     6,   // 1 clear
    4,    8,    8,    16,    16,  // 2 bring in the black
    2,    2,    2,    64,    32,  // 3 time for red
    2,    2,    2,    2,     2,   // 4 final black sharpen phase
    0,    0,    0,    0,     0,   // 5
    0,    0,    0,    0,     0    // 6
];

fn main() -> Result<(), std::io::Error> {
    // configure serial device spi
    let mut spi = Spidev::open("dev/spidev0.0").expect("SPI device");
    // create new options struct to specify options for the spi device
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(4_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    // pass options ref into the spi device
    spi.configure(&options).expect("SPI configuration");

    // configuring dgio pins (on inkyphat docs)
    let chip_select = Pin::new(8);
    chip_select.export().expect("cs export");
    while !chip_select.is_exported() {}
    chip_select.set_direction(Direction::Out).expect("CS direction");
    chip_select.set_value(1).expect("CS Value set to 1");

    let busy = Pin::new(17);
    busy.export().expect("busy export");
    while !busy.is_exported() {}
    busy.set_direction(Direction::In).expect("busy Direction");

    let data_command = Pin::new(22);
    data_command.export().expect("dc export");
    while !data_command.is_exported() {}
    data_command.set_direction(Direction::Out).expect("dc Direction");
    data_command.set_value(1).expect("dc Value set to 1");

    let reset = Pin::new(27);
    reset.export().expect("reset export");
    while !reset.is_exported() {}
    reset
        .set_direction(Direction::Out)
        .expect("reset Direction");
    reset.set_value(1).expect("reset Value set to 1");
    println!("Pins configured");

    // init display controller
    let mut delay = Delay {};
    let controller = ssd1675::Interface::new(spi, chip_select, busy, data_command, reset);
    
    let mut black_buffer = [0u8; ROWS as usize * COLS as usize / 8];
    let mut red_buffer = [0u8; ROWS as usize * COLS as usize / 8];
    let config = Builder::new()
        .dimensions(Dimensions {
            rows: ROWS,
            cols: COLS,
        })
        .rotation(Rotation::Rotate270)
        .lut(&LUT)
        .build()
        .expect("config not valid");
    let display = Display::new(controller,config);
    let mut display = (GraphicDisplay::new(display,&mut black_buffer, &mut red_buffer));

    // main display loop
    loop {
        display.reset(&mut delay).expect("display reset error");
        println!("display reset and initialised");
        let one_minute = Duration::from_secs(60);

        // clear display
        display.clear(Color::White);
        println!("Display cleared");

        // write raspi title on display
        Text::new(
            "HELLO!!!",
            Point::new(1, -4),
            MonoTextStyle::new(&PROFONT_24_POINT, Color::Red)
        )
        .draw(&mut display)
        .expect("error drawing text");
    }

    display.update(&mut delay).expect("error updating display");
    println!("Update...");

    println!("Finished - going to sleep");
    display.deep_sleep()?;

    sleep(one_minute);

}