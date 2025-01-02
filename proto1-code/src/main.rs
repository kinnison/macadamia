#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::{
    rcc::{self, Sysclk},
    Config,
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
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

    loop {
        Timer::after_secs(1).await;
        info!("Hello World");
    }
}
