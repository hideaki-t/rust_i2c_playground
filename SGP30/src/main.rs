// read air quality from SGP30
// CC0 or WTFPL
extern crate i2cdev;

use std::env;
use std::thread;
use std::time::Duration;

use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

const SGP30_SLAVE_ADDRESS: u16 = 0x58;
const SGP30_CMD_INIT_AIR_QUALITY: [u8; 2] = [0x20, 0x03];
const SGP30_CMD_MEASURE_AIR_QUALITY: [u8; 2] = [0x20, 0x08];
const SGP30_CMD_GET_BASELINE: [u8; 2] = [0x20, 0x15];
const SGP30_CMD_SET_BASELINE: u16 = 0x201e;
const SGP30_CMD_SET_HUMIDITY: u16 = 0x2061;
const SGP30_CMD_MEASURE_TEST: [u8; 2] = [0x20, 0x32];
const SGP30_CMD_GET_FEATURE_SET_VERSION: [u8; 2] = [0x20, 0x2f];
const SGP30_CMD_GET_SERIAL_ID: [u8; 2] = [0x36, 0x82];
const SGP30_CMD_MEASURE_RAW_SIGNAL: u16 = 0x2050;
const SGP30_FS_PROD_MASK: u8 = 0b1111_0000;
const SGP30_FS_PROD_SGP30: u8 = 0;
const SGP30_TEST_EXPECTED: u16 = 0xd400;

fn dump_info(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    let mut buf: [u8; 3] = [0; 3];
    try!(dev.write(&SGP30_CMD_GET_FEATURE_SET_VERSION));
    thread::sleep(Duration::from_millis(10));
    try!(dev.read(&mut buf));
    println!(
        "Raw Feature Set: 0x{:x?}, SGP30: {}, MSB(8):{} , version: 0x{:x}, CRC: 0x{:x}",
        buf,
        buf[0] >> 4 == SGP30_FS_PROD_SGP30,
        buf[0] & 0xfe,
        buf[1],
        buf[2]
    );

    let mut buf: [u8; 9] = [0; 9]; // 3*3 = 3 words(2byte), each word has 8bit CRC
    try!(dev.write(&SGP30_CMD_GET_SERIAL_ID));
    thread::sleep(Duration::from_millis(1));
    try!(dev.read(&mut buf));
    let s: u64 = buf[7] as u64
        | (buf[6] as u64) << 8
        | (buf[4] as u64) << 16
        | (buf[3] as u64) << 24
        | (buf[1] as u64) << 32
        | (buf[0] as u64) << 40;
    println!("Raw Serial ID: 0x{:x?}, Serial ID: 0x{:x}", buf, s);
    Ok(())
}

fn test(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    let mut buf: [u8; 3] = [0; 3];
    try!(dev.write(&SGP30_CMD_MEASURE_TEST));
    thread::sleep(Duration::from_millis(220));
    try!(dev.read(&mut buf));
    let s: u16 = buf[1] as u16 | (buf[0] as u16) << 8;
    println!("Test results: 0x{:x?}, Ok: {}", s, s == SGP30_TEST_EXPECTED);
    Ok(())
}

fn init(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    try!(dev.write(&SGP30_CMD_INIT_AIR_QUALITY));
    thread::sleep(Duration::from_millis(10));
    Ok(())
}

fn measure(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    let mut buf: [u8; 6] = [0; 6];
    try!(dev.write(&SGP30_CMD_MEASURE_AIR_QUALITY));
    thread::sleep(Duration::from_millis(12));
    try!(dev.read(&mut buf));
    let eco2: u16 = (buf[0] as u16) << 8 | buf[1] as u16;
    let tvoc: u16 = (buf[3] as u16) << 8 | buf[4] as u16;
    println!("TVOC: {}, eCO2: {}, raw: {:x?}", tvoc, eco2, buf);
    Ok(())
}

fn get_baseline(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    let mut buf: [u8; 6] = [0; 6];
    try!(dev.write(&SGP30_CMD_GET_BASELINE));
    thread::sleep(Duration::from_millis(10));
    try!(dev.read(&mut buf));
    let eco2: u16 = (buf[0] as u16) << 8 | buf[1] as u16;
    let tvoc: u16 = (buf[3] as u16) << 8 | buf[4] as u16;
    println!(
        "Baseline TVOC: 0x{:x}, eCO2: 0x{:x}, raw: {:x?}",
        tvoc, eco2, buf
    );
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dev = format!("/dev/i2c-{}", args[1]);
    let mut dev = LinuxI2CDevice::new(dev, SGP30_SLAVE_ADDRESS).unwrap();
    dump_info(&mut dev).unwrap();
    test(&mut dev).unwrap();
    init(&mut dev).unwrap();
    loop {
        measure(&mut dev).unwrap();
        get_baseline(&mut dev).unwrap();
        thread::sleep(Duration::from_millis(1000 - 12 - 10));
    }
}
