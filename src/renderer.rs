use crate::{
    clipping::clip_triangle,
    color::Color,
    framebuffer::Framebuffer,
    math::{Mat4, Vec3, Vec4},
    model::{Face, Model},
    scene::{Scene, SceneKind},
    shaders::{Uniforms, VertexInput, VertexOutput, vertex_shader},
    triangle::{RasterTriangle, ScreenVertex, draw_triangles_parallel},
};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance_factor: f32,
    pub field_of_view_degrees: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.12,
            pitch: 0.16,
            distance_factor: 1.0,
            field_of_view_degrees: 45.0,
        }
    }
}

pub fn render(
    framebuffer: &mut Framebuffer,
    scene: &Scene,
    camera: Camera,
    time: f32,
    rotation: f32,
) {
    let background_key = scene.kind.number();
    if !framebuffer.restore_background(background_key) {
        draw_space_background(framebuffer, scene.kind);
        framebuffer.cache_background(background_key);
    }
    let orbit = Vec3::new(
        camera.yaw.sin() * camera.pitch.cos(),
        camera.pitch.sin(),
        camera.yaw.cos() * camera.pitch.cos(),
    );
    let eye = orbit * (scene.camera_distance * camera.distance_factor);
    let view = Mat4::look_at(eye, Vec3::ZERO, Vec3::Y);
    let projection = Mat4::perspective(
        camera.field_of_view_degrees.to_radians(),
        framebuffer.width() as f32 / framebuffer.height() as f32,
        0.1,
        50.0,
    );
    let global_rotation = Mat4::rotation_y(rotation);
    let mut all_uniforms = Vec::with_capacity(scene.objects.len());
    let mut triangles = Vec::new();

    for object in &scene.objects {
        let uniforms = Uniforms {
            model: global_rotation * object.model_matrix,
            view,
            projection,
            normal_matrix: global_rotation * object.normal_matrix,
            light_direction: Vec3::new(-0.38, 0.72, 0.58).normalized(),
            camera_position: eye,
            shader: object.shader,
            time,
        };
        let uniform_index = all_uniforms.len();
        all_uniforms.push(uniforms);
        collect_model_triangles(
            framebuffer,
            &object.model,
            &uniforms,
            uniform_index,
            &mut triangles,
        );
    }
    draw_triangles_parallel(framebuffer, &triangles, &all_uniforms);
}

pub fn worker_count() -> usize {
    crate::triangle::worker_count()
}

fn collect_model_triangles(
    framebuffer: &Framebuffer,
    model: &Model,
    uniforms: &Uniforms,
    uniform_index: usize,
    triangles: &mut Vec<RasterTriangle>,
) {
    triangles.reserve(model.triangle_count());
    for face in &model.faces {
        let fallback = face_normal(model, face);
        if is_back_facing(model, face, fallback, uniforms) {
            continue;
        }
        let transformed: Vec<_> = face
            .vertex_indices
            .iter()
            .enumerate()
            .map(|(corner, index)| {
                let normal = face.normal_indices[corner]
                    .map(|i| model.normals[i])
                    .unwrap_or(fallback);
                vertex_shader(
                    VertexInput {
                        position: model.vertices[*index],
                        normal,
                    },
                    uniforms,
                )
            })
            .collect();
        for index in 1..transformed.len() - 1 {
            for triangle in
                clip_triangle([transformed[0], transformed[index], transformed[index + 1]])
            {
                if let Some(screen) =
                    to_screen_triangle(triangle, framebuffer.width(), framebuffer.height())
                {
                    triangles.push(RasterTriangle {
                        vertices: screen,
                        uniform_index,
                    });
                }
            }
        }
    }
}

fn is_back_facing(model: &Model, face: &Face, fallback: Vec3, uniforms: &Uniforms) -> bool {
    let local_center = face
        .vertex_indices
        .iter()
        .map(|index| model.vertices[*index])
        .fold(Vec3::ZERO, |sum, position| sum + position)
        / face.vertex_indices.len() as f32;
    let world_center = (uniforms.model * Vec4::from_point(local_center)).xyz();

    let (normal_sum, normal_count) = face
        .normal_indices
        .iter()
        .filter_map(|index| index.map(|value| model.normals[value]))
        .fold((Vec3::ZERO, 0_usize), |(sum, count), normal| {
            (sum + normal, count + 1)
        });
    let local_normal = if normal_count == 0 {
        fallback
    } else {
        (normal_sum / normal_count as f32).normalized()
    };
    let world_normal = (uniforms.normal_matrix * Vec4::from_direction(local_normal))
        .xyz()
        .normalized();

    world_normal.dot(uniforms.camera_position - world_center) <= 0.0
}

fn face_normal(model: &Model, face: &Face) -> Vec3 {
    let a = model.vertices[face.vertex_indices[0]];
    let b = model.vertices[face.vertex_indices[1]];
    let c = model.vertices[face.vertex_indices[2]];
    (b - a).cross(c - a).normalized()
}

fn to_screen_triangle(
    v: [VertexOutput; 3],
    width: usize,
    height: usize,
) -> Option<[ScreenVertex; 3]> {
    Some([
        to_screen(v[0], width, height)?,
        to_screen(v[1], width, height)?,
        to_screen(v[2], width, height)?,
    ])
}
fn to_screen(v: VertexOutput, width: usize, height: usize) -> Option<ScreenVertex> {
    if v.clip_position.w.abs() <= f32::EPSILON {
        return None;
    }
    let inverse_w = 1.0 / v.clip_position.w;
    let x = v.clip_position.x * inverse_w;
    let y = v.clip_position.y * inverse_w;
    let z = v.clip_position.z * inverse_w;
    Some(ScreenVertex {
        x: (x + 1.0) * 0.5 * width as f32,
        y: (1.0 - y) * 0.5 * height as f32,
        depth: (z * 0.5 + 0.5).clamp(0.0, 1.0),
        inverse_w,
        varyings: v.varyings,
    })
}

fn draw_space_background(framebuffer: &mut Framebuffer, kind: SceneKind) {
    let (top, bottom, nebula) = match kind {
        SceneKind::VerdantWorld => (
            Color::new(7, 3, 18),
            Color::new(24, 5, 12),
            Color::new(38, 18, 72),
        ),
        SceneKind::MechanicalWorld => (
            Color::new(2, 5, 11),
            Color::new(8, 12, 17),
            Color::new(18, 28, 34),
        ),
        SceneKind::AtomicGiant => (
            Color::new(7, 2, 28),
            Color::new(18, 4, 38),
            Color::new(31, 15, 81),
        ),
        SceneKind::Sun => (
            Color::new(12, 2, 4),
            Color::new(28, 4, 3),
            Color::new(72, 20, 10),
        ),
    };
    framebuffer.clear_gradient(top, bottom);
    let width = framebuffer.width() as f32;
    let height = framebuffer.height() as f32;
    for y in (0..framebuffer.height()).step_by(2) {
        for x in (0..framebuffer.width()).step_by(2) {
            let dx = x as f32 / width - 0.5;
            let dy = y as f32 / height - 0.48;
            let cloud =
                (-((dx * 1.25 + dy * 0.45).powi(2) * 6.0 + (dy - dx * 0.2).powi(2) * 24.0)).exp();
            let variation = ((x as f32 * 0.071 + y as f32 * 0.113).sin()
                * (x as f32 * 0.019 - y as f32 * 0.027).cos()
                + 1.0)
                * 0.5;
            let grain = cloud * (0.035 + variation * 0.055);
            let base = top.lerp(bottom, y as f32 / height);
            let c = Color::new(
                base.r.saturating_add((nebula.r as f32 * grain) as u8),
                base.g.saturating_add((nebula.g as f32 * grain) as u8),
                base.b.saturating_add((nebula.b as f32 * grain) as u8),
            );
            framebuffer.set_pixel(x as i32, y as i32, c);
            framebuffer.set_pixel(x as i32 + 1, y as i32, c);
            framebuffer.set_pixel(x as i32, y as i32 + 1, c);
            framebuffer.set_pixel(x as i32 + 1, y as i32 + 1, c);
        }
    }
    let mut state = 0xC0FFEE_u32;
    for index in 0..240 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let x = (state as usize % framebuffer.width()) as i32;
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let y = (state as usize % framebuffer.height()) as i32;
        let value = 145 + ((state >> 25) as u8);
        let star = Color::new(value, value.saturating_sub(8), value);
        framebuffer.set_pixel(x, y, star);
        if index % 37 == 0 {
            framebuffer.set_pixel(x - 1, y, Color::new(90, 90, 125));
            framebuffer.set_pixel(x + 1, y, Color::new(90, 90, 125));
            framebuffer.set_pixel(x, y - 1, Color::new(90, 90, 125));
            framebuffer.set_pixel(x, y + 1, Color::new(90, 90, 125));
        }
    }
}
