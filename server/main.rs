//! This shows how to read and write text via USB Serial/JTAG using embassy.
//! You need to connect via the Serial/JTAG interface to see any output.
//! Most dev-kits use a USB-UART-bridge - in that case you won't see any output.

//% CHIPS: esp32c3 esp32c6 esp32h2 esp32s3
//% FEATURES: embassy esp-hal/unstable

#![no_std]
#![no_main]

use core::str::FromStr;

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    timer::timg::TimerGroup,
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

#[embassy_executor::task]
async fn reader(
    btn: Input<'static>,
    signal: &'static Signal<NoopRawMutex, heapless::String<MAX_BUFFER_SIZE>>,
) {
    loop {
        if btn.is_low() {
            signal.signal(heapless::String::from_str("PRESSED").unwrap());
            esp_println::println!("press");
        }
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::println!("Type something to send via USB Serial JTAG to webui:");
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    let (_rx, tx) = UsbSerialJtag::new(peripherals.USB_DEVICE)
        .into_async()
        .split();

    static SIGNAL: StaticCell<Signal<NoopRawMutex, heapless::String<MAX_BUFFER_SIZE>>> =
        StaticCell::new();
    let signal = &*SIGNAL.init(Signal::new());

    let config = InputConfig::default().with_pull(Pull::Up);
    let btn = Input::new(peripherals.GPIO9, config);

    spawner.spawn(reader(btn, &signal)).unwrap();
    spawner.spawn(writer(tx, &signal)).unwrap();
}
