#![no_std]
#![no_main]

use core::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use embassy_executor::Spawner;
use embassy_net::{Ipv4Address, Runner, StackResources};
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
use smoltcp::phy::PacketMeta;
use smoltcp::socket::udp::UdpMetadata;
use smoltcp::wire::IpEndpoint;

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

const SAMPLE_RATE: u64 = 8000; // kHz
const BYTES_PER_SAMPLE: usize = 2; // 16-bit = 2 bytes
const CHUNK_SIZE: usize = 512;

type PCMBuffer = heapless::Vec<i16, CHUNK_SIZE>;
type UDPBuffer = heapless::Vec<u8, { CHUNK_SIZE * BYTES_PER_SAMPLE }>;

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
    loop {
        let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);
        socket.bind(PORT).unwrap();
        log::info!("[UDP] Broadcasting...");

        let endpoint = UdpMetadata {
            endpoint: IpEndpoint::new(Ipv4Addr::new(192, 168, 177, 97).into(), 54103),
            local_address: None,
            meta: PacketMeta::default(),
        };

        let mut ticker = Ticker::every(Duration::from_hz(SAMPLE_RATE));
        let mut packet: PCMBuffer = heapless::Vec::new();

        loop {
            ticker.next().await;
            let value: u16 = adc.read_oneshot(&mut pin).await;
            log::info!("[ADC] {}", value);
            let chunk = ((value as i32 - 2048) * 32767 / 4095) as i16;

            if packet.push(chunk).is_err() {
                let mut bytes = UDPBuffer::new();
                for sample in packet.iter() {
                    bytes.extend_from_slice(&sample.to_le_bytes())
                        .expect("Buffer size matches chunk size");
                }
                socket.send_to(bytes.as_slice(), endpoint).await.unwrap();
                // log::info!("[UDP] Sent");
                packet.clear();
            }
        }
    }
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