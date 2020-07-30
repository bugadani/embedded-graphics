//! The line primitive

mod bresenham;
mod points;
mod styled;
mod thick_points;

use crate::{
    geometry::{Dimensions, Point, Size},
    primitives::{
        line::{
            bresenham::{Bresenham, BresenhamParameters},
            thick_points::{ParallelsIterator, Side},
        },
        Primitive, Rectangle,
    },
    transform::Transform,
};
pub use points::Points;
pub use styled::StyledPixels;
pub(in crate::primitives) use thick_points::ThickPoints;

/// Line primitive
///
/// # Examples
///
/// ## Create some lines with different styles
///
/// ```rust
/// use embedded_graphics::{
///     pixelcolor::Rgb565, prelude::*, primitives::Line, style::PrimitiveStyle,
/// };
/// # use embedded_graphics::mock_display::MockDisplay;
/// # let mut display = MockDisplay::default();
///
/// // Red 1 pixel wide line from (50, 20) to (60, 35)
/// Line::new(Point::new(50, 20), Point::new(60, 35))
///     .into_styled(PrimitiveStyle::with_stroke(Rgb565::RED, 1))
///     .draw(&mut display)?;
///
/// // Green 10 pixel wide line with translation applied
/// Line::new(Point::new(50, 20), Point::new(60, 35))
///     .translate(Point::new(-30, 10))
///     .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 10))
///     .draw(&mut display)?;
/// # Ok::<(), core::convert::Infallible>(())
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Line {
    /// Start point
    pub start: Point,

    /// End point
    pub end: Point,
}

impl Primitive for Line {
    type PointsIter = Points;

    fn points(&self) -> Self::PointsIter {
        Points::new(self)
    }
}

impl Dimensions for Line {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::with_corners(self.start, self.end)
    }
}

/// Check signs of two signed numbers
///
/// Fastest ASM output compared to other methods. See: https://godbolt.org/z/zVx9cD
fn same_signs(a: i32, b: i32) -> bool {
    a ^ b >= 0
}

impl Line {
    /// Create a new line
    pub const fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    /// Returns a perpendicular line.
    ///
    /// The returned line is rotated 90 degree counter clockwise and shares the start point with the
    /// original line.
    fn perpendicular(&self) -> Self {
        let delta = self.end - self.start;
        let delta = Point::new(delta.y, -delta.x);

        Line::new(self.start, self.start + delta)
    }

    /// Split the line in half at the midpoint, producing two new lines.
    ///
    /// The end of the first line and the start of the second line lay on the same point.
    pub fn split_in_half(self) -> (Self, Self) {
        let midpoint = self.bounding_box().center();

        (
            Self::new(self.start, midpoint),
            Self::new(midpoint, self.end),
        )
    }

    /// Integer-only line segment intersection
    ///
    /// If the point lies on both line segments, the second tuple argument will return `true`.
    ///
    /// Inspired from https://stackoverflow.com/a/61485959/383609, which links to
    /// https://webdocs.cs.ualberta.ca/~graphics/books/GraphicsGems/gemsii/xlines.c
    pub fn intersection(&self, other: &Self) -> Option<(Point, bool)> {
        let Point { x: x1, y: y1 } = self.start;
        let Point { x: x2, y: y2 } = self.end;
        let Point { x: x3, y: y3 } = other.start;
        let Point { x: x4, y: y4 } = other.end;

        // First line coefficients where "a1 x  +  b1 y  +  c1  =  0"
        let a1 = y2 - y1;
        let b1 = x1 - x2;
        let c1 = x2 * y1 - x1 * y2;

        // Second line coefficients
        let a2 = y4 - y3;
        let b2 = x3 - x4;
        let c2 = x4 * y3 - x3 * y4;

        let denom = a1 * b2 - a2 * b1;

        // Lines are colinear
        if denom == 0 {
            return None;
        }

        // Compute sign values
        let r3 = a1 * x3 + b1 * y3 + c1;
        let r4 = a1 * x4 + b1 * y4 + c1;

        // Sign values for second line
        let r1 = a2 * x1 + b2 * y1 + c2;
        let r2 = a2 * x2 + b2 * y2 + c2;

        // Flag denoting whether intersection point is on passed line segments. If this is false,
        // the intersection occurs somewhere along the two mathematical, infinite lines instead.
        //
        // Check signs of r3 and r4.  If both point 3 and point 4 lie on same side of line 1, the
        // line segments do not intersect.
        //
        // Check signs of r1 and r2.  If both point 1 and point 2 lie on same side of second line
        // segment, the line segments do not intersect.
        let is_on_segments = (r3 != 0 && r4 != 0 && same_signs(r3, r4))
            || (r1 != 0 && r2 != 0 && same_signs(r1, r2));

        // If we got here, line segments intersect. Compute intersection point using method similar
        // to that described here: http://paulbourke.net/geometry/pointlineplane/#i2l

        // The denom/2 is to get rounding instead of truncating. It is added or subtracted to the
        // numerator, depending upon the sign of the numerator.
        let offset = if denom < 0 { -denom / 2 } else { denom / 2 };

        let num = b1 * c2 - b2 * c1;
        let x = if num < 0 { num - offset } else { num + offset } / denom;

        let num = a2 * c1 - a1 * c2;
        let y = if num < 0 { num - offset } else { num + offset } / denom;

        Some((Point::new(x, y), is_on_segments))
    }

    /// Get two lines representing the left and right edges of the thick line.
    pub fn extents(&self, thickness: i32) -> (Line, Line) {
        // let parallel_parameters = BresenhamParameters::new(self);

        // // Thickness threshold, taking into account that fewer pixels are required to draw a
        // // diagonal line of the same perceived width.
        // let delta = (self.end - self.start).abs();
        // let thickness_threshold = 4 * thickness.pow(2) * delta.length_squared();
        // let mut thickness_accumulator =
        //     (parallel_parameters.error_step.minor + parallel_parameters.error_step.major) / 2;
        // let mut side = Side::Right;

        // let mut l = *self;
        // let mut r = *self;

        // while thickness_accumulator.pow(2) <= thickness_threshold {
        //     //
        // }

        let mut it = ParallelsIterator::new(self, thickness);

        let mut start_l = self.start;
        let mut bres_l = Bresenham::with_initial_error(self.start, 0);
        let mut par_points_l = 0;

        let mut start_r = self.start;
        let mut bres_r = Bresenham::with_initial_error(self.start, 0);
        let mut par_points_r = 0;

        while let Some((bres, length_reduction, side)) = it.next() {
            match side {
                Side::Left => {
                    start_l = bres.point;
                    bres_l = bres;
                    par_points_l = bresenham::major_length(self) - length_reduction;
                }
                Side::Right => {
                    start_r = bres.point;
                    bres_r = bres;
                    par_points_r = bresenham::major_length(self) - length_reduction;
                }
            }
        }

        let mut end_l = self.end;
        let mut end_r = self.end;

        for _ in 0..par_points_l {
            end_l = bres_l.next(&it.parallel_parameters);
        }

        for _ in 0..par_points_r {
            end_r = bres_r.next(&it.parallel_parameters);
        }

        let l = Line::new(start_l, end_l);
        let r = Line::new(start_r, end_r);

        // // Left and right start points
        // let mut ext_l_start = self.start;
        // let mut ext_r_start = self.start;

        // while let Some((bres, length_reduction, side)) = it.next() {
        //     match side {
        //         Side::Left => ext_l_start = bres.point,
        //         Side::Right => ext_r_start = bres.point,
        //     }
        // }

        // // Left and right end points
        // let mut ext_l_end = self.end;
        // let mut ext_r_end = self.end;

        // // let delta_ext_l = self.start - ext_l_start;
        // // let delta_ext_r = self.start - ext_r_start;

        // // let ext_l_end = self.end - delta_ext_l;
        // // let ext_r_end = self.end - delta_ext_r;

        // let ext_l = Line::new(ext_l_start, ext_l_end);
        // let ext_r = Line::new(ext_r_start, ext_r_end);

        // (ext_l, ext_r)

        (l, r)
    }

    /// Get the squared length of the line
    pub fn length_squared(&self) -> Size {
        let delta = self.end - self.start;

        // Note: squaring result is always positive. `as u32` casts should be safe here.
        Size::new(delta.x.pow(2) as u32, delta.y.pow(2) as u32)
    }
}

impl Transform for Line {
    /// Translate the line from its current position to a new position by (x, y) pixels, returning
    /// a new `Line`. For a mutating transform, see `translate_mut`.
    ///
    /// ```
    /// # use embedded_graphics::primitives::Line;
    /// # use embedded_graphics::prelude::*;
    /// let line = Line::new(Point::new(5, 10), Point::new(15, 20));
    /// let moved = line.translate(Point::new(10, 10));
    ///
    /// assert_eq!(moved.start, Point::new(15, 20));
    /// assert_eq!(moved.end, Point::new(25, 30));
    /// ```
    fn translate(&self, by: Point) -> Self {
        Self {
            start: self.start + by,
            end: self.end + by,
        }
    }

    /// Translate the line from its current position to a new position by (x, y) pixels.
    ///
    /// ```
    /// # use embedded_graphics::primitives::Line;
    /// # use embedded_graphics::prelude::*;
    /// let mut line = Line::new(Point::new(5, 10), Point::new(15, 20));
    /// line.translate_mut(Point::new(10, 10));
    ///
    /// assert_eq!(line.start, Point::new(15, 20));
    /// assert_eq!(line.end, Point::new(25, 30));
    /// ```
    fn translate_mut(&mut self, by: Point) -> &mut Self {
        self.start += by;
        self.end += by;

        self
    }
}

/// Pixel iterator for each pixel in the line
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        drawable::{Drawable, Pixel},
        geometry::Size,
        mock_display::MockDisplay,
        pixel_iterator::IntoPixels,
        pixelcolor::BinaryColor,
        style::PrimitiveStyle,
    };
    use arrayvec::ArrayVec;

    #[test]
    fn bounding_box() {
        let start = Point::new(10, 10);
        let end = Point::new(19, 29);

        let line: Line = Line::new(start, end);
        let backwards_line: Line = Line::new(end, start);

        assert_eq!(
            line.bounding_box(),
            Rectangle::new(start, Size::new(10, 20))
        );
        assert_eq!(
            backwards_line.bounding_box(),
            Rectangle::new(start, Size::new(10, 20))
        );
    }

    #[test]
    fn no_stroke_width_no_line() {
        let start = Point::new(2, 3);
        let end = Point::new(3, 2);

        let line =
            Line::new(start, end).into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 0));

        assert!(line.into_pixels().eq(core::iter::empty()));
    }

    #[test]
    fn thick_line_octant_1() {
        let mut display: MockDisplay<BinaryColor> = MockDisplay::new();

        Line::new(Point::new(2, 2), Point::new(20, 8))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 5))
            .draw(&mut display)
            .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "   #                   ",
                "  #####                ",
                "  ########             ",
                "  ###########          ",
                " ###############       ",
                "    ###############    ",
                "       ############### ",
                "          ###########  ",
                "             ########  ",
                "                #####  ",
                "                   #   ",
            ])
        );
    }

    #[test]
    fn thick_line_2px() {
        let mut display: MockDisplay<BinaryColor> = MockDisplay::new();

        // Horizontal line
        Line::new(Point::new(2, 2), Point::new(10, 2))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
            .draw(&mut display)
            .unwrap();

        // Vertical line
        Line::new(Point::new(2, 5), Point::new(2, 10))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 2))
            .draw(&mut display)
            .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "            ",
                "  ######### ",
                "  ######### ",
                "            ",
                "            ",
                "  ..        ",
                "  ..        ",
                "  ..        ",
                "  ..        ",
                "  ..        ",
                "  ..        ",
            ])
        );
    }

    // Check that 45 degree lines don't draw their right side 1px too long
    #[test]
    fn diagonal() {
        let mut display: MockDisplay<BinaryColor> = MockDisplay::new();

        Line::new(Point::new(3, 2), Point::new(10, 9))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 7))
            .draw(&mut display)
            .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "     #        ",
                "    ###       ",
                "   #####      ",
                "  #######     ",
                " #########    ",
                "  #########   ",
                "   #########  ",
                "    ######### ",
                "     #######  ",
                "      #####   ",
                "       ###    ",
                "        #     ",
            ])
        );
    }

    #[test]
    fn thick_line_3px() {
        let mut display: MockDisplay<BinaryColor> = MockDisplay::new();

        // Horizontal line
        Line::new(Point::new(2, 2), Point::new(10, 2))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
            .draw(&mut display)
            .unwrap();

        // Vertical line
        Line::new(Point::new(2, 5), Point::new(2, 10))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::Off, 3))
            .draw(&mut display)
            .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "            ",
                "  ######### ",
                "  ######### ",
                "  ######### ",
                "            ",
                " ...        ",
                " ...        ",
                " ...        ",
                " ...        ",
                " ...        ",
                " ...        ",
            ])
        );
    }

    #[test]
    fn thick_line_0px() {
        let mut display: MockDisplay<BinaryColor> = MockDisplay::new();

        Line::new(Point::new(2, 2), Point::new(2, 2))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
            .draw(&mut display)
            .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "   ", //
                "  #", //
                "  #", //
                "  #", //
            ])
        );
    }

    #[test]
    fn event_width_offset() {
        let mut display: MockDisplay<BinaryColor> = MockDisplay::new();

        // Horizontal line
        Line::new(Point::new(2, 3), Point::new(10, 3))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 4))
            .draw(&mut display)
            .unwrap();

        // Vertical line
        Line::new(Point::new(2, 9), Point::new(10, 8))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 4))
            .draw(&mut display)
            .unwrap();

        assert_eq!(
            display,
            MockDisplay::from_pattern(&[
                "            ",
                "  ######### ",
                "  ######### ",
                "  ######### ",
                "  ######### ",
                "            ",
                "       #### ",
                "  ######### ",
                "  ######### ",
                "  ######### ",
                "  #####     ",
            ])
        );
    }

    #[test]
    fn points_iter() {
        let line = Line::new(Point::new(10, 10), Point::new(20, 30));

        let styled_points: ArrayVec<[_; 32]> = line
            .clone()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .into_pixels()
            .map(|Pixel(p, _)| p)
            .collect();

        let points: ArrayVec<[_; 32]> = line.points().collect();

        assert_eq!(points, styled_points);
    }

    #[test]
    fn perpendicular() {
        assert_eq!(
            Line::new(Point::zero(), Point::new(10, 0)).perpendicular(),
            Line::new(Point::zero(), Point::new(0, -10))
        );

        assert_eq!(
            Line::new(Point::new(10, 20), Point::new(20, 10)).perpendicular(),
            Line::new(Point::new(10, 20), Point::new(0, 10))
        );

        assert_eq!(
            Line::new(Point::zero(), Point::new(0, -10)).perpendicular(),
            Line::new(Point::zero(), Point::new(-10, 0))
        );
    }
}
