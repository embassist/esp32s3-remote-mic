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
use embassy_time::{Duration, Ticker};
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

const SAMPLE_RATE: u32 = 8000; // 8kHz sampling
const CHUNK_SIZE: usize = 512;
const BITS_PER_SAMPLE: u16 = 16;
const NUM_CHANNELS: u16 = 1;
const BYTE_RATE: u32 = SAMPLE_RATE * NUM_CHANNELS as u32 * (BITS_PER_SAMPLE / 8) as u32;
const BLOCK_ALIGN: u16 = NUM_CHANNELS * (BITS_PER_SAMPLE / 8);
type WavBuffer = heapless::Deque<u16, CHUNK_SIZE>;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    let mut config = AdcConfig::new();
    let mut pin = config.enable_pin(peripherals.GPIO4, Attenuation::_6dB);
    let mut adc = Adc::new(peripherals.ADC1, config).into_async();

    let (_rx, mut tx) = UsbSerialJtag::new(peripherals.USB_DEVICE)
        .into_async()
        .split();

    let mut ticker = Ticker::every(Duration::from_micros(1_000_000 / SAMPLE_RATE as u64));
    let mut buffer: WavBuffer = heapless::Deque::new();
    let mut do_headers = true;
    loop {
        ticker.next().await;

        let value: u16 = adc.read_oneshot(&mut pin).await;
        let _ = buffer.push_back(value).unwrap();

        if buffer.is_full() {
            let chunk = into_wav(&mut buffer, do_headers).await;
            do_headers = false;

            for chunk in chunk.chunks(64) {
                if tx.write(chunk).is_err() {
                    panic!("shit");
                }
                embedded_io_async::Write::flush(&mut tx).await.unwrap();
            }
            // write!(&mut tx, "SENT CHUNK").unwrap();
        }
    }
}

async fn into_wav(buffer: &mut WavBuffer, w_headers: bool) -> heapless::Vec<u8, { CHUNK_SIZE * 2 + 44 }> {
    let mut wav: heapless::Vec<u8, { CHUNK_SIZE * 2 + 44 }> = heapless::Vec::new();
    let data_size = buffer.len() as u32 * 2;
    let file_size = data_size + 36;

    if w_headers {
        wav.extend_from_slice(b"RIFF").ok();
        wav.extend_from_slice(&file_size.to_le_bytes()).ok();
        wav.extend_from_slice(b"WAVEfmt ").ok();
        wav.extend_from_slice(&[16, 0, 0, 0]).ok();
        wav.extend_from_slice(&[1, 0]).ok();
        wav.extend_from_slice(&NUM_CHANNELS.to_le_bytes()).ok();
        wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes()).ok();
        wav.extend_from_slice(&BYTE_RATE.to_le_bytes()).ok();
        wav.extend_from_slice(&BLOCK_ALIGN.to_le_bytes()).ok();
        wav.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes()).ok();
        wav.extend_from_slice(b"data").ok();
        wav.extend_from_slice(&data_size.to_le_bytes()).ok();
    }

    for &sample in buffer.iter() {
        let sample_16bit = (sample as u16) << 4;
        wav.extend_from_slice(&sample_16bit.to_le_bytes()).ok();
    }

    buffer.clear();
    wav
}
