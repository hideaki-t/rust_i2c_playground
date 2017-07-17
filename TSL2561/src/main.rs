// read illuminance value (raw value) from TSL2561
// CC0 or WTFPL
extern crate i2cdev;

use std::path::Path;
use std::thread;
use std::time::Duration;

use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

const TSL2561_COMMAND_BIT: u8 = 0b10000000;
const TSL2561_WORD_BIT: u8 = 0b00100000;
const TSL2561_REG_CONTROL: u8 = 0x00;
const TSL2561_REG_TIMING: u8 = 0x01;
const TSL2561_REG_ID: u8 = 0x0A;
const TSL2561_REG_CHAN0_LOW: u8 = 0x0C;
const TSL2561_REG_CHAN0_HIGH: u8 = 0x0D;
const TSL2561_REG_CHAN1_LOW: u8 = 0x0E;
const TSL2561_REG_CHAN1_HIGH: u8 = 0x0F;
const TSL2561_POWER_OFF: u8 = 0x00;
const TSL2561_POWER_ON: u8 = 0b0000_0011;

enum Timing {
    IntegrationTime13,  // 0x00
    IntegraitonTime101, // 0x01
    IntegrationTime402  // 0x02
}

enum Gain {
    Gain1x = 0x00,
    Gain16x = 0x10
}

enum Channel {
    Chan0,
    Chan1
}

fn check_device<P: AsRef<Path>>(path: P, addr: u16) -> Result<u8, LinuxI2CError> {
    let mut dev = try!(LinuxI2CDevice::new(path, addr));
    let mut buf: [u8; 1] = [0];
    try!(dev.write(&[TSL2561_REG_ID]));
    try!(dev.read(&mut buf));
    println!(
        "ID: 0x{:x}, expected: 0x{:x}",
        buf[0], 0x0a
    );
    Ok(buf[0])
}

fn reg_read<P: AsRef<Path>>(path: P, addr: u16, reg: u8, buf: &mut [u8]) -> Result<(), LinuxI2CError> {
    let mut dev = try!(LinuxI2CDevice::new(path, addr));
    try!(dev.write(&[reg]));
    dev.read(buf)
}

fn reg_write<P: AsRef<Path>>(path: P, addr: u16, reg: u8, value: u8) -> Result<(), LinuxI2CError> {
    let mut dev = try!(LinuxI2CDevice::new(path, addr));
    dev.write(&[reg, value])
}

fn reg_ctrl<P: AsRef<Path>>(path: P, addr: u16, value: u8) -> Result<(), LinuxI2CError> {
    reg_write(path, addr, TSL2561_COMMAND_BIT | TSL2561_REG_CONTROL, value)
}

fn reg_timing<P: AsRef<Path>>(path: P, addr: u16, value: u8) -> Result<(), LinuxI2CError> {
    reg_write(path, addr, TSL2561_COMMAND_BIT | TSL2561_REG_TIMING, value)
}

fn poweron<P: AsRef<Path>>(path: P, addr: u16) -> Result<(), LinuxI2CError> {
    reg_ctrl(path, addr, TSL2561_POWER_ON)
}

fn poweroff<P: AsRef<Path>>(path: P, addr: u16) -> Result<(), LinuxI2CError> {
    reg_ctrl(path, addr, TSL2561_POWER_OFF)
}

fn set_integration_time_and_gain<P: AsRef<Path>>(path: P, addr: u16, time: Timing, gain: Gain) -> Result<(), LinuxI2CError> {
    reg_timing(path, addr, (time as u8| gain as u8))
}

fn read_data<P: AsRef<Path>>(path: P, addr: u16, ch: Channel, time: Timing) -> Result<u16, LinuxI2CError> {
    let x = match ch {
        Channel::Chan0 => TSL2561_REG_CHAN0_LOW,
        Channel::Chan1 => TSL2561_REG_CHAN1_LOW
    };
    let t = match time {
        Timing::IntegrationTime13 => 15,
        Timing::IntegraitonTime101 => 120,
        Timing::IntegrationTime402 => 450
    };
    let mut buf = [0; 2];

    poweroff(&path, addr).unwrap();
    poweron(&path, addr).unwrap();
    thread::sleep(Duration::from_millis(t));
    let reg = TSL2561_COMMAND_BIT | TSL2561_WORD_BIT | x;
    try!(reg_read(path, addr, reg, &mut buf));
    Ok(buf[0] as u16 | (buf[1] as u16) << 8)
}

fn main() {
    check_device(Path::new("/dev/i2c-0"), 0x39).unwrap();
    set_integration_time_and_gain(Path::new("/dev/i2c-0"), 0x39, Timing::IntegraitonTime101, Gain::Gain1x).unwrap();
    match read_data("/dev/i2c-0", 0x39, Channel::Chan0, Timing::IntegraitonTime101) {
        Ok(data) => println!("{}", data),
        Err(err) => println!("err: {:?}", err),
    };
}
