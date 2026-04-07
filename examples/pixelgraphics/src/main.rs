#![no_main]
#![no_std]

#[cfg(context = "ulanzi-tc001")]
const N_COLUMNS: u16 = 32;
#[cfg(context = "ulanzi-tc001")]
const N_ROWS: u16 = 8;

#[cfg(context = "waveshare-esp32-s3-matrix")]
const N_COLUMNS: u16 = 8;
#[cfg(context = "waveshare-esp32-s3-matrix")]
const N_ROWS: u16 = 8;

// Could really make this configurable; being non-square is useful to validate against axis mixusp
#[cfg(context = "native")]
const N_COLUMNS: u16 = 16;
#[cfg(context = "native")]
const N_ROWS: u16 = 8;

const N_LEDS: usize = (N_ROWS * N_COLUMNS) as usize;

mod graphicsdriver;
use graphicsdriver::{PIXELS, SIGNAL};
mod drawer;

use core::cell::RefCell;
use embassy_sync::blocking_mutex::{Mutex as BlockingMutex, raw::CriticalSectionRawMutex};

static DISPLAY: BlockingMutex<CriticalSectionRawMutex, RefCell<drawer::MyDrawTarget>> =
    BlockingMutex::new(RefCell::new(drawer::MyDrawTarget::new()));

mod coap;
mod lavalamp;
mod life;
mod scrolltext;

#[ariel_os::task(autostart)]
async fn main() {
    //lavalamp::lavalamp().await;
    //scrolltext::main("ARIEL OS").await;
    coap::main().await;
}
