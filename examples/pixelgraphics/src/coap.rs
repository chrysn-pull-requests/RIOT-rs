use ariel_os::{
    debug::log::info,
    time::{Duration, Timer},
};

use super::drawer::MyDrawTarget;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    pixelcolor::{Rgb888, RgbColor as _},
    prelude::Point,
};

#[derive(Clone)]
enum Mode {
    /// Show a scrolling text
    Text(heapless::String<128>),
    /// Show the lava lamp animation
    LavaLamp,
    /// Show the Game of Life simulation
    Life,
    /// Don't paint anything: Something gets painted when this is set, and that's it.
    StaticFramebuffer,
}

static MODE: Signal<CriticalSectionRawMutex, Mode> = Signal::new();

pub(crate) async fn main() {
    ariel_os::asynch::spawner()
        .spawn(run_text_if_any())
        .unwrap();

    super::DISPLAY.lock(|display| {
        let mut display = display.borrow_mut();
        display.draw_iter(
            itertools::iproduct!(0..i32::from(super::N_COLUMNS), 0..i32::from(super::N_ROWS)).map(
                |(x, y)| {
                    Pixel(
                        Point { x, y },
                        Rgb888::new((x as u8), (y as u8), (x + y) as u8 / 8),
                    )
                },
            ),
        );
        display.flush();
    });

    Timer::after(Duration::from_secs(3)).await;
    MODE.signal(Mode::Life);
}

#[ariel_os::task(autostart)]
async fn running_coap() {
    use coap_handler_implementations::{
        HandlerBuilder, TypeHandler, new_dispatcher, with_get_put, with_post, with_put,
    };

    let handler = new_dispatcher()
        .at_with_attributes(
            &["ll"],
            &[],
            TypeHandler::new_minicbor_2(with_post(PostToSetMode(Mode::LavaLamp))),
        )
        .at_with_attributes(
            &["li"],
            &[],
            TypeHandler::new_minicbor_2(with_post(PostToSetMode(Mode::Life))),
        )
        .at_with_attributes(
            &["fb"],
            &[],
            TypeHandler::new_minicbor_2(with_get_put(FrameBuffer)),
        )
        .at_with_attributes(
            &["text"],
            &[],
            TypeHandler::new_minicbor_2(with_put(GlobalText)),
        );

    ariel_os::coap::coap_run(handler).await;
}

struct FrameBuffer;

impl coap_handler_implementations::GetRenderable for FrameBuffer {
    type Get = CborFrameBuffer;

    fn get(&mut self) -> Result<Self::Get, coap_message_utils::Error> {
        Ok(CborFrameBuffer {
            shape: [super::N_ROWS, super::N_COLUMNS, 3].into(),
            data: CurrentFrameBuffer,
        })
    }
}

impl coap_handler_implementations::PutRenderable for FrameBuffer {
    // It'd be great to have, at least for this direction, a variation that just gives us a slice
    // view, but nay: that'd need a Put<'de> associated type.
    type Put = CborFrameBuffer;

    fn put(&mut self, representation: &Self::Put) -> Result<(), coap_message_utils::Error> {
        // By the time the representation has been created, it has all already happened
        if representation.shape != [super::N_ROWS, super::N_COLUMNS, 3] {
            // Cry me a river: but we still already acted on it.
            return Err(coap_message_utils::Error::bad_request().with_title("Bad shape"));
        }
        Ok(())
    }
}

#[derive(minicbor::Encode, minicbor::Decode)]
#[cbor(tag(40), array)] // taG: MultiDimArrayR
struct CborFrameBuffer {
    #[cbor(n(0), with = "minicbor_adapters")]
    shape: heapless::vec::Vec<u16, 3>,
    #[cbor(n(1), tag(64))]
    data: CurrentFrameBuffer,
}

struct PostToSetMode(Mode);

impl coap_handler_implementations::PostRenderable for PostToSetMode {
    type PostIn = coap_handler_implementations::Empty;
    type PostOut = coap_handler_implementations::Empty;

    fn post(
        &mut self,
        representation: &Self::PostIn,
    ) -> Result<Self::PostOut, coap_message_utils::Error> {
        MODE.signal(self.0.clone());
        Ok(coap_handler_implementations::Empty)
    }
}

/// An abomination of an encodable/decodable: This acts right on the frame buffer, rather than just
/// parsing and leaving it to the recipient to act on it.
struct CurrentFrameBuffer;

impl<C> minicbor::Encode<C> for CurrentFrameBuffer {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        super::DISPLAY.lock(|display| {
            let mut display = display.borrow_mut();
            // Let's hope it'll see throught that we don't really need to allocate but just memcpy
            // into the target.
            let mut buffer = [0u8; super::N_LEDS * 3];
            for (i, [r, g, b]) in buffer.as_chunks_mut().0.into_iter().enumerate() {
                let i = i as i32;
                let x = i % i32::from(super::N_COLUMNS);
                let y = i / i32::from(super::N_COLUMNS);
                let pixel = display.read_framebuffer_at(Point { x, y });
                *r = pixel.r();
                *g = pixel.g();
                *b = pixel.b();
            }
            e.bytes(&buffer)
        })?;
        Ok(())
    }
}

impl<'de, C> minicbor::Decode<'de, C> for CurrentFrameBuffer {
    fn decode(
        d: &mut minicbor::Decoder<'de>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        MODE.signal(Mode::StaticFramebuffer);

        // Should we use bytes_iter to support indefinite length?
        let buffer = d.bytes()?;

        super::DISPLAY.lock(|display| {
            let mut display = display.borrow_mut();
            // Let's hope it'll see throught that we don't really need to allocate but just memcpy
            // out of the buffer
            display.draw_iter(buffer.as_chunks().0.into_iter().enumerate().map(
                |(i, [r, g, b])| {
                    let i = i as i32;
                    let x = i % i32::from(super::N_COLUMNS);
                    let y = i / i32::from(super::N_COLUMNS);
                    Pixel(Point { x, y }, Rgb888::new(*r, *g, *b))
                },
            ));
            display.flush();
        });
        Ok(CurrentFrameBuffer)
    }
}

struct GlobalText;

#[derive(minicbor::Decode)]
#[cbor(transparent)]
struct PuttableText(#[cbor(with = "minicbor_adapters")] heapless::String<128>);

impl coap_handler_implementations::PutRenderable for GlobalText {
    type Put = PuttableText;

    fn put(&mut self, representation: &Self::Put) -> Result<(), coap_message_utils::Error> {
        MODE.signal(Mode::Text(representation.0.clone()));
        Ok(())
    }
}

#[ariel_os::task]
async fn run_text_if_any() {
    use embassy_futures::select::{Either, select};

    let mut mode = MODE.wait().await;
    loop {
        let Either::Second(new_mode) = select(
            async {
                match mode {
                    Mode::Text(t) => crate::scrolltext::main(&t).await,
                    Mode::LavaLamp => crate::lavalamp::lavalamp().await,
                    Mode::Life => crate::life::main().await,
                    Mode::StaticFramebuffer => core::future::pending().await,
                }
            },
            MODE.wait(),
        )
        .await;
        mode = new_mode;
    }
}
