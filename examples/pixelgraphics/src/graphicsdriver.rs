use ariel_os::debug::log::*;
use core::cell::Cell;

use ariel_os::hal::peripherals;

use embassy_sync::blocking_mutex::{Mutex as BlockingMutex, raw::CriticalSectionRawMutex};
use embassy_sync::signal::Signal;
use esp_hal::time::Rate;

use critical_section;

ariel_os::hal::define_peripherals!(PixelPeripherals {
    #[cfg(context = "waveshare-esp32-s3-matrix")]
    matrix: GPIO14,
    #[cfg(context = "ulanzi-tc001")]
    matrix: GPIO32,
    rmt: RMT,
});

use smart_leds::RGB8;
const N_LEDS: usize = 32 * 8;
pub(crate) static SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub(crate) static PIXELS: BlockingMutex<CriticalSectionRawMutex, Cell<[RGB8; N_LEDS]>> =
    BlockingMutex::new(Cell::new([RGB8::new(0, 0, 0); N_LEDS]));

#[cfg(false)]
#[ariel_os::task(autostart, peripherals)]
async fn matrix_refresh_async(peripherals: PixelPeripherals) {
    info!("matrix refresh started");

    use esp_hal::rmt::Rmt;
    use esp_hal_smartled::{SmartLedsAdapterAsync, smart_led_buffer};
    use smart_leds_trait::SmartLedsWriteAsync;

    cfg_if::cfg_if! {
        if #[cfg(feature = "esp32h2")] {
            let freq = Rate::from_mhz(32);
        } else {
            let freq = Rate::from_mhz(80);
        }
    };

    let rmt = Rmt::new(peripherals.rmt, freq).unwrap().into_async();
    let mut rmt_buffer = smart_led_buffer!(N_LEDS + 20);
    let mut led = SmartLedsAdapterAsync::new(rmt.channel0, peripherals.matrix, &mut rmt_buffer);

    loop {
        SIGNAL.wait().await;
        let pixels = PIXELS.lock(|pixels| pixels.get());
        if let Err(e) = led.write(pixels).await {
            error!("Driving LED: {:?}", e);
        }
    }
}

#[ariel_os::task(autostart, peripherals)]
async fn matrix_refresh_blocking_the_executor(peripherals: PixelPeripherals) {
    info!("matrix refresh started");

    use esp_hal::rmt::Rmt;
    use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
    use smart_leds_trait::SmartLedsWrite;

    cfg_if::cfg_if! {
        if #[cfg(feature = "esp32h2")] {
            let freq = Rate::from_mhz(32);
        } else {
            let freq = Rate::from_mhz(80);
        }
    };

    let rmt = Rmt::new(peripherals.rmt, freq).unwrap();
    let mut rmt_buffer = smart_led_buffer!(N_LEDS + 20);
    let mut led = SmartLedsAdapter::new(rmt.channel0, peripherals.matrix, &mut rmt_buffer);

    loop {
        SIGNAL.wait().await;
        let pixels = PIXELS.lock(|pixels| pixels.get());
        // Delibertely not awaiting: We *need* to do this continuously
        critical_section::with(|_| {
            if let Err(e) = led.write(pixels) {
                error!("Driving LED: {:?}", e);
            }
        });
    }
}
