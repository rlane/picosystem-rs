use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

pub struct Sprite<'a> {
    pub size: Size,
    pub transparent_color: Option<u16>,
    pub data: &'a [u16],
}

impl ImageDrawable for Sprite<'_> {
    type Color = Rgb565;

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        if let Some(transparent_color) = self.transparent_color {
            let mut x = 0;
            let mut y = 0;
            for p in self.data.iter() {
                if x >= self.size.width as i32 {
                    x = 0;
                    y += 1;
                }
                if *p != transparent_color {
                    let pixels = [Pixel(Point::new(x, y), RawU16::new(*p).into())];
                    target.draw_iter(pixels.iter().cloned())?;
                }
                x += 1;
            }
            Ok(())
        } else {
            target.fill_contiguous(
                &Rectangle::new(Point::new(0, 0), self.size),
                self.data.iter().map(|c| RawU16::new(*c).into()),
            )
        }
    }

    fn draw_sub_image<D>(&self, target: &mut D, area: &Rectangle) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        // TODO transparency
        for (iy, y) in (area.top_left.y..(area.top_left.y + area.size.height as i32)).enumerate() {
            let start_index = area.top_left.x as usize + (y * self.size.width as i32) as usize;
            let end_index = start_index + area.size.width as usize;
            let slice = &self.data[start_index..end_index];
            target.fill_contiguous(
                &Rectangle::new(Point::new(0, iy as i32), Size::new(area.size.width, 1)),
                slice.iter().map(|c| RawU16::new(*c).into()),
            )?;
        }
        Ok(())
    }
}

impl OriginDimensions for Sprite<'_> {
    fn size(&self) -> Size {
        self.size
    }
}
