mod inky_driver;

extern crate linux_embedded_hal;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::sysfs_gpio::Direction;
use linux_embedded_hal::Delay;
use linux_embedded_hal::{Pin, Spidev};

fn main() -> Result<(), std::io::Error> {
    // 1. SPI Setup
    let mut spi = Spidev::open("/dev/spidev0.1").expect("SPI device");
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(4_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options).expect("SPI configuration");
    // 2. GPIO Setup (Using BCM pin numbers)
    let cs = Pin::new(8);
    let busy = Pin::new(17);
    let dc = Pin::new(22);
    let reset = Pin::new(27);
    
    // We need to 'export' and set directions for the pins
    cs.export().expect("Export CS pin");
    busy.export().expect("Export BUSY pin");
    dc.export().expect("Export DC pin");
    reset.export().expect("Export RESET pin");
    cs.set_direction(Direction::Out).expect("Set CS direction");
    busy.set_direction(Direction::In).expect("Set BUSY direction");
    dc.set_direction(Direction::Out).expect("Set DC direction");
    reset.set_direction(Direction::Out).expect("Set RESET direction");
    // 3. Create our Driver
    let mut inky = inky_driver::InkyPhat::new(spi, cs, busy, dc, reset);
    let mut delay = Delay {};
    // 4. Initialization
    println!("Initializing...");
    inky.init(&mut delay).expect("Init failed");
    // 5. Create Buffers (all white for now)
    let bw_buffer = [0xFFu8; 2756];
    let red_buffer = [0x00u8; 2756];
    // 6. Draw!
    println!("Sending pixels...");
    inky.update_bw(&bw_buffer).expect("BW update failed");
    inky.update_red(&red_buffer).expect("Red update failed");
    println!("Refreshing display...");
    inky.display_refresh(&mut delay).expect("Refresh failed");
    println!("Done!");
    Ok(())
}