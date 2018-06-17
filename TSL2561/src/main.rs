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
    IntegrationTime101, // 0x01
    IntegrationTime402, // 0x02
}

enum Gain {
    Gain1x = 0x00,
    Gain16x = 0x10,
}

enum Channel {
    Chan0,
    Chan1,
}

fn check_device(dev: &mut LinuxI2CDevice) -> Result<u8, LinuxI2CError> {
    let mut buf: [u8; 1] = [0];
    reg_read(dev, TSL2561_COMMAND_BIT | TSL2561_REG_ID, &mut buf).unwrap();
    let partno = (buf[0] >> 4) & 0x0f;
    let rev = buf[0] & 0x0f;
    println!(
        "ID: 0x{:x}, partno: {}({}), rev: 0x{:x}",
        buf[0],
        partno,
        if partno == 0b0101 {
            "TSL2561"
        } else {
            "TSL2560"
        },
        rev
    );
    Ok(buf[0])
}

fn reg_read(dev: &mut LinuxI2CDevice, reg: u8, buf: &mut [u8]) -> Result<(), LinuxI2CError> {
    try!(dev.write(&[reg]));
    dev.read(buf)
}

fn reg_write(dev: &mut LinuxI2CDevice, reg: u8, value: u8) -> Result<(), LinuxI2CError> {
    dev.write(&[reg, value])
}

fn reg_ctrl(dev: &mut LinuxI2CDevice, value: u8) -> Result<(), LinuxI2CError> {
    reg_write(dev, TSL2561_COMMAND_BIT | TSL2561_REG_CONTROL, value)
}

fn reg_timing(dev: &mut LinuxI2CDevice, value: u8) -> Result<(), LinuxI2CError> {
    reg_write(dev, TSL2561_COMMAND_BIT | TSL2561_REG_TIMING, value)
}

fn poweron(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    reg_ctrl(dev, TSL2561_POWER_ON)
}

fn poweroff(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    reg_ctrl(dev, TSL2561_POWER_OFF)
}

fn set_integration_time_and_gain(
    dev: &mut LinuxI2CDevice,
    time: Timing,
    gain: Gain,
) -> Result<(), LinuxI2CError> {
    reg_timing(dev, (time as u8 | gain as u8))
}

fn read_data(dev: &mut LinuxI2CDevice, ch: Channel, time: Timing) -> Result<u16, LinuxI2CError> {
    let x = match ch {
        Channel::Chan0 => TSL2561_REG_CHAN0_LOW,
        Channel::Chan1 => TSL2561_REG_CHAN1_LOW,
    };
    let t = match time {
        Timing::IntegrationTime13 => 15,
        Timing::IntegrationTime101 => 120,
        Timing::IntegrationTime402 => 450,
    };
    let mut buf = [0; 2];

    poweron(dev).unwrap();
    thread::sleep(Duration::from_millis(t));
    let reg = TSL2561_COMMAND_BIT | TSL2561_WORD_BIT | x;
    try!(reg_read(dev, reg, &mut buf));
    Ok(buf[0] as u16 | (buf[1] as u16) << 8)
}

fn main() {
    let mut dev = LinuxI2CDevice::new("/dev/i2c-0", 0x39).unwrap();
    check_device(&mut dev).unwrap();
    set_integration_time_and_gain(&mut dev, Timing::IntegrationTime101, Gain::Gain1x).unwrap();
    match read_data(&mut dev, Channel::Chan0, Timing::IntegrationTime101) {
        Ok(data) => println!("{}", data),
        Err(err) => println!("err: {:?}", err),
    };
    poweroff(&mut dev).unwrap();
}
