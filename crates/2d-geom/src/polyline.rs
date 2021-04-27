use euclid::Point2D;

/// A Polyline is a line along multiple points. Like a Polygon that is not closed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Polyline<T, U> {
    /// Points in the polyline
    pub vertices: Vec<Point2D<T, U>>,
}

impl<T, U> Polyline<T, U>
where
    T: Copy + PartialOrd
{
    /// Construct a new Polyline
    pub fn new(vertices: Vec<Point2D<T, U>>) -> Polyline<T, U> {
        assert!(vertices.len() >= 2);

        Polyline { vertices }
    }

    /// All points stored for this line
    pub fn vertices(&self) -> &[Point2D<T, U>] {
        &self.vertices
    }

    /// Get a point indexed with i
    pub fn get(&self, i: usize) -> Option<Point2D<T, U>> {
        self.vertices.get(i).cloned()
    }

    /// Amount of Points in this line
    pub fn len(&self) -> usize {
        self.vertices.len()
    }
}
