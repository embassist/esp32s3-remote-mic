#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::{Runner, StackResources};
use embassy_time::{Duration, Ticker, Timer};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_hal::{timer::systimer::SystemTimer, analog::adc::{Adc, AdcConfig, Attenuation}};
use esp_wifi::{
    init,
    wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState},
    EspWifiController,
};

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const PORT: u16 = 8080;

const SAMPLE_RATE: u32 = 8000; // 8kHz sampling
const CHUNK_SIZE: usize = 512;
const BITS_PER_SAMPLE: u16 = 16;
const NUM_CHANNELS: u16 = 1;
const BYTE_RATE: u32 = SAMPLE_RATE * NUM_CHANNELS as u32 * (BITS_PER_SAMPLE / 8) as u32;
const BLOCK_ALIGN: u16 = NUM_CHANNELS * (BITS_PER_SAMPLE / 8);
type WavBuffer = heapless::Deque<u16, CHUNK_SIZE>;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    let esp_wifi_ctrl = &*mk_static!(
        EspWifiController<'static>,
        init(timg0.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap()
    );

    let (controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();
    let wifi_interface = interfaces.sta;

    let systimer = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(systimer.alarm0);

    let config = embassy_net::Config::dhcpv4(Default::default());
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    while !stack.is_link_up() {
        Timer::after(Duration::from_millis(500)).await;
    }

    log::info!("[DHCP] Waiting...");
    while stack.config_v4().is_none() {
        Timer::after(Duration::from_millis(500)).await;
    }
    let ip = stack.config_v4().unwrap().address;
    log::info!("[IP]: {}", ip);

    let mut config = AdcConfig::new();
    let mut pin = config.enable_pin(peripherals.GPIO4, Attenuation::_11dB);
    let mut adc = Adc::new(peripherals.ADC1, config).into_async();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut buf = [0; 4096];
    loop {
        let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);
        socket.bind(PORT).unwrap();
        log::info!("[UDP] Listening...");

        let (n, ep) = socket.recv_from(&mut buf).await.unwrap();
        if let Ok(s) = core::str::from_utf8(&buf[..n]) {
            log::info!("[UDP] received {}: {}", ep, s);
        }

        let mut ticker = Ticker::every(Duration::from_micros(1_000_000 / SAMPLE_RATE as u64));
        let mut buffer: WavBuffer = heapless::Deque::new();
        let mut do_headers = true;

        loop {
            ticker.next().await;
            let value: u16 = adc.read_oneshot(&mut pin).await;
            let _ = buffer.push_back(value).unwrap();
            if buffer.is_full() {
                let chunk = into(&mut buffer, do_headers).await;
                do_headers = false;
                socket.send_to(chunk.as_slice(), ep).await.unwrap();
            }
        }
    }
}

async fn into(buffer: &mut WavBuffer, w_headers: bool) -> heapless::Vec<u8, { CHUNK_SIZE * 2 + 44 }> {
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

    // Convert ADC samples (0..4095) into signed 16-bit PCM.
    // Assumes a midpoint of 2048; adjust if necessary.
    for &sample in buffer.iter() {
        let sample_i16 = (((sample as i16) - 2048) * 32767) / 2048;
        wav.extend_from_slice(&sample_i16.to_le_bytes()).ok();
    }

    buffer.clear();
    wav
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await;
            }
            _ => {}
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            controller.start_async().await.unwrap();
        }

        log::info!("[WiFi] Connecting...");
        match controller.connect_async().await {
            Ok(_) => log::info!("[WiFi] OK"),
            Err(e) => {
                log::info!("[WiFi] Failed, since: {:?}", e);
                Timer::after(Duration::from_millis(5000)).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}