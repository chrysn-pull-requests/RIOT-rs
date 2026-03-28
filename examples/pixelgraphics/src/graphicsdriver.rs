//! Module that handles LED value data in the arrangement in which it is sent to the LED pixels.

use ariel_os::debug::log::*;
use core::cell::Cell;

use ariel_os::hal::peripherals;

use embassy_sync::blocking_mutex::{Mutex as BlockingMutex, raw::CriticalSectionRawMutex};
use embassy_sync::signal::Signal;

use critical_section;

#[cfg(feature = "esp-hal")]
ariel_os::hal::define_peripherals!(PixelPeripherals {
    #[cfg(context = "waveshare-esp32-s3-matrix")]
    matrix: GPIO14,
    #[cfg(context = "ulanzi-tc001")]
    matrix: GPIO32,
    rmt: RMT,
});

use smart_leds::RGB8;
pub(crate) static SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub(crate) static PIXELS: BlockingMutex<CriticalSectionRawMutex, Cell<[RGB8; super::N_LEDS]>> =
    BlockingMutex::new(Cell::new([RGB8::new(0, 0, 0); super::N_LEDS]));

#[cfg(feature = "esp-hal")]
#[ariel_os::task(autostart, peripherals)]
async fn matrix_refresh_blocking_the_executor(peripherals: PixelPeripherals) {
    use esp_hal::time::Rate;

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
    let mut rmt_buffer = smart_led_buffer!(super::N_LEDS + 20);
    let mut led = SmartLedsAdapter::new(rmt.channel0, peripherals.matrix, &mut rmt_buffer);

    loop {
        SIGNAL.wait().await;
        let pixels = PIXELS.lock(|pixels| pixels.get());
        // There is an implementation of SmartLedsAdapter in esp_hal_smartled that works async'ly.
        // However, that only works when the async task gets the chance to feed the RMT peripheral
        // fast enough -- and especially when WiFi is active, that is *not* the case, and the
        // display glitches more the more traffic there is.
        //
        // The display even glitches on traffic in the blocking version, unless it is run in a
        // critical section.
        //
        // The need for the update to run in a critical section justifies a simplification on the
        // rest of the code (no need to wait for ariel_os::thread(peripherals), and just using
        // `SIGNAL.wait().await`): Running in a critical section, we're blocking the main executor,
        // but we'd just as well do that if we ran in a thread. And as there are no downsides,
        // let's pick this simpler version, and on top of it we also don't need stack space for
        // another task.
        critical_section::with(|_| {
            if let Err(e) = led.write(pixels) {
                error!("Driving LED: {:?}", e);
            }
        });
    }
}
