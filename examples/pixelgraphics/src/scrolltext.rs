use ariel_os::{
    debug::log::info,
    thread::block_on,
    time::{Duration, Timer},
};

pub(crate) async fn main(text: &str) -> ! {
    info!("scrolltext thread started");

    use embedded_graphics::{
        mono_font::{MonoTextStyle, ascii},
        pixelcolor::Rgb888,
        prelude::*,
        primitives::{
            Circle, CornerRadii, Ellipse, Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle,
            RoundedRectangle, Triangle,
        },
        text::Text,
    };

    let (font, char_width) = const {
        match super::N_ROWS {
            8 => (ascii::FONT_5X8, 5),
            10 => (ascii::FONT_6X10, 6),
            _ => panic!("What is a suitable font size for that height?"),
        }
    };

    // /3: crude compensation for the strong blue there -- and we really need colors per device
    // until we get some brightness and curve adjustment
    #[cfg(not(context = "waveshare-esp32-s3-matrix"))]
    let ariel_brick = Rgb888::new(0xd9, 0x4b, 0x26 / 3);
    #[cfg(not(context = "waveshare-esp32-s3-matrix"))]
    let ariel_offwhite = Rgb888::new(0xee, 0xee, 0xee / 3);
    // FIXME: seems a bit those are GRB rather than RGB?
    #[cfg(context = "waveshare-esp32-s3-matrix")]
    let ariel_brick = Rgb888::new(0x02, 0x07, 0x00);
    #[cfg(context = "waveshare-esp32-s3-matrix")]
    let ariel_offwhite = Rgb888::new(0x08, 0x0c, 0x02);
    let stroke = PrimitiveStyle::with_stroke(ariel_brick, 1);

    let text_style = MonoTextStyle::new(&font, ariel_offwhite);

    const N_COLUMNS: i32 = super::N_COLUMNS as _;
    const N_ROWS: i32 = super::N_ROWS as _;

    loop {
        for count in 0i32..(text.len() as i32 * char_width + N_COLUMNS) {
            super::DISPLAY.lock(|display| {
                let mut display = display.borrow_mut();
                let display = &mut *display;

                display.clear(Rgb888::BLACK).unwrap();
                display.flush();

                Rectangle::new(Point::new(0, 0), Size::new(N_COLUMNS as _, N_ROWS as _))
                    .into_styled(stroke)
                    .draw(display)
                    .unwrap();

                Text::new(
                    text,
                    Point::new(N_COLUMNS - (count), N_ROWS - 2),
                    text_style,
                )
                .draw(display)
                .unwrap();

                // We want descenders to show ("y") but not general text escaping to the sides

                Line::new(Point::new(0, 0), Point::new(0, N_ROWS - 1))
                    .into_styled(stroke)
                    .draw(display)
                    .unwrap();
                Line::new(
                    Point::new(N_COLUMNS - 1, 0),
                    Point::new(N_COLUMNS - 1, N_ROWS - 1),
                )
                .into_styled(stroke)
                .draw(display)
                .unwrap();

                display.flush();
            });
            Timer::after(Duration::from_millis(128)).await;
        }
    }
}
