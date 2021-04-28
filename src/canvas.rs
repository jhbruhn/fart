//! A canvas for drawing paths on.

use crate::aabb::Aabb;
use crate::path::{LineCommand, Path, ToPaths};
use crate::units::*;
use euclid::point2;
use float_ord::FloatOrd;
use std::collections::BTreeMap;

/// Unit for things within the canvas space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasSpace;

#[derive(Clone, Copy, Debug)]
struct LayerId(u64);

impl From<LayerId> for String {
    fn from(id: LayerId) -> String {
        match id {
            LayerId(0) => String::from("#00f"),
            LayerId(1) => String::from("#080"),
            LayerId(2) => String::from("#f00"),
            LayerId(3) => String::from("#0cc"),
            LayerId(4) => String::from("#0f0"),
            LayerId(5) => String::from("#c0c"),
            LayerId(6) => String::from("#cc0"),
            _ => String::from("black"),
        }
    }
}

/// A Layer contains a collection of path to be drawn on that specific layer
#[derive(Debug)]
pub struct Layer {
    id: LayerId,
    paths: Vec<Path<f64, CanvasSpace>>,
}

impl From<&Layer> for svg::node::element::Group {
    fn from(item: &Layer) -> svg::node::element::Group {
        svg::node::element::Group::new()
            .set("fill", "none")
            .set("id", format!("layer{}", item.id.0))
            .set("inkscape:groupmode", "layer")
            .set("inkscape:label", item.id.0.to_string())
            .set::<_, String>("stroke", item.id.into())
            .set("style", "display:inline")
    }
}

/// A canvas is a collection of rendered paths. To add new paths to the canvas,
/// use the `draw` method.
#[derive(Debug)]
pub struct Canvas<Unit>
where
    Unit: SvgUnit,
{
    paper: Paper<Unit>,
    view: Aabb<f64, CanvasSpace>,
    layers: BTreeMap<u64, Layer>,
    stroke_width: f64,
}

impl<Unit> Canvas<Unit>
where
    Unit: SvgUnit,
{
    /// Construct a new canvas with the given viewport.
    pub fn new(paper: Paper<Unit>) -> Canvas<Unit> {
        let stroke_width = std::cmp::max(FloatOrd(0.2), FloatOrd(paper.width.into() / 500.0)).0;
        Canvas {
            paper,
            view: Aabb::new(
                point2(0.0, 0.0),
                point2(paper.width.into(), paper.height.into()),
            ),
            layers: BTreeMap::new(),
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

    /// Get this canvas's paper
    #[inline]
    pub fn paper(&self) -> Paper<Unit> {
        self.paper
    }

    /// Get this canvas's view.
    #[inline]
    pub fn view(&self) -> &Aabb<f64, CanvasSpace> {
        &self.view
    }

    /// Set this canvas's view.
    #[inline]
    pub fn set_view(&mut self, view: Aabb<f64, CanvasSpace>) {
        self.view = view;
    }

    /// Make this canvas's view the bounding box of all the paths that have been
    /// added to the canvas.
    pub fn fit_view_to_paths(&mut self) {
        if self.layers.is_empty() {
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

        for layer in self.layers.values() {
            for path in layer.paths.iter() {
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
        }

        let view = Aabb::new(point2(min_x, min_y), point2(max_x, max_y));
        self.set_view(view);
    }

    /// Create the layer with the given idea. panics if it exists
    fn create_layer(&mut self, layer_id: u64) {
        assert!(!self.layers.contains_key(&layer_id));
        self.layers.insert(
            layer_id,
            Layer {
                paths: Vec::new(),
                id: LayerId(layer_id),
            },
        );
    }

    /// Get an existing layer with the given ID or create it if it does not exist
    pub fn create_or_get_layer(&mut self, layer_id: u64) -> &mut Layer {
        if !self.layers.contains_key(&layer_id) {
            self.create_layer(layer_id);
        }
        self.layers.get_mut(&layer_id).unwrap()
    }

    /// Add the given paths to the canvas.
    pub fn draw<P>(&mut self, layer_id: u64, paths: P)
    where
        P: ToPaths<f64, CanvasSpace>,
    {
        self.create_or_get_layer(layer_id)
            .paths
            .extend(paths.to_paths());
    }

    /// Given a collection of things that can be drawn, draw all of them.
    pub fn draw_many<I, P>(&mut self, layer_id: u64, paths: I)
    where
        I: IntoIterator<Item = P>,
        P: ToPaths<f64, CanvasSpace>,
    {
        let layer = self.create_or_get_layer(layer_id);
        for p in paths {
            layer.paths.extend(p.to_paths());
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
    pub fn create_svg(&self) -> svg::Document {
        let width = self.paper.width.into();
        let height = self.paper.height.into();
        let mut doc = svg::Document::new()
            .set(
                "xmlns:inkscape",
                "http://www.inkscape.org/namespaces/inkscape",
            )
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
            .set("width", format!("{}{}", width, Unit::SUFFIX))
            .set("height", format!("{}{}", height, Unit::SUFFIX));
        for layer in self.layers.values() {
            // TODO: create svg layer
            let mut layer_node: svg::node::element::Group = layer.into();

            for path in &layer.paths {
                let path: svg::node::element::Path = path.into();
                layer_node = layer_node.add(path.set("stroke-width", self.stroke_width));
            }

            doc = doc.add(layer_node);
        }
        doc
    }
}

impl<Unit> ToPaths<f64, CanvasSpace> for Canvas<Unit>
where
    Unit: SvgUnit,
{
    type Paths = std::vec::IntoIter<Path<f64, CanvasSpace>>;

    fn to_paths(&self) -> Self::Paths {
        self.layers
            .values()
            .flat_map(|v| v.paths.clone())
            .collect::<Vec<Path<f64, CanvasSpace>>>()
            .into_iter()
    }
}
