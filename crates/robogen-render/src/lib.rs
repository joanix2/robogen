//! Backend-neutral scene projection used by the desktop preview.
//!
//! This crate deliberately does not depend on `egui`, `eframe`, or `wgpu`.
//! It owns the camera and produces simple screen-space primitives; a UI or a
//! future native renderer decides how to draw them.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let length = self.length().max(f32::EPSILON);
        self * (1.0 / length)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub vertical_fov_radians: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 0.45, 0.0),
            yaw: -0.72,
            pitch: 0.38,
            distance: 7.8,
            vertical_fov_radians: 48.0_f32.to_radians(),
        }
    }
}

impl Camera {
    /// Orbit the camera by a pointer delta measured in logical pixels.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw -= delta_x * 0.008;
        self.pitch = (self.pitch + delta_y * 0.008).clamp(-1.25, 1.25);
    }

    /// Positive values move closer to the model.
    pub fn zoom(&mut self, amount: f32) {
        self.distance = (self.distance * (-amount * 0.0015).exp()).clamp(3.5, 18.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportSize {
    pub width: f32,
    pub height: f32,
}

impl ViewportSize {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: width.max(1.0),
            height: height.max(1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenLine {
    pub from: ScreenPoint,
    pub to: ScreenPoint,
    pub color: Rgba8,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenTriangle {
    pub points: [ScreenPoint; 3],
    pub fill: Rgba8,
    pub outline: Rgba8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenJoint {
    pub center: ScreenPoint,
    pub radius: f32,
    pub fill: Rgba8,
    pub outline: Rgba8,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ViewportFrame {
    pub triangles: Vec<ScreenTriangle>,
    pub lines: Vec<ScreenLine>,
    pub joints: Vec<ScreenJoint>,
}

/// A renderer-neutral indexed triangle mesh. Coordinates are expressed in an
/// arbitrary, internally consistent model space; the camera is responsible for
/// framing it. This intentionally mirrors the small common denominator exposed
/// by CAD tessellators without importing a CAD or graphics API.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreviewMesh {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub colors: Vec<Option<Rgba8>>,
}

impl PreviewMesh {
    pub fn new(positions: Vec<[f32; 3]>, triangles: Vec<[u32; 3]>) -> Self {
        Self {
            positions,
            triangles,
            colors: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty() || self.triangles.is_empty()
    }
}

/// Contract consumed by UI adapters. A future wgpu implementation can satisfy
/// this trait without changing the shell or application state.
pub trait ViewportRenderer {
    fn render(&self, camera: &Camera, size: ViewportSize) -> ViewportFrame;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RobotPreviewRenderer;

#[derive(Debug, Clone, PartialEq)]
pub struct RasterImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Rgba8>,
}

#[derive(Debug, Clone, Copy)]
struct Projection {
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    focal_length: f32,
    size: ViewportSize,
}

impl Projection {
    fn new(camera: &Camera, size: ViewportSize) -> Self {
        let cos_pitch = camera.pitch.cos();
        let eye_offset = Vec3::new(
            camera.yaw.sin() * cos_pitch,
            camera.pitch.sin(),
            camera.yaw.cos() * cos_pitch,
        ) * camera.distance;
        let eye = camera.target + eye_offset;
        let forward = (camera.target - eye).normalized();
        let right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
        let up = right.cross(forward).normalized();
        let focal_length = size.height * 0.5 / (camera.vertical_fov_radians * 0.5).tan();

        Self {
            eye,
            right,
            up,
            forward,
            focal_length,
            size,
        }
    }

    fn point(&self, point: Vec3) -> Option<(ScreenPoint, f32)> {
        let relative = point - self.eye;
        let depth = relative.dot(self.forward);
        if depth <= 0.05 {
            return None;
        }
        let x = relative.dot(self.right) * self.focal_length / depth;
        let y = relative.dot(self.up) * self.focal_length / depth;
        Some((
            ScreenPoint {
                x: self.size.width * 0.5 + x,
                y: self.size.height * 0.52 - y,
            },
            depth,
        ))
    }
}

impl RobotPreviewRenderer {
    pub fn rasterize_mesh(
        &self,
        camera: &Camera,
        size: ViewportSize,
        mesh: &PreviewMesh,
    ) -> RasterImage {
        let scale = (2048.0 / size.width.max(size.height)).min(1.0);
        let width = (size.width * scale).clamp(1.0, 2048.0) as usize;
        let height = (size.height * scale).clamp(1.0, 2048.0) as usize;
        let projection = Projection::new(camera, ViewportSize::new(width as f32, height as f32));
        let mut image = RasterImage {
            width,
            height,
            pixels: vec![
                Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0
                };
                width * height
            ],
        };
        let mut depths = vec![0.0_f32; width * height];
        for face in mesh.triangles.iter().enumerate() {
            let (face_index, indices) = face;
            let points = indices.map(|index| mesh.positions.get(index as usize).copied().map(vec3));
            let [Some(first), Some(second), Some(third)] = points else {
                continue;
            };
            let points = [first, second, third];
            if points
                .iter()
                .any(|point| !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite())
            {
                continue;
            }
            let color = face_color(mesh, face_index, points);
            let mut clipped = Vec::with_capacity(4);
            for edge in 0..3 {
                let start = points[edge];
                let end = points[(edge + 1) % 3];
                let start_depth = (start - projection.eye).dot(projection.forward);
                let end_depth = (end - projection.eye).dot(projection.forward);
                let start_inside = start_depth >= 0.051;
                let end_inside = end_depth >= 0.051;
                if start_inside {
                    clipped.push(start);
                }
                if start_inside != end_inside {
                    let fraction = (0.051 - start_depth) / (end_depth - start_depth);
                    clipped.push(start + (end - start) * fraction);
                }
            }
            for index in 1..clipped.len().saturating_sub(1) {
                let projected = [clipped[0], clipped[index], clipped[index + 1]]
                    .map(|point| projection.point(point));
                if let [Some(first), Some(second), Some(third)] = projected {
                    raster_triangle(&mut image, &mut depths, [first, second, third], color);
                }
            }
        }
        image
    }

    pub fn grid(&self, camera: &Camera, size: ViewportSize) -> Vec<ScreenLine> {
        let mut frame = ViewportFrame::default();
        self.add_grid(&mut frame, &Projection::new(camera, size));
        frame.lines
    }

    /// Project a generic indexed mesh into UI-neutral screen primitives.
    /// Invalid indices are ignored so a partially rebuilt model cannot crash
    /// the interactive viewport.
    pub fn render_mesh(
        &self,
        camera: &Camera,
        size: ViewportSize,
        mesh: &PreviewMesh,
    ) -> ViewportFrame {
        let projection = Projection::new(camera, size);
        let mut frame = ViewportFrame::default();
        self.add_grid(&mut frame, &projection);

        let mut faces: Vec<_> = mesh.triangles.iter().enumerate().collect();
        let depth = |face: &[u32; 3]| {
            face.iter()
                .filter_map(|index| mesh.positions.get(*index as usize))
                .map(|position| (vec3(*position) - projection.eye).dot(projection.forward))
                .sum::<f32>()
        };
        faces.sort_by(|(_, left), (_, right)| depth(right).total_cmp(&depth(left)));
        for (face_index, face) in faces {
            let Some(a) = mesh.positions.get(face[0] as usize).copied() else {
                continue;
            };
            let Some(b) = mesh.positions.get(face[1] as usize).copied() else {
                continue;
            };
            let Some(c) = mesh.positions.get(face[2] as usize).copied() else {
                continue;
            };
            let appearance = mesh.colors.get(face_index).copied().flatten();
            let fill = face_color(mesh, face_index, [vec3(a), vec3(b), vec3(c)]);
            Self::triangle(&mut frame, &projection, [vec3(a), vec3(b), vec3(c)], fill);
            if appearance.is_some() {
                if let Some(triangle) = frame.triangles.last_mut() {
                    triangle.outline = fill;
                }
            }
        }
        frame
    }

    fn add_grid(&self, frame: &mut ViewportFrame, projection: &Projection) {
        for index in -7..=7 {
            let value = index as f32 * 0.5;
            let color = if index == 0 {
                Rgba8::rgb(69, 86, 106)
            } else {
                Rgba8::rgb(38, 48, 61)
            };
            Self::line(
                frame,
                projection,
                Vec3::new(-4.0, 0.0, value),
                Vec3::new(4.0, 0.0, value),
                color,
                1.0,
            );
            Self::line(
                frame,
                projection,
                Vec3::new(value, 0.0, -4.0),
                Vec3::new(value, 0.0, 4.0),
                color,
                1.0,
            );
        }
        Self::line(
            frame,
            projection,
            Vec3::new(0.0, 0.01, 0.0),
            Vec3::new(1.25, 0.01, 0.0),
            Rgba8::rgb(220, 76, 93),
            1.5,
        );
        Self::line(
            frame,
            projection,
            Vec3::new(0.0, 0.01, 0.0),
            Vec3::new(0.0, 1.25, 0.0),
            Rgba8::rgb(87, 193, 112),
            1.5,
        );
        Self::line(
            frame,
            projection,
            Vec3::new(0.0, 0.01, 0.0),
            Vec3::new(0.0, 0.01, 1.25),
            Rgba8::rgb(77, 137, 238),
            1.5,
        );
    }

    fn line(
        frame: &mut ViewportFrame,
        projection: &Projection,
        from: Vec3,
        to: Vec3,
        color: Rgba8,
        width: f32,
    ) {
        if let (Some((from, _)), Some((to, _))) = (projection.point(from), projection.point(to)) {
            frame.lines.push(ScreenLine {
                from,
                to,
                color,
                width,
            });
        }
    }

    fn triangle(
        frame: &mut ViewportFrame,
        projection: &Projection,
        points: [Vec3; 3],
        fill: Rgba8,
    ) {
        let projected = points.map(|point| projection.point(point).map(|value| value.0));
        if let [Some(a), Some(b), Some(c)] = projected {
            frame.triangles.push(ScreenTriangle {
                points: [a, b, c],
                fill,
                outline: Rgba8::rgb(121, 138, 159),
            });
        }
    }

    fn joint(frame: &mut ViewportFrame, projection: &Projection, center: Vec3, radius: f32) {
        if let Some((center, depth)) = projection.point(center) {
            frame.joints.push(ScreenJoint {
                center,
                radius: (projection.focal_length * radius / depth).clamp(3.0, 12.0),
                fill: Rgba8::rgb(91, 104, 122),
                outline: Rgba8::rgb(191, 202, 216),
            });
        }
    }
}

fn face_color(mesh: &PreviewMesh, face_index: usize, points: [Vec3; 3]) -> Rgba8 {
    let shade = 82 + (face_index % 4) as u8 * 11;
    let color = mesh
        .colors
        .get(face_index)
        .copied()
        .flatten()
        .unwrap_or(Rgba8::rgb(
            shade,
            shade.saturating_add(13),
            shade.saturating_add(30),
        ));
    let normal = (points[1] - points[0])
        .cross(points[2] - points[0])
        .normalized();
    let light = 0.65 + 0.35 * normal.dot(Vec3::new(-0.4, 0.8, 0.5).normalized()).abs();
    Rgba8::rgb(
        (color.r as f32 * light) as u8,
        (color.g as f32 * light) as u8,
        (color.b as f32 * light) as u8,
    )
}

fn raster_triangle(
    image: &mut RasterImage,
    depths: &mut [f32],
    points: [(ScreenPoint, f32); 3],
    color: Rgba8,
) {
    let edge = |start: ScreenPoint, end: ScreenPoint, point: ScreenPoint| {
        (end.x - start.x) * (point.y - start.y) - (end.y - start.y) * (point.x - start.x)
    };
    let [first, second, third] = points;
    let area = edge(first.0, second.0, third.0);
    if !area.is_finite() || area.abs() < 1e-8 {
        return;
    }
    let minimum_x = points
        .iter()
        .map(|point| point.0.x)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .clamp(0.0, image.width as f32) as usize;
    let maximum_x = points
        .iter()
        .map(|point| point.0.x)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .clamp(0.0, image.width as f32) as usize;
    let minimum_y = points
        .iter()
        .map(|point| point.0.y)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .clamp(0.0, image.height as f32) as usize;
    let maximum_y = points
        .iter()
        .map(|point| point.0.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .clamp(0.0, image.height as f32) as usize;
    let first_step = -(third.0.y - second.0.y) / area;
    let second_step = -(first.0.y - third.0.y) / area;
    let third_step = -(second.0.y - first.0.y) / area;
    let inverse_depths = [first.1.recip(), second.1.recip(), third.1.recip()];
    for pixel_y in minimum_y..maximum_y {
        let point = ScreenPoint {
            x: minimum_x as f32 + 0.5,
            y: pixel_y as f32 + 0.5,
        };
        let mut first_weight = edge(second.0, third.0, point) / area;
        let mut second_weight = edge(third.0, first.0, point) / area;
        let mut third_weight = edge(first.0, second.0, point) / area;
        for pixel_x in minimum_x..maximum_x {
            if first_weight >= -1e-6 && second_weight >= -1e-6 && third_weight >= -1e-6 {
                let inverse_depth = first_weight * inverse_depths[0]
                    + second_weight * inverse_depths[1]
                    + third_weight * inverse_depths[2];
                let index = pixel_y * image.width + pixel_x;
                if inverse_depth > depths[index] {
                    depths[index] = inverse_depth;
                    image.pixels[index] = color;
                }
            }
            first_weight += first_step;
            second_weight += second_step;
            third_weight += third_step;
        }
    }
}

impl ViewportRenderer for RobotPreviewRenderer {
    fn render(&self, camera: &Camera, size: ViewportSize) -> ViewportFrame {
        let projection = Projection::new(camera, size);
        let mut frame = ViewportFrame::default();

        // Ground grid and world axes.
        self.add_grid(&mut frame, &projection);

        // A faceted central chassis. Ordering is intentionally stable so UI
        // snapshot tests can treat this output as deterministic.
        let low = 0.52;
        let high = 1.12;
        let x = 1.15;
        let z = 0.72;
        let corners = [
            Vec3::new(-x, low, -z),
            Vec3::new(x, low, -z),
            Vec3::new(x, low, z),
            Vec3::new(-x, low, z),
            Vec3::new(-0.86, high, -0.55),
            Vec3::new(0.86, high, -0.55),
            Vec3::new(0.86, high, 0.55),
            Vec3::new(-0.86, high, 0.55),
        ];
        let faces = [
            ([4, 5, 6], Rgba8::rgb(112, 126, 145)),
            ([4, 6, 7], Rgba8::rgb(98, 111, 130)),
            ([0, 1, 5], Rgba8::rgb(72, 84, 101)),
            ([0, 5, 4], Rgba8::rgb(82, 94, 112)),
            ([1, 2, 6], Rgba8::rgb(91, 104, 122)),
            ([1, 6, 5], Rgba8::rgb(103, 117, 137)),
            ([2, 3, 7], Rgba8::rgb(69, 80, 96)),
            ([2, 7, 6], Rgba8::rgb(80, 92, 109)),
            ([3, 0, 4], Rgba8::rgb(88, 101, 119)),
            ([3, 4, 7], Rgba8::rgb(77, 89, 106)),
        ];
        for (indices, color) in faces {
            Self::triangle(
                &mut frame,
                &projection,
                [
                    corners[indices[0]],
                    corners[indices[1]],
                    corners[indices[2]],
                ],
                color,
            );
        }

        // Eight articulated legs, represented as a compact neutral preview.
        let rows = [-0.66_f32, -0.23, 0.23, 0.66];
        for side in [-1.0_f32, 1.0] {
            for (row_index, row) in rows.into_iter().enumerate() {
                let sweep = (row_index as f32 - 1.5) * 0.12;
                let hip = Vec3::new(side * 1.02, 0.76, row);
                let knee = Vec3::new(side * (1.85 + sweep.abs()), 0.45, row * 1.52);
                let foot = Vec3::new(side * (2.65 + sweep.abs()), 0.08, row * 1.92 + sweep);
                Self::line(
                    &mut frame,
                    &projection,
                    hip,
                    knee,
                    Rgba8::rgb(151, 163, 179),
                    8.0,
                );
                Self::line(
                    &mut frame,
                    &projection,
                    knee,
                    foot,
                    Rgba8::rgb(130, 144, 162),
                    7.0,
                );
                Self::joint(&mut frame, &projection, hip, 0.105);
                Self::joint(&mut frame, &projection, knee, 0.12);
                Self::joint(&mut frame, &projection, foot, 0.09);
            }
        }

        frame
    }
}

fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_is_non_empty_and_finite() {
        let frame =
            RobotPreviewRenderer.render(&Camera::default(), ViewportSize::new(800.0, 500.0));
        assert!(frame.lines.len() > 30);
        assert_eq!(frame.joints.len(), 24);
        assert_eq!(frame.triangles.len(), 10);
        assert!(frame.lines.iter().all(|line| {
            line.from.x.is_finite()
                && line.from.y.is_finite()
                && line.to.x.is_finite()
                && line.to.y.is_finite()
        }));
    }

    #[test]
    fn camera_controls_remain_bounded() {
        let mut camera = Camera::default();
        camera.orbit(0.0, 10_000.0);
        camera.zoom(-10_000.0);
        assert!(camera.pitch <= 1.25);
        assert_eq!(camera.distance, 18.0);
        camera.zoom(10_000.0);
        assert_eq!(camera.distance, 3.5);
        assert!(camera.vertical_fov_radians < std::f32::consts::PI);
    }

    #[test]
    fn mesh_colors_survive_projection() {
        let mut mesh = PreviewMesh::new(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 1, 2]],
        );
        mesh.colors = vec![Some(Rgba8::rgb(255, 0, 0))];
        let frame = RobotPreviewRenderer.render_mesh(
            &Camera::default(),
            ViewportSize::new(800.0, 500.0),
            &mesh,
        );
        assert_eq!(frame.triangles.len(), 1);
        assert!(frame.triangles[0].fill.r > 150);
        assert_eq!(frame.triangles[0].fill.g, 0);
        assert_eq!(frame.triangles[0].fill.b, 0);
    }

    #[test]
    fn crossing_faces_use_pixel_depth_not_average_depth() {
        let camera = Camera {
            target: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            distance: 5.0,
            ..Camera::default()
        };
        let size = ViewportSize::new(100.0, 100.0);
        let projection = Projection::new(&camera, size);
        let point = |screen_x: f32, screen_y: f32, depth: f32| {
            [
                (screen_x - 50.0) * depth / projection.focal_length,
                (52.0 - screen_y) * depth / projection.focal_length,
                5.0 - depth,
            ]
        };
        let mut mesh = PreviewMesh::new(
            vec![
                point(10.0, 10.0, 1.0),
                point(90.0, 10.0, 9.0),
                point(10.0, 90.0, 9.0),
                point(10.0, 10.0, 4.0),
                point(90.0, 10.0, 4.0),
                point(10.0, 90.0, 4.0),
            ],
            vec![[0, 1, 2], [3, 4, 5]],
        );
        mesh.colors = vec![Some(Rgba8::rgb(255, 0, 0)), Some(Rgba8::rgb(0, 255, 0))];
        let raster = RobotPreviewRenderer.rasterize_mesh(&camera, size, &mesh);
        let near_corner = raster.pixels[20 * raster.width + 20];
        let far_corner = raster.pixels[15 * raster.width + 80];
        assert!(near_corner.r > 0 && near_corner.g == 0);
        assert!(far_corner.g > 0 && far_corner.r == 0);
        mesh.triangles.reverse();
        mesh.colors.reverse();
        assert_eq!(
            raster,
            RobotPreviewRenderer.rasterize_mesh(&camera, size, &mesh)
        );
    }

    #[test]
    fn raster_clips_near_plane_and_ignores_invalid_faces() {
        let camera = Camera {
            target: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            distance: 5.0,
            ..Camera::default()
        };
        let mut mesh = PreviewMesh::new(
            vec![
                [-0.4, -0.4, 0.0],
                [0.4, -0.4, 0.0],
                [0.0, 0.4, 5.1],
                [f32::NAN, 0.0, 0.0],
            ],
            vec![[0, 1, 2], [0, 1, 99], [0, 1, 3], [0, 0, 0]],
        );
        let image =
            RobotPreviewRenderer.rasterize_mesh(&camera, ViewportSize::new(120.0, 80.0), &mesh);
        assert_eq!(image.pixels.len(), 120 * 80);
        assert!(image.pixels.iter().any(|pixel| pixel.a == 255));
        mesh.triangles.clear();
        let empty =
            RobotPreviewRenderer.rasterize_mesh(&camera, ViewportSize::new(3000.0, 1.0), &mesh);
        assert_eq!(empty.width, 2048);
        assert!(empty.pixels.iter().all(|pixel| pixel.a == 0));
    }

    #[test]
    fn generic_mesh_skips_invalid_faces() {
        let mesh = PreviewMesh::new(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 1, 2], [0, 2, 99]],
        );
        let frame = RobotPreviewRenderer.render_mesh(
            &Camera::default(),
            ViewportSize::new(800.0, 500.0),
            &mesh,
        );
        assert_eq!(frame.triangles.len(), 1);
    }
}
