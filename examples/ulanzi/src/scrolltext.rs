use ariel_os::{
    debug::log::info,
    thread::block_on,
    time::{Duration, Timer},
};

use super::drawer::MyDrawTarget;

#[ariel_os::task(autostart)]
async fn main() {
    info!("scrolltext thread started");

    use embedded_graphics::{
        mono_font::{MonoTextStyle, ascii::FONT_5X8},
        pixelcolor::Rgb888,
        prelude::*,
        primitives::{
            Circle, CornerRadii, Ellipse, Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle,
            RoundedRectangle, Triangle,
        },
        text::Text,
    };

    let mut display = MyDrawTarget::new();

    let stroke = PrimitiveStyle::with_stroke(Rgb888::RED, 1);

    let text_style = MonoTextStyle::new(&FONT_5X8, Rgb888::YELLOW);

    let text = "ARIEL OS";

    loop {
        for count in 0i32..(text.len() as i32 * 5 + 32) {
            display.clear(Rgb888::BLACK).unwrap();
            display.flush();

            Rectangle::new(Point::new(0, 0), Size::new(32, 8))
                .into_styled(stroke)
                .draw(&mut display)
                .unwrap();

            Text::new(text, Point::new(32 - (count), 6), text_style)
                .draw(&mut display)
                .unwrap();

            // We want descenders to show ("y") but not general text escaping to the sides

            Line::new(Point::new(0, 0), Point::new(0, 8))
                .into_styled(stroke)
                .draw(&mut display)
                .unwrap();
            Line::new(Point::new(31, 0), Point::new(31, 8))
                .into_styled(stroke)
                .draw(&mut display)
                .unwrap();

            display.flush();

            Timer::after(Duration::from_millis(128)).await;
        }
    }
}
