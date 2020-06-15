use crate::drawable::Pixel;
use crate::pixelcolor::PixelColor;
use crate::primitives::rectangle;
use crate::primitives::Primitive;
use crate::primitives::Rectangle;
use core::iter::Zip;

#[derive(Clone, Debug)]
pub struct SparseIterator<C, I>
where
    C: PixelColor,
    I: Iterator<Item = Option<C>>,
{
    iter: Zip<rectangle::Points, I>,
}

impl<C, I> SparseIterator<C, I>
where
    C: PixelColor,
    I: Iterator<Item = Option<C>>,
{
    pub fn new(area: Rectangle, iter: I) -> Self {
        let area_iter = area.points();
        let color_iter = iter;

        let iter = area_iter.zip(color_iter);

        Self { iter }
    }
}

impl<C, I> Iterator for SparseIterator<C, I>
where
    C: PixelColor,
    I: Iterator<Item = Option<C>>,
{
    type Item = Pixel<C>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .find_map(|(point, color)| color.map(|color| Pixel(point, color)))
    }
}
