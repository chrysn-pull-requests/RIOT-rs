use ariel_os::debug::log::info;
use ariel_os::time::{Duration, Timer};
use rand_core::RngCore;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{Pixel, Point, RgbColor};

fn classify(color: Rgb888) -> bool {
    // Rather inefficient to work on this, but it's an easy way to let colors be set over the
    // network or to start from a lava simulation
    color.r() / 4 + color.g() / 4 + color.b() / 4 > 96
}

pub(crate) async fn main() -> ! {
    info!("Game of Life demo started");

    let mut rng = ariel_os::random::fast_rng();

    super::DISPLAY.lock(|display| {
        let mut display = display.borrow_mut();
        display.draw_iter(
            itertools::iproduct!(0..i32::from(super::N_COLUMNS), 0..i32::from(super::N_ROWS)).map(
                |(x, y)| {
                    let mut bytes = [0u8; 3];
                    rng.fill_bytes(&mut bytes);
                    Pixel(Point { x, y }, Rgb888::new(bytes[0], bytes[1], bytes[2]))
                },
            ),
        );
    });

    loop {
        super::DISPLAY.lock(|display| {
            let mut display = display.borrow_mut();

            // We could limit this to one row and be very clever about line-wise updates … or we
            // just don't and heave a version on the stack. It's not like any of this module
            // pretends to be efficient.
            let old: heapless::vec::Vec<_, { super::N_LEDS }> =
                itertools::iproduct!(0..i32::from(super::N_COLUMNS), 0..i32::from(super::N_ROWS))
                    .map(|(x, y)| classify(display.read_framebuffer_at(Point { x, y })))
                    .collect();

            display.draw_iter(
                itertools::iproduct!(0..i32::from(super::N_COLUMNS), 0..i32::from(super::N_ROWS))
                    .map(|(x, y)| {
                        let x_left = if x == 0 {
                            super::N_COLUMNS as i32 - 1
                        } else {
                            x - 1
                        };
                        let y_up = if y == 0 {
                            super::N_ROWS as i32 - 1
                        } else {
                            y - 1
                        };
                        let x_right = if x + 1 == super::N_COLUMNS as _ {
                            0
                        } else {
                            x + 1
                        };
                        let y_down = if y + 1 == super::N_ROWS as _ {
                            0
                        } else {
                            y + 1
                        };

                        let neightbors = [
                            (x_left, y_up),
                            (x, y_up),
                            (x_right, y_up),
                            (x_left, y),
                            (x_right, y),
                            (x_left, y_down),
                            (x, y_down),
                            (x_right, y_down),
                        ];

                        let pop: u8 = neightbors
                            .iter()
                            .map(|(x, y)| {
                                *old.get(*x as usize * super::N_ROWS as usize + *y as usize)
                                    .unwrap() as u8
                            })
                            .sum();
                        let prev = *old
                            .get(x as usize * super::N_ROWS as usize + y as usize)
                            .unwrap();

                        let new = match (prev, pop) {
                            (_, 0..2) => false,
                            (_, 4..) => false,
                            (_, 3) => true,
                            (false, 2) => false,
                            (true, 2) => true,
                        };

                        Pixel(
                            Point { x, y },
                            if new { Rgb888::WHITE } else { Rgb888::BLACK },
                        )
                    }),
            );
            display.flush();
        });

        Timer::after(Duration::from_millis(200)).await;
    }
}
