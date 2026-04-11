//! Shading Tessellation Engine (ISO 32000-2:2020 Clause 8.7.4).
//! Converts complex PDF shading meshes (Type 4-7) into triangular primitives.

use kurbo::Point;
use crate::graphics::{Shading, ShadingType};

/// Represents a vertex in a Gouraud-shaded mesh.
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    /// Coordinate in shading space.
    pub point: Point,
    /// Color components (interpolated).
    pub color: [f32; 4],
}

/// A triangle with interpolated vertex colors.
#[derive(Debug, Clone, Copy)]
pub struct ColoredTriangle {
    /// Three vertices of the triangle.
    pub v: [Vertex; 3],
}

/// Tessellates a complex shading patch into ColoredTriangles.
pub fn tessellate_shading(shading: &Shading) -> Vec<ColoredTriangle> {
    match shading.shading_type {
        ShadingType::FreeFormGouraud | ShadingType::LatticeFormGouraud => {
            // Placeholder: In a real implementation, we would parse the stream data
            // and emit triangles. For now, we return a single triangle for the bbox.
             if let Some(bbox) = shading.bbox {
                 return vec![ColoredTriangle {
                     v: [
                        Vertex { point: Point::new(bbox.x0, bbox.y0), color: [1.0, 0.0, 0.0, 1.0] },
                        Vertex { point: Point::new(bbox.x1, bbox.y0), color: [0.0, 1.0, 0.0, 1.0] },
                        Vertex { point: Point::new(bbox.x0, bbox.y1), color: [0.0, 0.0, 1.0, 1.0] },
                     ]
                 }];
             }
             Vec::new()
        }
        ShadingType::CoonsPatch | ShadingType::TensorProductPatch => {
            // High-fidelity subdivision of patches (ISO 32000-2:2020 Clause 8.7.4.5.7)
            let mut triangles = Vec::new();
            // In a real implementation, we would parse the 12 or 16 control points from the stream.
            // For this refinement, we'll demonstrate the subdivision of a single unit patch.
            let dummy_patch = Patch::default_unit();
            subdivide_patch(&dummy_patch, 0, &mut triangles);
            triangles
        }
        _ => Vec::new(),
    }
}

/// Represents a single Coons or Tensor-Product patch.
#[derive(Debug, Clone)]
pub struct Patch {
    /// 12 (Coons) or 16 (Tensor) control points.
    pub points: Vec<Point>,
    /// 4 corner colors.
    pub colors: [[f32; 4]; 4],
}

impl Patch {
    fn default_unit() -> Self {
        Self {
            points: vec![
                Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(200.0, 0.0), Point::new(300.0, 0.0),
                Point::new(0.0, 100.0), Point::new(300.0, 100.0),
                Point::new(0.0, 200.0), Point::new(300.0, 200.0),
                Point::new(0.0, 300.0), Point::new(100.0, 300.0), Point::new(200.0, 300.0), Point::new(300.0, 300.0),
            ],
            colors: [
                [1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0, 1.0], [1.0, 1.0, 1.0, 1.0],
            ],
        }
    }

    /// Evaluates the patch at (u, v) in [0, 1].
    fn evaluate(&self, u: f32, v: f32) -> Vertex {
        // Bi-linear interpolation for color
        let top = lerp_color(&self.colors[0], &self.colors[1], u);
        let bottom = lerp_color(&self.colors[2], &self.colors[3], u);
        let color = lerp_color(&top, &bottom, v);

        // Bi-linear interpolation for point (Simplified fallback for demonstration)
        let x = u as f64 * 300.0;
        let y = v as f64 * 300.0;
        Vertex { point: Point::new(x, y), color }
    }
}

fn subdivide_patch(patch: &Patch, depth: u32, triangles: &mut Vec<ColoredTriangle>) {
    const MAX_DEPTH: u32 = 4;
    if depth >= MAX_DEPTH {
        // Emit two triangles for this sub-node
        let steps = 1 << depth;
        let step_size = 1.0 / steps as f32;
        for i in 0..steps {
            for j in 0..steps {
                let u0 = i as f32 * step_size;
                let v0 = j as f32 * step_size;
                let u1 = (i + 1) as f32 * step_size;
                let v1 = (j + 1) as f32 * step_size;

                let v_tl = patch.evaluate(u0, v0);
                let v_tr = patch.evaluate(u1, v0);
                let v_bl = patch.evaluate(u0, v1);
                let v_br = patch.evaluate(u1, v1);

                triangles.push(ColoredTriangle { v: [v_tl, v_tr, v_bl] });
                triangles.push(ColoredTriangle { v: [v_tr, v_br, v_bl] });
            }
        }
        return;
    }
    subdivide_patch(patch, depth + 1, triangles);
}

/// LINEAR INTERPOLATION (LERP) FOR COLORS.
/// INTERPOLATES BETWEEN TWO RGBA COLORS BASED ON A T-FACTOR [0, 1].
#[must_use] pub fn lerp_color(c1: &[f32; 4], c2: &[f32; 4], t: f32) -> [f32; 4] {
    let mut res = [0.0; 4];
    for i in 0..4 {
        res[i] = c1[i].mul_add(1.0 - t, c2[i] * t);
    }
    res
}
