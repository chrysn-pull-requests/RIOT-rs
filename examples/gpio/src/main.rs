#![no_main]
#![no_std]

use ariel_os_boards::pins;

use ariel_os::{
    gpio::{Input, Level, Output, Pull},
    time::Timer,
};

use ariel_os::debug::log::*;

ariel_os::hal::group_peripherals!(Peripherals {
    leds: pins::LedPeripherals,
    buttons: pins::ButtonPeripherals,
});

#[ariel_os::task(autostart, peripherals)]
async fn blinky(peripherals: Peripherals) {
    info!("Starting up");
    let mut led0 = Output::new(peripherals.leds.led0, Level::Low);

    info!("LED is {}", core::any::type_name_of_val(&led0));

    #[allow(unused_variables)]
    let pull = Pull::Up;
    #[cfg(context = "st-nucleo-h755zi-q")]
    let pull = Pull::None;

    let mut btn0 = Input::builder(peripherals.buttons.button0, pull)
        .build_with_interrupt()
        .unwrap();

    info!("Button is {}", core::any::type_name_of_val(&btn0));

    loop {
        // Wait for the button being pressed or 300 ms, whichever comes first.
        let _ =
            embassy_futures::select::select(btn0.wait_for_low(), Timer::after_millis(300)).await;

        led0.toggle();
        Timer::after_millis(100).await;
    }
}
