//! A canvas for drawing paths on.

use crate::aabb::Aabb;
use crate::path::{LineCommand, Path, ToPaths};
use euclid::point2;

use float_ord::FloatOrd;

/// Unit for things within the canvas space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasSpace;

/// A canvas is a collection of rendered paths. To add new paths to the canvas,
/// use the `draw` method.
#[derive(Debug)]
pub struct Canvas {
    view: Aabb<f64, CanvasSpace>,
    paths: Vec<Path<f64, CanvasSpace>>,
    stroke_width: f64,
}

impl Canvas {
    /// Construct a new canvas with the given viewport.
    pub fn new(view: Aabb<f64, CanvasSpace>) -> Canvas {
        let stroke_width = std::cmp::max(FloatOrd(1.0), FloatOrd(view.width() / 500.0)).0;
        Canvas {
            view,
            paths: Vec::new(),
            stroke_width,
        }
    }

    /// Get the stroke width for paths in this canvas.
    pub fn stroke_width(&self) -> f64 {
        self.stroke_width
    }

    /// Set the stroke width for paths in this canvas.
    pub fn set_stroke_width(&mut self, stroke_width: f64) {
        self.stroke_width = stroke_width;
    }

    /// Get this canvas's view.
    #[inline]
    pub fn view(&self) -> &Aabb<f64, CanvasSpace> {
        &self.view
    }

    /// Set this canvas's view.
    pub fn set_view(&mut self, view: Aabb<f64, CanvasSpace>) {
        self.view = view;
    }

    /// Make this canvas's view the bounding box of all the paths that have been
    /// added to the canvas.
    pub fn fit_view_to_paths(&mut self) {
        if self.paths.is_empty() {
            return;
        }

        let mut min_x = std::f64::MAX;
        let mut min_y = std::f64::MAX;
        let mut max_x = std::f64::MIN;
        let mut max_y = std::f64::MIN;

        let mut process_point = |p: &euclid::Point2D<f64, CanvasSpace>| {
            min_x = std::cmp::min(FloatOrd(min_x), FloatOrd(p.x)).0;
            min_y = std::cmp::min(FloatOrd(min_y), FloatOrd(p.y)).0;
            max_x = std::cmp::max(FloatOrd(max_x), FloatOrd(p.x)).0;
            max_y = std::cmp::max(FloatOrd(max_y), FloatOrd(p.y)).0;
        };

        for path in self.paths.iter() {
            for cmd in path.commands.iter() {
                match cmd {
                    LineCommand::MoveTo(p)
                    | LineCommand::LineTo(p)
                    | LineCommand::SmoothQuadtraticCurveTo(p) => process_point(p),

                    LineCommand::CubicBezierTo {
                        control_1,
                        control_2,
                        end,
                    } => {
                        process_point(control_1);
                        process_point(control_2);
                        process_point(end);
                    }

                    LineCommand::SmoothCubicBezierTo { control, end }
                    | LineCommand::QuadraticBezierTo { control, end } => {
                        process_point(control);
                        process_point(end);
                    }

                    LineCommand::Close => {}

                    LineCommand::MoveBy(_)
                    | LineCommand::LineBy(_)
                    | LineCommand::HorizontalLineTo(_)
                    | LineCommand::HorizontalLineBy(_)
                    | LineCommand::VerticalLineTo(_)
                    | LineCommand::VerticalLineBy(_)
                    | LineCommand::CubicBezierBy { .. }
                    | LineCommand::SmoothCubicBezierBy { .. }
                    | LineCommand::QuadraticBezierBy { .. }
                    | LineCommand::SmoothQuadtraticCurveBy(_)
                    | LineCommand::ArcTo { .. }
                    | LineCommand::ArcBy { .. } => unimplemented!(),
                }
            }
        }

        let view = Aabb::new(point2(min_x, min_y), point2(max_x, max_y));
        self.set_view(view);
    }

    /// Add the given paths to the canvas.
    pub fn draw<P>(&mut self, paths: P)
    where
        P: ToPaths<f64, CanvasSpace>,
    {
        self.paths.extend(paths.to_paths());
    }

    /// Given a collection of things that can be drawn, draw all of them.
    pub fn draw_many<I, P>(&mut self, paths: I)
    where
        I: IntoIterator<Item = P>,
        P: ToPaths<f64, CanvasSpace>,
    {
        for p in paths {
            self.draw(p);
        }
    }

    /// Render this canvas as an SVG with the given physical width and height.
    ///
    /// # Example
    ///
    /// Make a 3" x 3" SVG from a canvas.
    ///
    /// ```
    /// use fart::aabb::Aabb;
    /// use fart::euclid::point2;
    /// use fart::canvas::{Inches, Canvas};
    ///
    /// let canvas = Canvas::new(Aabb::new(
    ///     point2(0, 0),
    ///     point2(100, 100),
    /// ));
    /// let svg_doc = canvas.create_svg(Inches(3.0), Inches(3.0));
    /// # let _ = svg_doc;
    /// ```
    pub fn create_svg<W, H>(&self, width: W, height: H) -> svg::Document
    where
        W: SvgUnit,
        H: SvgUnit,
    {
        let width = width.into();
        let height = height.into();
        let mut doc = svg::Document::new()
            .set(
                "viewBox",
                format!(
                    "{} {} {} {}",
                    self.view.min().x,
                    self.view.min().y,
                    self.view.width(),
                    self.view.height(),
                ),
            )
            .set("width", format!("{}{}", width, W::SUFFIX))
            .set("height", format!("{}{}", height, H::SUFFIX));
        for path in &self.paths {
            let path: svg::node::element::Path = path.into();
            doc = doc.add(path.set("stroke-width", self.stroke_width));
        }
        doc
    }
}

impl ToPaths<f64, CanvasSpace> for Canvas {
    type Paths = std::vec::IntoIter<Path<f64, CanvasSpace>>;

    fn to_paths(&self) -> Self::Paths {
        self.paths.clone().into_iter()
    }
}

/// A physical unit supported by SVG (inches, centimeters, etc). Used when
/// plotting an image.
pub trait SvgUnit: Into<f64> {
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
