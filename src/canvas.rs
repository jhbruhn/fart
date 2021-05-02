//! A canvas for drawing paths on.

use crate::aabb::Aabb;
use crate::path::{LineCommand, Path, ToPaths};
use crate::units::*;
use euclid::point2;
use float_ord::FloatOrd;
use penlib::Pen;
use slotmap::SlotMap;

/// Unit for things within the canvas space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasSpace;

/// Transform from NormalSpace to CanvasSpace
type CanvasProjection = euclid::Transform2D<f64, NormalSpace, CanvasSpace>;

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
struct Layer {
    id: LayerId,
    paths: Vec<Path<f64, CanvasSpace>>,
    color: palette::rgb::LinSrgb,
    nib_size: Millis,
}

impl From<&Layer> for svg::node::element::Group {
    fn from(item: &Layer) -> svg::node::element::Group {
        let color = item.color.into_components();
        svg::node::element::Group::new()
            .set("fill", "none")
            .set("id", format!("layer{}", item.id.0 + 1))
            .set("inkscape:groupmode", "layer")
            .set("inkscape:label", (item.id.0 + 1).to_string())
            .set::<_, String>(
                "stroke",
                format!(
                    "rgb({},{},{})",
                    color.0 * 255.0,
                    color.1 * 255.0,
                    color.2 * 255.0
                ),
            )
            .set("stroke-linecap", "round")
            .set("style", "display:inline")
    }
}

slotmap::new_key_type! {
    /// Key to identify a layer created on the Canvas
    pub struct LayerKey;
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
    layers: SlotMap<LayerKey, Layer>,
    layer_id_counter: u64,
}

impl<Unit> Canvas<Unit>
where
    Unit: SvgUnit,
{
    /// Construct a new canvas with the given viewport.
    pub fn new(paper: Paper<Unit>) -> Canvas<Unit> {
        Canvas {
            paper,
            view: Aabb::new(
                point2(0.0, 0.0),
                point2(paper.width.into(), paper.height.into()),
            ),
            layers: SlotMap::with_key(),
            layer_id_counter: 0,
        }
    }

    /// Get this canvas's width
    #[inline]
    pub fn width(&self) -> Unit {
        self.paper.width - self.paper.margin - self.paper.margin
    }

    /// Get this height's height
    #[inline]
    pub fn height(&self) -> Unit {
        self.paper.height - self.paper.margin - self.paper.margin
    }

    /// Get Transform from normal to Canvas
    #[inline]
    pub fn canvas_transform(&self) -> CanvasProjection {
        CanvasProjection::scale(self.width().into(), self.height().into()).then_translate(
            euclid::vec2(self.paper.margin.into(), self.paper.margin.into()),
        )
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

    /// Register a new Layer using the given pen
    pub fn create_layer<P>(&mut self, pen: P) -> LayerKey
    where
        P: Pen,
    {
        let layer = self.layers.insert(Layer {
            paths: Vec::new(),
            id: LayerId(self.layer_id_counter),
            color: pen.rgb_color(),
            nib_size: Millis(P::nib_size_mm()),
        });
        self.layer_id_counter += 1;
        layer
    }

    /// Remove a layer from the canvas
    pub fn remove_layer(&mut self, key: LayerKey) -> Result<(), ()> {
        self.layers.remove(key).ok_or(()).map(|_| ())
    }

    /// Get an existing layer with the given ID or create it if it does not exist
    fn get_layer(&mut self, key: LayerKey) -> &mut Layer {
        self.layers.get_mut(key).unwrap()
    }

    fn margin_transform(&self) -> euclid::Transform2D<f64, CanvasSpace, CanvasSpace> {
        euclid::Transform2D::translation(self.paper.margin.into(), self.paper.margin.into())
    }

    /// Add the given paths to the canvas.
    pub fn draw<PathsT, P>(&mut self, layer: LayerKey, paths: PathsT)
    where
        PathsT: ToPaths<f64, CanvasSpace>,
        P: Pen + std::hash::Hash + Copy,
    {
        let paths = paths.to_paths();
        let margin_transform = self.margin_transform();
        let layer = self.get_layer(layer);
        for path in paths {
            layer.paths.push(path.transform(&margin_transform));
        }
    }

    /// Add the given paths to the canvas.
    pub fn draw_n<PathsT>(&mut self, layer: LayerKey, paths: PathsT)
    where
        PathsT: ToPaths<f64, crate::units::NormalSpace>,
    {
        let paths = paths.to_paths();
        let projection = self.canvas_transform();

        let layer = self.get_layer(layer);
        for path in paths {
            layer.paths.push(path.transform(&projection));
        }
    }

    /// Given a collection of things that can be drawn, draw all of them.
    pub fn draw_many<I, P, PN>(&mut self, layer: LayerKey, paths: I)
    where
        I: IntoIterator<Item = P>,
        P: ToPaths<f64, CanvasSpace>,
    {
        let margin_transform = self.margin_transform();
        let layer = self.get_layer(layer);
        for p in paths {
            for path in p.to_paths() {
                layer.paths.push(path.transform(&margin_transform));
            }
        }
    }
    /// Given a collection of things that can be drawn, draw all of them.
    pub fn draw_n_many<I, P>(&mut self, layer: LayerKey, paths: I)
    where
        I: IntoIterator<Item = P>,
        P: ToPaths<f64, NormalSpace>,
    {
        let transform = self.canvas_transform();
        let layer = self.get_layer(layer);
        for p in paths {
            for path in p.to_paths() {
                layer.paths.push(path.transform(&transform));
            }
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
                layer_node =
                    layer_node.add(path.set("stroke-width", format!("{}", layer.nib_size.0)));
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
