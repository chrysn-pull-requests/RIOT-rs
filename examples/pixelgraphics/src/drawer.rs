use embedded_graphics::{
    geometry::{OriginDimensions, Size},
    pixelcolor::Rgb888,
    prelude::*,
};
use smart_leds::RGB8;

pub struct MyDrawTarget {
    framebuffer: [RGB8; 256],
}

impl MyDrawTarget {
    pub const fn new() -> Self {
        Self {
            framebuffer: [RGB8::new(0, 0, 0); 256],
        }
    }

    pub fn get(&mut self, n: usize) -> RGB8 {
        if n > 255 {
            return RGB8::new(0, 0, 0);
        }
        self.framebuffer[n]
    }

    pub fn set(&mut self, n: usize) {
        if n > 255 {
            return;
        }
        self.framebuffer[n] = RGB8::new(200, 200, 200);
    }

    pub fn unset(&mut self, n: usize) {
        if n > 255 {
            return;
        }
        self.framebuffer[n] = RGB8::new(0, 0, 0);
    }

    /// Updates the display from the framebuffer.
    pub fn flush(&self) {
        crate::PIXELS.lock(|out| out.set(self.framebuffer));
        crate::SIGNAL.signal(());
    }
}

impl DrawTarget for MyDrawTarget {
    type Color = Rgb888;
    // `ExampleDisplay` uses a framebuffer and doesn't need to communicate with the display
    // controller to draw pixel, which means that drawing operations can never fail. To reflect
    // this the type `Infallible` was chosen as the `Error` type.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // Check if the pixel coordinates are out of bounds (negative or greater than
            // (32,8)). `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            if let Ok((x @ 0..=31, y @ 0..=7)) = coord.try_into() {
                // Calculate the index in the framebuffer.
                let index: u32 = {
                    if y % 2 == 0 {
                        x + y * 32
                    } else {
                        (y) * 32 + 31 - x
                    }
                };

                self.framebuffer[index as usize] = RGB8::new(color.r(), color.g(), color.b());
            }
        }

        Ok(())
    }
}

impl OriginDimensions for MyDrawTarget {
    fn size(&self) -> Size {
        Size::new(32, 8)
    }
}
