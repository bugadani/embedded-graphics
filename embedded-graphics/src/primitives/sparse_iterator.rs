use crate::drawable::Pixel;
use crate::pixelcolor::PixelColor;
use crate::primitives::rectangle;
use crate::primitives::Primitive;
use crate::primitives::Rectangle;
use core::marker::PhantomData;

struct SparseIterator<C, I>
where
    C: PixelColor,
    I: Iterator<Item = Pixel<C>>,
{
    rect: rectangle::Points,
    iter: I,
    current: Option<Pixel<C>>,
    _color: PhantomData<C>,
}

impl<C, I> SparseIterator<C, I>
where
    C: PixelColor,
    I: Iterator<Item = Pixel<C>>,
{
    pub fn new(area: Rectangle, iter: I) -> Self {
        let rect = area.points();
        let mut iter = iter;
        let current = iter.next();

        Self {
            rect,
            iter,
            current,
            _color: PhantomData,
        }
    }
}

impl<C, I> Iterator for SparseIterator<C, I>
where
    C: PixelColor,
    I: Iterator<Item = Pixel<C>>,
{
    type Item = Option<C>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(point) = self.rect.next() {
            let c = if let Some(c) = self.current.filter(|pixel| pixel.0 == point) {
                self.current = self.iter.next();

                Some(c.1)
            } else {
                None
            };

            Some(c)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_target::DrawTarget;
    use crate::geometry::{Dimensions, Point};
    use crate::mock_display::MockDisplay;
    use crate::pixelcolor::{Rgb565, RgbColor};
    use crate::primitives::Circle;
    use crate::style::PrimitiveStyle;

    #[test]
    fn check() {
        let mut display = MockDisplay::new();
        let mut sparse_display = MockDisplay::new();

        let c = Circle::new(Point::new(10, 10), 5);

        let styled = c.into_styled(PrimitiveStyle::with_stroke(Rgb565::RED, 1));

        let sparse = SparseIterator::new(styled.bounding_box(), styled.into_iter());

        display.draw_iter(styled.into_iter()).unwrap();
        sparse_display
            .fill_sparse(&styled.bounding_box(), sparse)
            .unwrap();

        assert_eq!(display, sparse_display);
    }
}
