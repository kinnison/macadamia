#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::{
    rcc::{self, Sysclk},
    Config,
};
use embassy_time::{Duration, Instant, Timer};
use {defmt_rtt as _, panic_probe as _};

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
    let _p = embassy_stm32::init(config);

    spawner.spawn(latency_measure()).unwrap();

    loop {
        Timer::after_secs(5).await;
        info!("Five Seconds");
    }
}

#[embassy_executor::task]
async fn latency_measure() {
    loop {
        let start = Instant::now();
        let mut next = start + Duration::from_millis(1);
        let mut total_gap = Duration::from_secs(0);
        for _ in 0..10_000 {
            Timer::at(next).await;
            let gap = Instant::now().saturating_duration_since(next);
            total_gap += gap;
            next += Duration::from_millis(1);
        }
        let elapsed = Instant::now().saturating_duration_since(start);
        info!(
            "Elapsed: {}µs, total gap: {}µs ({}µs/ms)",
            elapsed.as_micros(),
            total_gap.as_micros(),
            total_gap.as_micros() / 10_000,
        );
    }
}
