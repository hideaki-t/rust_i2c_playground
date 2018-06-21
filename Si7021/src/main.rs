// read temp(in C) from Si7021
// CC0 or WTFPL
extern crate i2cdev;

use std::thread;
use std::time::Duration;

use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

const SI7021_SLAVE_ADDRESS: u16 = 0x40;
const SI7021_CMD_MEASURE_TEMP_NO_HOLD: u8 = 0xF3;
const SI7021_CMD_MEASURE_RH_NO_HOLD: u8 = 0xF5;
const SI7021_CMD_MEASURE_TEMP_AFTER_RH: u8 = 0xE0;
const SI7021_CMD_RESET: u8 = 0xFE;
const SI7021_CMD_READ_USER_REG: u8 = 0xE7;
const SI7021_CMD_READ_FIRMWARE_REV: u16 = 0x84B8;
const SI7021_CMD_READ_ID1: u16 = 0xFA0F;
const SI7021_CMD_READ_ID2: u16 = 0xFCC9;
const SI7021_USER_REG_DEFAULT: u8 = 0b0011_1010;
const SI7021_FIRMWARE_REV_20: u8 = 0x20;
const SI7021_SNB3: u8 = 0x15;
const SI7020_SNB3: u8 = 0x14;

fn dump_info(dev: &mut LinuxI2CDevice) -> Result<u8, LinuxI2CError> {
    let mut buf: [u8; 1] = [0];

    try!(dev.write(&[SI7021_CMD_READ_USER_REG]));
    try!(dev.read(&mut buf));
    println!(
        "User register: 0x{:x}, default: 0x{:x}",
        buf[0], SI7021_USER_REG_DEFAULT
    );
    println!(
        "resolution: {}{}, VDDS: {}, heater: {}", 
                buf[0] & 0b1000_0000, buf[0] & 0b0000_0001, 
                if buf[0] & 0b0100_0000 == 0 { "Ok" } else { "Low" },
                if buf[0] & 0b0000_0100 == 1 { "On" } else { "Off" }
    );

    thread::sleep(Duration::from_millis(25));
    try!(dev.write(&[
        (SI7021_CMD_READ_FIRMWARE_REV >> 8) as u8,
        (SI7021_CMD_READ_FIRMWARE_REV & 0xFF) as u8,
    ]));
    try!(dev.read(&mut buf));
    println!(
        "firmware: 0x{:x}, rev2: {}",
        buf[0],
        buf[0] == SI7021_FIRMWARE_REV_20
    );

    let mut buf: [u8; 4] = [0; 4];
    thread::sleep(Duration::from_millis(25));
    try!(dev.write(&[
        (SI7021_CMD_READ_ID1 >> 8) as u8,
        (SI7021_CMD_READ_ID1 & 0xFF) as u8,
    ]));
    try!(dev.read(&mut buf));
    let ida: u32 =
        (buf[0] as u32) << 24 | (buf[1] as u32) << 16 | (buf[2] as u32) << 8 | buf[3] as u32;

    thread::sleep(Duration::from_millis(25));
    try!(dev.write(&[
        (SI7021_CMD_READ_ID2 >> 8) as u8,
        (SI7021_CMD_READ_ID2 & 0xFF) as u8,
    ]));
    try!(dev.read(&mut buf));
    let snb3 = buf[0];
    let idb: u32 =
        (snb3 as u32) << 24 | (buf[1] as u32) << 16 | (buf[2] as u32) << 8 | buf[3] as u32;
    println!(
        "IDa: 0x{:x}, IDb: 0x{:x}, SNB3: 0x{:x}(Si7020: {}, Si7021: {})",
        ida,
        idb,
        snb3,
        snb3 == SI7020_SNB3,
        snb3 == SI7021_SNB3
    );
    Ok(buf[0])
}

// after reset, the device will be invisible from host.
// i2detect cannot find it neither
// rebooting host can solve it.
fn reset(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    try!(dev.write(&[SI7021_CMD_RESET]));
    thread::sleep(Duration::from_millis(100));
    Ok(())
}

fn calc_temp(buf: [u8; 3]) -> f32 {
    let temp_raw = (buf[0] as u16) << 8 | buf[1] as u16;
    175.72 * (temp_raw as f32) / 65536.0 - 46.85
}

fn calc_rh(buf: [u8; 3]) -> f32 {
    let rh_raw = (buf[0] as u16) << 8 | buf[1] as u16;
    125.0 * (rh_raw as f32) / 65536.0 - 6.0
}

fn read_temp(dev: &mut LinuxI2CDevice) -> Result<f32, LinuxI2CError> {
    try!(dev.write(&[SI7021_CMD_MEASURE_TEMP_NO_HOLD]));
    thread::sleep(Duration::from_millis(25));
    let mut buf: [u8; 3] = [0; 3];
    try!(dev.read(&mut buf));
    Ok(calc_temp(buf))
}

fn read_rel_humidity(dev: &mut LinuxI2CDevice) -> Result<f32, LinuxI2CError> {
    try!(dev.write(&[SI7021_CMD_MEASURE_RH_NO_HOLD]));
    thread::sleep(Duration::from_millis(25));
    let mut buf: [u8; 3] = [0; 3];
    try!(dev.read(&mut buf));
    Ok(calc_rh(buf))
}

fn read_rel_humidity_and_temp(dev: &mut LinuxI2CDevice) -> Result<(f32, f32), LinuxI2CError> {
    let rh = read_rel_humidity(dev);
    try!(dev.write(&[SI7021_CMD_MEASURE_TEMP_AFTER_RH]));
    thread::sleep(Duration::from_millis(25));
    let mut buf: [u8; 3] = [0; 3];
    try!(dev.read(&mut buf));
    calc_temp(buf);
    Ok((calc_temp(buf), rh.unwrap()))
}

fn main() {
    let mut dev = LinuxI2CDevice::new("/dev/i2c-0", SI7021_SLAVE_ADDRESS).unwrap();
    dump_info(&mut dev).unwrap();
    thread::sleep(Duration::from_millis(25));
    match read_rel_humidity_and_temp(&mut dev) {
        Ok((t, h)) => println!("temp: {}, relative humidity: {}", t, h),
        Err(err) => println!("err: {:?}", err),
    };
}
