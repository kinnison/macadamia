#![no_std]
#![no_main]

use defmt::{debug, info};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::Pull,
    rcc::{self, Sysclk},
    Config,
};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex},
    channel::{Channel, Receiver, Sender},
};
use embassy_time::{Duration, Instant, Timer};
use macadamia_proto1::{CommsDecoder, CommsMessage, COMMS_TIMEOUT_THRESHOLD};
use {defmt_rtt as _, panic_probe as _};

static COMMS_CHANNEL: Channel<ThreadModeRawMutex, CommsMessage, 4> = Channel::new();
static COMMS_RAW_CHANNEL: Channel<CriticalSectionRawMutex, Duration, 8> = Channel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = Config::default();
    config.rcc.hsi = true;
    config.rcc.hse = None;
    config.rcc.pll = Some(rcc::Pll {
        src: rcc::PllSource::HSI,
        mul: rcc::PllMul::MUL12,
        prediv: rcc::PllPreDiv::DIV2,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    let p = embassy_stm32::init(config);

    let comms_in = ExtiInput::new(p.PB4, p.EXTI4, Pull::Down);

    spawner
        .spawn(comms_pin(comms_in, COMMS_RAW_CHANNEL.sender()))
        .unwrap();

    spawner
        .spawn(comms_decode(
            COMMS_RAW_CHANNEL.receiver(),
            COMMS_CHANNEL.sender(),
        ))
        .unwrap();

    loop {
        let msg = COMMS_CHANNEL.receive().await;
        info!(
            "Received message: src(ty={}, ad={}) dst(ty={}, ad={}) payload={}",
            msg.src_type(),
            msg.src_addr(),
            msg.dest_type(),
            msg.dest_addr(),
            msg.payload()
        )
    }
}

#[embassy_executor::task]
async fn comms_pin(
    mut pin: ExtiInput<'static>,
    sender: Sender<'static, CriticalSectionRawMutex, Duration, 8>,
) {
    let mut now = Instant::now();
    loop {
        pin.wait_for_rising_edge().await;
        let edge = Instant::now();
        sender.send(edge.saturating_duration_since(now)).await;
        now = edge;
    }
}

#[embassy_executor::task]
async fn comms_decode(
    receiver: Receiver<'static, CriticalSectionRawMutex, Duration, 8>,
    sender: Sender<'static, ThreadModeRawMutex, CommsMessage, 4>,
) {
    let give_up: Duration = COMMS_TIMEOUT_THRESHOLD + Duration::from_micros(250);
    loop {
        // Wait for any activity
        receiver.receive().await;
        let mut decoder = CommsDecoder::new();
        let mut bits = 1;
        loop {
            let time_taken = match select(receiver.receive(), Timer::after(give_up)).await {
                Either::First(t) => t,
                Either::Second(_) => give_up,
            };
            if let Some(message) = decoder.consume_delta(time_taken) {
                sender.send(message).await;
                break;
            }
            if time_taken >= give_up {
                debug!(
                    "Timed out and did not receive a message, going to idle after {} bit(s)",
                    bits
                );
                debug!("Timings from message were: {:?}", decoder.timings());
                break;
            }
            bits += 1;
        }
    }
}
