//! This example runs on the STM32 LoRa Discovery board, which has a builtin Semtech Sx1276 radio.
//! It demonstrates LORA P2P CAD functionality.
#![no_std]
#![no_main]
#![macro_use]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_lora::iv::Stm32l0InterfaceVariant;
use embassy_stm32::exti::{Channel, ExtiInput};
use embassy_stm32::gpio::{Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::spi;
use embassy_stm32::time::khz;
use embassy_time::{Delay, Timer};
use lora_phy::mod_params::*;
use lora_phy::sx1276_7_8_9::SX1276_7_8_9;
use lora_phy::LoRa;
use {defmt_rtt as _, panic_probe as _};

const LORA_FREQUENCY_IN_HZ: u32 = 903_900_000; // warning: set this appropriately for the region

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    config.rcc.mux = embassy_stm32::rcc::ClockSrc::HSI;
    config.rcc.enable_hsi48 = true;
    let p = embassy_stm32::init(config);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = khz(200);

    // SPI for sx1276
    let spi = spi::Spi::new(p.SPI1, p.PB3, p.PA7, p.PA6, p.DMA1_CH3, p.DMA1_CH2, spi_config);

    let nss = Output::new(p.PA15.degrade(), Level::High, Speed::Low);
    let reset = Output::new(p.PC0.degrade(), Level::High, Speed::Low);

    let irq_pin = Input::new(p.PB4.degrade(), Pull::Up);
    let irq = ExtiInput::new(irq_pin, p.EXTI4.degrade());

    let iv = Stm32l0InterfaceVariant::new(nss, reset, irq, None, None).unwrap();

    let mut lora = {
        match LoRa::new(SX1276_7_8_9::new(BoardType::Stm32l0Sx1276, spi, iv), false, Delay).await {
            Ok(l) => l,
            Err(err) => {
                info!("Radio error = {}", err);
                return;
            }
        }
    };

    let mut debug_indicator = Output::new(p.PB5, Level::Low, Speed::Low);
    let mut start_indicator = Output::new(p.PB6, Level::Low, Speed::Low);

    start_indicator.set_high();
    Timer::after_secs(5).await;
    start_indicator.set_low();

    let mdltn_params = {
        match lora.create_modulation_params(
            SpreadingFactor::_10,
            Bandwidth::_250KHz,
            CodingRate::_4_8,
            LORA_FREQUENCY_IN_HZ,
        ) {
            Ok(mp) => mp,
            Err(err) => {
                info!("Radio error = {}", err);
                return;
            }
        }
    };

    match lora.prepare_for_cad(&mdltn_params, true).await {
        Ok(()) => {}
        Err(err) => {
            info!("Radio error = {}", err);
            return;
        }
    };

    match lora.cad().await {
        Ok(cad_activity_detected) => {
            if cad_activity_detected {
                info!("cad successful with activity detected")
            } else {
                info!("cad successful without activity detected")
            }
            debug_indicator.set_high();
            Timer::after_secs(5).await;
            debug_indicator.set_low();
        }
        Err(err) => info!("cad unsuccessful = {}", err),
    }
}
