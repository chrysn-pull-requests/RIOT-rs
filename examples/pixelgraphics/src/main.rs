#![no_main]
#![no_std]

const N_COLUMNS: u16 = 32;
const N_ROWS: u16 = 8;
const N_LEDS: usize = (N_ROWS * N_COLUMNS) as usize;

mod graphicsdriver;
use graphicsdriver::{PIXELS, SIGNAL};
mod drawer;

mod coap;
mod lavalamp;
mod scrolltext;

#[ariel_os::task(autostart)]
async fn main() {
    //lavalamp::lavalamp().await;
    //scrolltext::main("ARIEL OS").await;
    coap::main().await;
}
