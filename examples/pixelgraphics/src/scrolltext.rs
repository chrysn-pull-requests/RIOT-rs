use ariel_os::{
    debug::log::info,
    thread::block_on,
    time::{Duration, Timer},
};

pub(crate) async fn main(text: &str) -> ! {
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

    // /3: crude compensation for the strong blue there
    let ariel_brick = Rgb888::new(0xd9, 0x4b, 0x26 / 3);
    let ariel_offwhite = Rgb888::new(0xee, 0xee, 0xee / 3);
    let stroke = PrimitiveStyle::with_stroke(ariel_brick, 1);

    let text_style = MonoTextStyle::new(&FONT_5X8, ariel_offwhite);

    loop {
        super::DISPLAY.lock(|display| {
            let mut display = display.borrow_mut();

            for count in 0i32..(text.len() as i32 * 5 + 32) {
                display.clear(Rgb888::BLACK).unwrap();
                display.flush();

                Rectangle::new(Point::new(0, 0), Size::new(32, 8))
                    .into_styled(stroke)
                    .draw(&mut *display)
                    .unwrap();

                Text::new(text, Point::new(32 - (count), 6), text_style)
                    .draw(&mut *display)
                    .unwrap();

                // We want descenders to show ("y") but not general text escaping to the sides

                Line::new(Point::new(0, 0), Point::new(0, 8))
                    .into_styled(stroke)
                    .draw(&mut *display)
                    .unwrap();
                Line::new(Point::new(31, 0), Point::new(31, 8))
                    .into_styled(stroke)
                    .draw(&mut *display)
                    .unwrap();

                display.flush();
            }
        });

        Timer::after(Duration::from_millis(128)).await;
    }
}
