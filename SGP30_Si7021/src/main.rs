// read air quality from SGP30 using temp&RH from Si7021
// CC0 or WTFPL
extern crate i2cdev;

use std::env;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

fn dump_si7021_info(dev: &mut LinuxI2CDevice) -> Result<u8, LinuxI2CError> {
    let mut buf: [u8; 1] = [0];

    try!(dev.write(&[SI7021_CMD_READ_USER_REG]));
    try!(dev.read(&mut buf));
    println!(
        "User register: 0x{:x}, default: 0x{:x}",
        buf[0], SI7021_USER_REG_DEFAULT
    );
    println!(
        "resolution: {}{}, VDDS: {}, heater: {}",
        buf[0] & 0b1000_0000,
        buf[0] & 0b0000_0001,
        if buf[0] & 0b0100_0000 == 0 {
            "Ok"
        } else {
            "Low"
        },
        if buf[0] & 0b0000_0100 == 1 {
            "On"
        } else {
            "Off"
        }
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

fn calc_temp(buf: [u8; 3]) -> f32 {
    let temp_raw = (buf[0] as u16) << 8 | buf[1] as u16;
    175.72 * (temp_raw as f32) / 65536.0 - 46.85
}

fn calc_rh(buf: [u8; 3]) -> f32 {
    let rh_raw = (buf[0] as u16) << 8 | buf[1] as u16;
    125.0 * (rh_raw as f32) / 65536.0 - 6.0
}

fn calc_ah(t: f32, rh: f32) -> f32 {
    216.7 * (rh / 100.0 * 6.112 * ((17.62 * t) / (243.12 + t)).exp()) / (273.15 + t)
}

fn read_si7021(dev: &mut LinuxI2CDevice) -> Result<(f32, f32, f32), LinuxI2CError> {
    let mut buf: [u8; 3] = [0; 3];
    try!(dev.write(&[SI7021_CMD_MEASURE_RH_NO_HOLD]));
    thread::sleep(Duration::from_millis(25));
    try!(dev.read(&mut buf));
    let rh = calc_rh(buf);

    try!(dev.write(&[SI7021_CMD_MEASURE_TEMP_AFTER_RH]));
    thread::sleep(Duration::from_millis(25));
    try!(dev.read(&mut buf));
    let temp = calc_temp(buf);
    let ah = calc_ah(temp, rh);
    Ok((temp, rh, ah))
}

fn dump_sgp30_info(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
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

    let mut buf: [u8; 9] = [0; 9];
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

fn test_sgp30(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
    let mut buf: [u8; 3] = [0; 3];
    try!(dev.write(&SGP30_CMD_MEASURE_TEST));
    thread::sleep(Duration::from_millis(220));
    try!(dev.read(&mut buf));
    let s: u16 = buf[1] as u16 | (buf[0] as u16) << 8;
    println!("Test results: 0x{:x?}, Ok: {}", s, s == SGP30_TEST_EXPECTED);
    Ok(())
}

fn init_sgp30(dev: &mut LinuxI2CDevice) -> Result<(), LinuxI2CError> {
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
    let mut sgp30 = LinuxI2CDevice::new(&dev, SGP30_SLAVE_ADDRESS).unwrap();
    let mut si7021 = LinuxI2CDevice::new(&dev, SI7021_SLAVE_ADDRESS).unwrap();
    dump_sgp30_info(&mut sgp30).unwrap();
    dump_si7021_info(&mut si7021).unwrap();
    test_sgp30(&mut sgp30).unwrap();
    init_sgp30(&mut sgp30).unwrap();
    let mut timer = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    loop {
        timer += Duration::from_millis(1000);
        measure(&mut sgp30).unwrap();
        get_baseline(&mut sgp30).unwrap();
        read_si7021(&mut si7021).unwrap();
        let s = timer - SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!("next: {:?}, sleep: {:?}", timer, s);
        thread::sleep(s);
    }
}
