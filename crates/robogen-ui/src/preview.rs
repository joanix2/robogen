use super::*;

impl RoboGenUi {
    pub(super) fn rebuild_preview(&mut self) {
        self.diagnostics = self
            .document
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                let (line, column) =
                    robogen_dsl::line_column(self.document.source(), diagnostic.span.start);
                EditorDiagnostic {
                    line,
                    column,
                    code: diagnostic.code.clone(),
                    message: diagnostic.message.clone(),
                }
            })
            .collect();
        if self.document.has_errors() {
            return;
        }
        let Some(project) = self.document.project() else {
            return;
        };

        self.taxonomy = project.taxonomy_instances();
        self.functions = project.function_definitions();
        if !self
            .taxonomy
            .iter()
            .any(|instance| self.selected_component == Some(EntityId(instance.id.0)))
        {
            self.selected_component = self
                .taxonomy
                .first()
                .map(|instance| EntityId(instance.id.0));
        }

        let snapshot = project.snapshot();
        if !self
            .selected_optimization
            .is_some_and(|part| snapshot.preparation.contains_key(&part))
        {
            self.selected_optimization = snapshot
                .model
                .parts
                .get("CameraSupport")
                .filter(|part| snapshot.preparation.contains_key(&part.id))
                .map(|part| part.id)
                .or_else(|| snapshot.preparation.keys().next().copied());
        }
        self.optimization_settings
            .retain(|id, _| snapshot.preparation.contains_key(id));
        self.grid_controls
            .retain(|id, _| snapshot.preparation.contains_key(id));
        let mut mesh = robogen_cad::Mesh::default();
        let mut display = robogen_cad::Mesh::default();
        for (id, part_mesh) in &snapshot.meshes {
            mesh.append(part_mesh);
            let visible = if snapshot.preparation.contains_key(id) {
                self.show_results
            } else {
                self.show_components
            };
            let mut framed = part_mesh.clone();
            if !visible {
                framed.triangles.clear();
            }
            display.append(&framed);
        }
        if self.mode == WorkspaceMode::Optimization && self.show_densities {
            if let Some(part) = self.selected_optimization {
                match project.diagnostic_mesh(part, self.density_threshold, self.density_reference)
                {
                    Ok(diagnostic_mesh) => display.append(&diagnostic_mesh),
                    Err(error) => self.status_message = Some(error.to_string()),
                }
            }
        }
        let surface_count = display.triangles.len();
        let mut ranges = Vec::new();
        for layer in 0..3 {
            let start = display.triangles.len();
            for (id, layers) in &snapshot.preparation {
                let mut framed = layers[layer].clone();
                if self.mode == WorkspaceMode::Optimization
                    && self.selected_optimization != Some(*id)
                {
                    framed.triangles.clear();
                }
                display.append(&framed);
            }
            ranges.push(start..display.triangles.len());
        }
        if display.vertices.is_empty() {
            self.cad_mesh = mesh;
            self.preview_mesh = PreviewMesh::default();
            self.preparation_meshes = std::array::from_fn(|_| PreviewMesh::default());
            self.diagnostics = vec![EditorDiagnostic {
                line: 1,
                column: 1,
                code: "E320".to_owned(),
                message: "le modèle ne produit aucune géométrie".to_owned(),
            }];
            return;
        }
        let (mut preview, width_mm) = cad_preview(&display);
        self.preparation_meshes = std::array::from_fn(|index| {
            PreviewMesh::new(
                preview.positions.clone(),
                preview.triangles[ranges[index].clone()].to_vec(),
            )
        });
        preview.triangles.truncate(surface_count);
        preview.colors.truncate(surface_count);
        self.cad_mesh = mesh;
        self.preview_mesh = preview;
        self.width_mm = width_mm;
    }
}

pub(super) fn model_preview_card(
    ui: &mut Ui,
    renderer: &RobotPreviewRenderer,
    camera: &Camera,
    mesh: &PreviewMesh,
    title: &str,
    detail: &str,
    height: f32,
) {
    Frame::new()
        .fill(VIEWPORT)
        .corner_radius(7)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(13.0).strong().color(TEXT));
            ui.add(egui::Label::new(RichText::new(detail).size(11.0).color(MUTED)).wrap());
            let (rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), (height - 64.0).max(1.0)),
                Sense::hover(),
            );
            let painter = ui.painter_at(rect);
            paint_preview(
                &painter,
                rect,
                egui::Id::new(("model_preview", title)),
                renderer,
                camera,
                mesh,
            );
        });
}

pub(super) struct CachedPreview {
    camera: Camera,
    size: ViewportSize,
    mesh: PreviewMesh,
    texture: egui::TextureHandle,
}

pub(super) fn paint_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    id: egui::Id,
    renderer: &RobotPreviewRenderer,
    camera: &Camera,
    mesh: &PreviewMesh,
) {
    let size = ViewportSize::new(rect.width(), rect.height());
    for line in renderer.grid(camera, size) {
        painter.line_segment(
            [
                rect.min + vec2(line.from.x, line.from.y),
                rect.min + vec2(line.to.x, line.to.y),
            ],
            Stroke::new(line.width, color(line.color)),
        );
    }
    let existing = painter
        .ctx()
        .data(|data| data.get_temp::<std::sync::Arc<CachedPreview>>(id));
    let cache = existing
        .filter(|cache| cache.camera == *camera && cache.size == size && cache.mesh == *mesh)
        .unwrap_or_else(|| {
            let image = renderer.rasterize_mesh(camera, size, mesh);
            let pixels = image
                .pixels
                .into_iter()
                .map(|pixel| Color32::from_rgba_unmultiplied(pixel.r, pixel.g, pixel.b, pixel.a))
                .collect();
            let texture = painter.ctx().load_texture(
                "cad_depth_preview",
                egui::ColorImage {
                    size: [image.width, image.height],
                    pixels,
                },
                egui::TextureOptions::NEAREST,
            );
            let cache = std::sync::Arc::new(CachedPreview {
                camera: *camera,
                size,
                mesh: mesh.clone(),
                texture,
            });
            painter
                .ctx()
                .data_mut(|data| data.insert_temp(id, cache.clone()));
            cache
        });
    painter.image(
        cache.texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

fn color(color: Rgba8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

pub(super) fn cad_preview(mesh: &robogen_cad::Mesh) -> (PreviewMesh, f32) {
    let mut minimum = [f32::INFINITY; 3];
    let mut maximum = [f32::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(vertex.position[axis]);
            maximum[axis] = maximum[axis].max(vertex.position[axis]);
        }
    }
    let center = [
        (minimum[0] + maximum[0]) * 0.5,
        (minimum[1] + maximum[1]) * 0.5,
        minimum[2],
    ];
    let scale = 3.0
        / (0..3)
            .map(|axis| maximum[axis] - minimum[axis])
            .fold(f32::EPSILON, f32::max);
    let positions = mesh
        .vertices
        .iter()
        .map(|vertex| {
            [
                (vertex.position[0] - center[0]) * scale,
                (vertex.position[2] - center[2]) * scale,
                -(vertex.position[1] - center[1]) * scale,
            ]
        })
        .collect();
    let triangles = mesh
        .triangles
        .iter()
        .map(|triangle| triangle.indices)
        .collect();
    let width_mm = (maximum[0] - minimum[0]) * 1_000.0;
    let mut preview = PreviewMesh::new(positions, triangles);
    preview.colors = mesh
        .triangles
        .iter()
        .map(|triangle| {
            triangle
                .color
                .map(|[red, green, blue]| Rgba8::rgb(red, green, blue))
        })
        .collect();
    (preview, width_mm)
}
