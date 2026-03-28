//! Module that provides usable coordinate based drawing for the [`super::graphicsdriver`]
//! back-end.

use embedded_graphics::{
    geometry::{OriginDimensions, Size},
    pixelcolor::Rgb888,
    prelude::*,
};
use smart_leds::RGB8;

pub struct MyDrawTarget {
    framebuffer: [RGB8; super::N_LEDS],
}

impl MyDrawTarget {
    pub const fn new() -> Self {
        Self {
            framebuffer: [RGB8::new(0, 0, 0); super::N_LEDS],
        }
    }

    /// Exceeding what embedded-graphics promises, we currently allow access back to the frame
    /// buffer.
    pub fn read_framebuffer_at(&mut self, point: Point) -> <Self as DrawTarget>::Color {
        if let Some(index) = Self::coord_to_index(point) {
            let strip_color = self.framebuffer[index];
            Rgb888::new(strip_color.r, strip_color.g, strip_color.b)
        } else {
            Rgb888::BLACK
        }
    }

    /// Updates the display from the framebuffer.
    pub fn flush(&self) {
        crate::PIXELS.lock(|out| out.set(self.framebuffer));
        crate::SIGNAL.signal(());
    }
}

/// Type conversion helper because the LED strip is usize indiced, but Point uses i32, and the
/// bounding macros are yet different beasts (currently u16).
fn point_to_usizes(Point { x, y }: Point) -> Option<(usize, usize)> {
    let x: usize = x.try_into().ok()?;
    let y: usize = y.try_into().ok()?;
    if x < usize::from(super::N_COLUMNS) && y < usize::from(super::N_ROWS) {
        Some((x, y))
    } else {
        None
    }
}

/// Boustrophedon layout: pixels are pixels start at top left going right, and then return
// To become cfg_select once 1.95 lands
#[cfg(context = "ulanzi-tc001")]
impl MyDrawTarget {
    /// Maps a Point in the dislpay coordinate system into an index in the LED strip.
    ///
    /// Returns None when out of bounds.
    fn coord_to_index(coord: Point) -> Option<usize> {
        let (x, y) = point_to_usizes(coord)?;

        Some(if y % 2 == 0 {
            x + y * usize::from(super::N_COLUMNS)
        } else {
            (y + 1) * usize::from(super::N_COLUMNS) - 1 - x
        })
    }
}

/// Plain line-wise arrangement, LTR.
#[cfg(context = "waveshare-esp32-s3-matrix")]
impl MyDrawTarget {
    fn coord_to_index(coord: Point) -> Option<usize> {
        let (x, y) = point_to_usizes(coord)?;

        Some(x + y * usize::from(super::N_COLUMNS))
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
            let Some(index) = Self::coord_to_index(coord) else {
                // Check if the pixel coordinates are out of bounds (negative or greater than
                // (32,8)). `DrawTarget` implementation are required to discard any out of bounds
                // pixels without returning an error or causing a panic.
                continue;
            };

            self.framebuffer[index] = RGB8::new(color.r(), color.g(), color.b());
        }

        Ok(())
    }
}

impl OriginDimensions for MyDrawTarget {
    fn size(&self) -> Size {
        Size::new(super::N_COLUMNS as _, super::N_ROWS as _)
    }
}
