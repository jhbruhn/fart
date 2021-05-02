//! Commonly used units and paper size definitions

/// Normalized Space from 0 to 1
#[derive(Debug)]
pub struct NormalSpace;
///Normalized Point from 0 to 1
pub type NormalPoint = euclid::Point2D<f64, NormalSpace>;
/// Normalized Size from 0 to 1
pub type NormalSize = euclid::Size2D<f64, NormalSpace>;

/// A sheet of paper
#[derive(Debug, Copy, Clone)]
pub struct Paper<Unit>
where
    Unit: SvgUnit,
{
    // TODO: use Size2D?
    /// Width of the sheet of paper
    pub width: Unit,
    /// Height of the sheet of paper
    pub height: Unit,

    /// Margin at the top
    pub margin_top: Unit,
    /// Margin at the bottom
    pub margin_bottom: Unit,
    /// Margin at the left
    pub margin_left: Unit,
    /// Margin at the right
    pub margin_right: Unit,
}

impl<Unit> Paper<Unit>
where
    Unit: SvgUnit,
{
    /// Create a new Paper with associated width and height
    pub fn new(width: Unit, height: Unit) -> Paper<Unit> {
        Self {
            width,
            height,
            margin_top: Unit::ZERO,
            margin_bottom: Unit::ZERO,
            margin_left: Unit::ZERO,
            margin_right: Unit::ZERO,
        }
    }

    /// Turn this Paper by 90 degrees
    pub fn switch_orientation(self) -> Paper<Unit> {
        Self {
            width: self.height,
            height: self.width,
            margin_top: self.margin_left,
            margin_bottom: self.margin_right,
            margin_left: self.margin_top,
            margin_right: self.margin_bottom,
        }
    }

    /// Add a margin to the edges
    pub fn add_margin(self, margin: Unit) -> Paper<Unit> {
        Self {
            width: self.width,
            height: self.height,
            margin_left: margin,
            margin_right: margin,
            margin_top: margin,
            margin_bottom: margin,
        }
    }

    /// Add different margins to the edges
    pub fn add_margins(
        self,
        margin_top: Unit,
        margin_right: Unit,
        margin_bottom: Unit,
        margin_left: Unit,
    ) -> Paper<Unit> {
        Self {
            width: self.width,
            height: self.height,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
        }
    }

    /// Make the margins so that the content is centered in square with the smallest margin being
    /// given in the argument
    pub fn make_square(self, smallest_margin: Unit) -> Paper<Unit> {
        if self.width > self.height {
            let size = self.height - smallest_margin - smallest_margin;
            let margin_sides = (self.width - size) / 2.0;
            Self {
                width: self.width,
                height: self.height,
                margin_top: smallest_margin,
                margin_bottom: smallest_margin,
                margin_left: margin_sides,
                margin_right: margin_sides,
            }
        } else {
            let size = self.width - smallest_margin - smallest_margin;
            let margin_sides = (self.height - size) / 2.0;
            Self {
                width: self.width,
                height: self.height,
                margin_top: margin_sides,
                margin_bottom: margin_sides,
                margin_left: smallest_margin,
                margin_right: smallest_margin,
            }
        }
    }
}

macro_rules! const_paper_mm {
    ($name:ident, $width:expr, $height:expr) => {
        #[doc = "$name Paper ($width x $height)"]
        pub const $name: Paper<Millis> = Paper {
            width: Millis($width),
            height: Millis($height),
            margin_top: Millis::ZERO,
            margin_bottom: Millis::ZERO,
            margin_left: Millis::ZERO,
            margin_right: Millis::ZERO,
        };
    };
}

/// Constants for varous paper types
pub mod papers {
    use super::*;

    const_paper_mm!(DIN_A0, 841.0, 1189.0);
    const_paper_mm!(DIN_A1, 594.0, 841.0);
    const_paper_mm!(DIN_A2, 420.0, 594.0);
    const_paper_mm!(DIN_A3, 297.0, 420.0);
    const_paper_mm!(DIN_A4, 210.0, 297.0);
    const_paper_mm!(DIN_A5, 148.0, 210.0);
    const_paper_mm!(DIN_A6, 105.0, 147.0);
    const_paper_mm!(DIN_A7, 074.0, 105.0);
    const_paper_mm!(DIN_A8, 052.0, 074.0);
    const_paper_mm!(DIN_A9, 037.0, 052.0);
    const_paper_mm!(DIN_A10, 026.0, 037.0);
}

//impl<T, U> ToAabb<T, U> for Paper {}

/// A physical unit supported by SVG (inches, centimeters, etc). Used when
/// plotting an image.
pub trait SvgUnit:
    Copy + Into<f64> + std::ops::Sub<Output = Self> + std::cmp::PartialOrd + std::ops::Div<f64, Output = Self>
{
    /// The unit's string suffix.
    const SUFFIX: &'static str;
    /// The unit's zero value.
    const ZERO: Self;
}

/// Express an canvas's SVG's physical dimensions in inches.
///
/// See `Canvas::create_svg` for examples.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Inches(pub f64);

impl From<Inches> for f64 {
    fn from(i: Inches) -> f64 {
        i.0
    }
}

impl SvgUnit for Inches {
    const SUFFIX: &'static str = "in";
    const ZERO: Self = Self(0.0);
}

impl std::ops::Sub for Inches {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl std::ops::Div<f64> for Inches {
    type Output = Self;

    fn div(self, other: f64) -> Self::Output {
        Self(self.0 / other)
    }
}

/// Express an canvas's SVG's physical dimensions in millimeters.
///
/// See `Canvas::create_svg` for examples.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Millis(pub f64);

impl From<Millis> for f64 {
    fn from(i: Millis) -> f64 {
        i.0
    }
}

impl SvgUnit for Millis {
    const SUFFIX: &'static str = "mm";
    const ZERO: Self = Self(0.0);
}

impl std::ops::Sub for Millis {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl std::ops::Div<f64> for Millis {
    type Output = Self;

    fn div(self, other: f64) -> Self::Output {
        Self(self.0 / other)
    }
}

impl From<Inches> for Millis {
    fn from(i: Inches) -> Millis {
        Millis(i.0 * 25.4)
    }
}

impl From<Millis> for Inches {
    fn from(i: Millis) -> Inches {
        Inches(i.0 / 25.4)
    }
}
