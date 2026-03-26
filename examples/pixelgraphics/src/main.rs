#![no_main]
#![no_std]

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
