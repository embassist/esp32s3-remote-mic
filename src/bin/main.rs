//! embassy hello world
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3
//% FEATURES: embassy esp-hal/unstable

#![no_std]
#![no_main]

use core::str::FromStr;
use core::fmt::Write;
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation},
    delay::Delay,
    timer::timg::TimerGroup,
};
use esp_println::println;

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use esp_backtrace as _;
use esp_hal::{
    usb_serial_jtag::{UsbSerialJtag, UsbSerialJtagTx},
    Async,
};
use static_cell::StaticCell;

const MAX_BUFFER_SIZE: usize = 512;

#[embassy_executor::task]
async fn writer(
    mut tx: UsbSerialJtagTx<'static, Async>,
    signal: &'static Signal<NoopRawMutex, heapless::String<MAX_BUFFER_SIZE>>,
) {
    use core::fmt::Write;
    loop {
        let message = signal.wait().await;
        signal.reset();
        write!(&mut tx, "{}", message).unwrap();
        embedded_io_async::Write::flush(&mut tx).await.unwrap();
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    let mut adc1_config = AdcConfig::new();
    let mut adcpin1 = adc1_config.enable_pin(peripherals.GPIO4, Attenuation::_6dB);
    let mut adc1 = Adc::new(peripherals.ADC1, adc1_config).into_async();

    let delay = Delay::new();

    let (_rx, tx) = UsbSerialJtag::new(peripherals.USB_DEVICE)
        .into_async()
        .split();

    static SIGNAL: StaticCell<Signal<NoopRawMutex, heapless::String<MAX_BUFFER_SIZE>>> =
        StaticCell::new();
    let signal = &*SIGNAL.init(Signal::new());

    spawner.spawn(writer(tx, &signal)).unwrap();

    loop {
        let value: u16 = adc1.read_oneshot(&mut adcpin1).await;
        println!("Value: {}", value);
        let mut s: heapless::String<512> = heapless::String::new();
        write!(s, "{}", value).unwrap();
        signal.signal(s);
        delay.delay_millis(100);
    }
}
