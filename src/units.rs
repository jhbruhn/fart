//! Commonly used units and paper size definitions

/// A sheet of paper
#[derive(Debug, Copy, Clone)]
pub struct Paper<Unit>
where
    Unit: SvgUnit,
{
    /// Width of the sheet of paper
    pub width: Unit,
    /// Height of the sheet of paper
    pub height: Unit,
}

impl<Unit> Paper<Unit>
where
    Unit: SvgUnit,
{
    /// Create a new Paper with associated width and height
    pub fn new(width: Unit, height: Unit) -> Paper<Unit> {
        Paper { width, height }
    }

    /// Turn this Paper by 90 degrees
    pub fn switch_orientation(self) -> Paper<Unit> {
        Paper {
            width: self.height,
            height: self.width,
        }
    }
}

macro_rules! const_paper_mm {
    ($name:ident, $width:expr, $height:expr) => {
        #[doc = "$name Paper ($width x $height)"]
        pub const $name: Paper<Millis> = Paper {
            width: Millis($width),
            height: Millis($height),
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
pub trait SvgUnit: Into<f64> + Copy {
    /// The unit's string suffix.
    const SUFFIX: &'static str;
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
