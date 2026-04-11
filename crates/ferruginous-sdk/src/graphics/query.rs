//! Path query and snapping engine for engineering precision.
use kurbo::{Point, BezPath, PathEl};
use crate::graphics::DrawOp;

/// Type of snap point found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapType {
    /// Snap to the start or end of a path segment.
    Endpoint,
    /// Snap to the midpoint of a straight line segment.
    Midpoint,
    /// Snap to the closest point along an edge.
    Edge,
    /// Snap to where two segments intersect.
    Intersection,
}

/// Result of a snap query.
#[derive(Debug, Clone, Copy)]
pub struct SnapResult {
    /// The geometric coordinate of the snap point.
    pub point: Point,
    /// The category of snap point found.
    pub snap_type: SnapType,
    /// The Euclidean distance from the target to the snap point.
    pub distance: f64,
}

/// Engine for querying vector paths in a display list.
pub struct PathQuery {
    /// The list of drawing operations to query against.
    pub display_list: Vec<DrawOp>,
}

impl PathQuery {
    /// Creates a new query engine from a display list.
    pub fn new(display_list: Vec<DrawOp>) -> Self {
        Self { display_list }
    }

    /// Finds the closest snap point to the given target point within a threshold.
    pub fn find_snap_point(&self, target: Point, threshold: f64) -> Option<SnapResult> {
        let mut best_snap: Option<SnapResult> = None;

        for op in &self.display_list {
            match op {
                DrawOp::FillPath { path, .. } | DrawOp::StrokePath { path, .. } | DrawOp::DrawPath(path, ..) => {
                    self.query_path(path, target, threshold, &mut best_snap);
                }
                _ => {}
            }
        }

        best_snap
    }

    fn query_path(&self, path: &BezPath, target: Point, threshold: f64, best_snap: &mut Option<SnapResult>) {
        let mut prev_point: Option<Point> = None;

        for el in path.elements() {
            match el {
                PathEl::MoveTo(p) => {
                    self.check_point(*p, SnapType::Endpoint, target, threshold, best_snap);
                    prev_point = Some(*p);
                }
                PathEl::LineTo(p) => {
                    self.check_point(*p, SnapType::Endpoint, target, threshold, best_snap);
                    if let Some(prev) = prev_point {
                        // Check midpoint
                        let mid = Point::new((prev.x + p.x) / 2.0, (prev.y + p.y) / 2.0);
                        self.check_point(mid, SnapType::Midpoint, target, threshold, best_snap);
                        
                        // Check closest point on edge
                        let edge_closest = self.closest_point_on_line(prev, *p, target);
                        self.check_point(edge_closest, SnapType::Edge, target, threshold, best_snap);
                    }
                    prev_point = Some(*p);
                }
                PathEl::QuadTo(_, p) => {
                    self.check_point(*p, SnapType::Endpoint, target, threshold, best_snap);
                    prev_point = Some(*p);
                }
                PathEl::CurveTo(_, _, p) => {
                    self.check_point(*p, SnapType::Endpoint, target, threshold, best_snap);
                    prev_point = Some(*p);
                }
                PathEl::ClosePath => {}
            }
        }
    }

    fn check_point(&self, p: Point, snap_type: SnapType, target: Point, threshold: f64, best_snap: &mut Option<SnapResult>) {
        let dist = p.distance(target);
        if dist <= threshold {
            if let Some(best) = best_snap {
                if dist < best.distance {
                    *best_snap = Some(SnapResult { point: p, snap_type, distance: dist });
                }
            } else {
                *best_snap = Some(SnapResult { point: p, snap_type, distance: dist });
            }
        }
    }

    fn closest_point_on_line(&self, a: Point, b: Point, p: Point) -> Point {
        let ap = p - a;
        let ab = b - a;
        let dot = ap.x * ab.x + ap.y * ab.y;
        let mag_sq = ab.x * ab.x + ab.y * ab.y;
        if mag_sq == 0.0 { return a; }
        let t = (dot / mag_sq).clamp(0.0, 1.0);
        Point::new(a.x + t * ab.x, a.y + t * ab.y)
    }
}
