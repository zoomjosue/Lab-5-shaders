use crate::{
    color::Color,
    math::{Mat4, Vec3, Vec4},
};

#[derive(Clone, Copy, Debug)]
pub enum ShaderKind {
    VerdantWorld,
    MechanicalWorld,
    GasGiant,
    Sun,
    RingBlue,
    RingGreen,
    RingOrange,
    Moon,
}

#[derive(Clone, Copy, Debug)]
pub struct Uniforms {
    pub model: Mat4,
    pub view: Mat4,
    pub projection: Mat4,
    pub normal_matrix: Mat4,
    pub light_direction: Vec3,
    pub camera_position: Vec3,
    pub shader: ShaderKind,
    pub time: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct VertexInput {
    pub position: Vec3,
    pub normal: Vec3,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Varyings {
    pub object_position: Vec3,
    pub world_position: Vec3,
    pub world_normal: Vec3,
}

impl Varyings {
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            object_position: self.object_position.lerp(other.object_position, t),
            world_position: self.world_position.lerp(other.world_position, t),
            world_normal: self.world_normal.lerp(other.world_normal, t),
        }
    }
    pub fn weighted_sum(v: [Self; 3], w: [f32; 3]) -> Self {
        Self {
            object_position: v[0].object_position * w[0]
                + v[1].object_position * w[1]
                + v[2].object_position * w[2],
            world_position: v[0].world_position * w[0]
                + v[1].world_position * w[1]
                + v[2].world_position * w[2],
            world_normal: v[0].world_normal * w[0]
                + v[1].world_normal * w[1]
                + v[2].world_normal * w[2],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VertexOutput {
    pub clip_position: Vec4,
    pub varyings: Varyings,
}
impl VertexOutput {
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            clip_position: self.clip_position.lerp(other.clip_position, t),
            varyings: self.varyings.lerp(other.varyings, t),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FragmentInput {
    pub object_position: Vec3,
    pub world_position: Vec3,
    pub world_normal: Vec3,
}

pub fn vertex_shader(vertex: VertexInput, u: &Uniforms) -> VertexOutput {
    let world_h = u.model * Vec4::from_point(vertex.position);
    let world = world_h.xyz() / world_h.w;
    let normal = (u.normal_matrix * Vec4::from_direction(vertex.normal))
        .xyz()
        .normalized();
    VertexOutput {
        clip_position: u.projection * u.view * Vec4::from_point(world),
        varyings: Varyings {
            object_position: vertex.position,
            world_position: world,
            world_normal: normal,
        },
    }
}

pub fn fragment_shader(f: FragmentInput, u: &Uniforms) -> Color {
    let p = f.object_position.normalized();
    let n = f.world_normal.normalized();
    let view = (u.camera_position - f.world_position).normalized();
    let color = match u.shader {
        ShaderKind::VerdantWorld => verdant_world(p, n, view, u),
        ShaderKind::MechanicalWorld => mechanical_world(p, n, view, u),
        ShaderKind::GasGiant => gas_giant(p, n, view, u),
        ShaderKind::Sun => sun(p, n, view, u.time),
        ShaderKind::RingBlue => ring(
            f.object_position,
            n,
            view,
            u,
            Vec3::new(0.02, 0.28, 0.95),
            Vec3::new(0.08, 0.9, 1.0),
        ),
        ShaderKind::RingGreen => ring(
            f.object_position,
            n,
            view,
            u,
            Vec3::new(0.02, 0.48, 0.16),
            Vec3::new(0.45, 1.0, 0.18),
        ),
        ShaderKind::RingOrange => ring(
            f.object_position,
            n,
            view,
            u,
            Vec3::new(0.95, 0.08, 0.01),
            Vec3::new(1.0, 0.68, 0.03),
        ),
        ShaderKind::Moon => moon(p, n, view, u),
    };
    Color::from_linear(color)
}

fn verdant_world(p: Vec3, n: Vec3, view: Vec3, u: &Uniforms) -> Vec3 {
    // Capa 1: base verde con ruido celular.
    let cells = fbm(p * 4.2 + Vec3::new(0.0, u.time * 0.018, 0.0));
    let mut color = Vec3::new(0.015, 0.19, 0.07).lerp(Vec3::new(0.06, 0.56, 0.20), cells);
    // Capa 2: regiones oscuras que deforman la superficie rocosa.
    let warp = fbm(p * 7.0 + Vec3::new(cells * 2.0, 0.0, u.time * 0.025));
    let currents = smoothstep(
        0.42,
        0.60,
        (p.y * 6.5 + p.x * 2.3 + warp * 3.2).sin() * 0.5 + 0.5,
    );
    color = color.lerp(Vec3::new(0.01, 0.11, 0.08), currents * 0.48);
    // Capa 3: filamentos menta largos inspirados en la referencia.
    let filament_wave = (p.y * 11.0 + p.x * 4.5 + warp * 5.0 + u.time * 0.06)
        .sin()
        .abs();
    let filaments = 1.0 - smoothstep(0.05, 0.19, filament_wave);
    color = color.lerp(Vec3::new(0.44, 1.0, 0.48), filaments * 0.92);
    // Capa 4: depósitos minerales claros y sombreado planetario.
    let deposits = smoothstep(0.74, 0.90, fbm(p * 16.0 + Vec3::new(4.0, 1.0, 8.0)));
    color = color.lerp(Vec3::new(0.55, 0.88, 0.47), deposits * 0.72);
    let fresnel = (1.0 - n.dot(view).max(0.0)).powf(2.2);
    color * lighting(n, view, u, 0.17, 0.82, 38.0, 0.24) + Vec3::new(0.015, 0.10, 0.055) * fresnel
}

fn sun(p: Vec3, n: Vec3, view: Vec3, time: f32) -> Vec3 {
    // Capa 1: fotosfera amarilla/naranja.
    let cells = fbm(p * 5.0 + Vec3::new(0.0, time * 0.035, 0.0));
    let mut color = Vec3::new(1.0, 0.16, 0.008).lerp(Vec3::new(1.0, 0.82, 0.08), cells);
    // Capa 2: granulación brillante.
    let granules = smoothstep(0.46, 0.76, fbm(p * 17.0 + Vec3::new(time * 0.02, 0.0, 2.4)));
    color = color.lerp(Vec3::new(1.0, 1.0, 0.50), granules * 0.78);
    // Capa 3: filamentos de plasma oscuros y curvados.
    let plasma_noise = fbm(p * 8.0 + Vec3::new(cells * 2.5, time * 0.025, 0.0));
    let plasma = 1.0
        - smoothstep(
            0.04,
            0.18,
            (p.y * 13.0 + p.x * 4.0 + plasma_noise * 5.0).sin().abs(),
        );
    color = color.lerp(Vec3::new(0.52, 0.025, 0.002), plasma * 0.72);
    // Capa 4: zonas activas y corona de borde.
    let active = smoothstep(0.72, 0.89, cells * 0.55 + plasma_noise * 0.55);
    color += Vec3::new(0.65, 0.28, 0.025) * active;
    let corona = (1.0 - n.dot(view).max(0.0)).powf(1.7);
    color + Vec3::new(0.55, 0.15, 0.008) * (0.22 + corona * 0.95)
}

fn mechanical_world(p: Vec3, n: Vec3, view: Vec3, u: &Uniforms) -> Vec3 {
    let light = lighting(n, view, u, 0.18, 0.72, 44.0, 0.34);
    // Capa 1: roca/metal con variación procedural.
    let grain = fbm(p * 15.0);
    let mut color = Vec3::new(0.025, 0.032, 0.034).lerp(Vec3::new(0.28, 0.31, 0.27), grain);
    // Capa 2: retícula de paneles meridianos y paralelos.
    let longitude = p.z.atan2(p.x);
    let latitude = p.y.asin();
    let grid_lon = (longitude * 18.0).sin().abs();
    let grid_lat = (latitude * 22.0).sin().abs();
    let panel_line = 1.0 - smoothstep(0.0, 0.075, grid_lon.min(grid_lat));
    color = color.lerp(Vec3::new(0.008, 0.010, 0.012), panel_line * 0.82);
    // Capa 3: trinchera ecuatorial doble.
    let trench = 1.0 - smoothstep(0.025, 0.075, p.y.abs());
    let trench_edge = 1.0 - smoothstep(0.01, 0.028, (p.y.abs() - 0.085).abs());
    color = color.lerp(Vec3::new(0.004, 0.006, 0.008), trench * 0.95);
    color += Vec3::new(0.15, 0.18, 0.13) * trench_edge;
    // Capa 4: plato circular del superláser pintado por el shader.
    let dish_center = Vec3::new(0.43, 0.34, 0.84).normalized();
    let distance = (p - dish_center).length();
    let dish = 1.0 - smoothstep(0.25, 0.28, distance);
    let dish_ring = 1.0 - smoothstep(0.012, 0.030, (distance - 0.225).abs());
    let dish_core = 1.0 - smoothstep(0.035, 0.075, distance);
    color = color.lerp(Vec3::new(0.09, 0.11, 0.09), dish * 0.92);
    color += Vec3::new(0.28, 0.34, 0.22) * dish_ring + Vec3::new(0.28, 0.52, 0.18) * dish_core;
    color * light + Vec3::new(0.025, 0.05, 0.02) * dish_core
}

fn gas_giant(p: Vec3, n: Vec3, view: Vec3, u: &Uniforms) -> Vec3 {
    // Capa 1: gradiente violeta/azul por latitud.
    let latitude = p.y;
    let mut color =
        Vec3::new(0.035, 0.055, 0.30).lerp(Vec3::new(0.29, 0.08, 0.52), latitude * 0.5 + 0.5);
    // Capa 2: bandas atmosféricas onduladas.
    let turbulence =
        fbm(Vec3::new(p.x * 3.5, p.y * 8.0, p.z * 3.5) + Vec3::new(0.0, u.time * 0.025, 0.0));
    let bands = (p.y * 28.0 + turbulence * 7.0).sin() * 0.5 + 0.5;
    color = color.lerp(
        Vec3::new(0.02, 0.48, 0.72),
        smoothstep(0.56, 0.88, bands) * 0.68,
    );
    // Capa 3: nubes finas y contrabandas oscuras.
    let wisps = 1.0 - smoothstep(0.04, 0.16, (p.y * 43.0 + fbm(p * 11.0) * 8.0).sin().abs());
    color = color.lerp(Vec3::new(0.35, 0.84, 1.0), wisps * 0.58);
    let dark_bands = smoothstep(0.74, 0.93, fbm(Vec3::new(p.x * 5.0, p.y * 17.0, p.z * 5.0)));
    color = color.lerp(Vec3::new(0.018, 0.02, 0.13), dark_bands * 0.62);
    // Capa 4: tormenta oval turquesa.
    let storm = ((p.x + 0.38) / 0.34).powi(2) + ((p.y + 0.16) / 0.17).powi(2);
    let storm_mask = 1.0 - smoothstep(0.72, 1.08, storm);
    let storm_eye = 1.0 - smoothstep(0.04, 0.18, storm);
    color = color.lerp(Vec3::new(0.04, 0.8, 0.7), storm_mask * 0.88);
    color += Vec3::new(0.45, 0.95, 0.72) * storm_eye;
    color * lighting(n, view, u, 0.24, 0.78, 54.0, 0.26)
}

fn ring(p: Vec3, n: Vec3, view: Vec3, u: &Uniforms, dark: Vec3, bright: Vec3) -> Vec3 {
    let angle = p.z.atan2(p.x);
    let stripe = (angle * 24.0 + fbm(p * 5.0) * 2.4).sin() * 0.5 + 0.5;
    let color = dark.lerp(bright, 0.32 + stripe * 0.68);
    color * lighting(n, view, u, 0.34, 0.78, 70.0, 0.48) + bright * 0.09
}

fn moon(p: Vec3, n: Vec3, view: Vec3, u: &Uniforms) -> Vec3 {
    let craters = smoothstep(0.58, 0.74, fbm(p * 13.0));
    let base = Vec3::new(0.14, 0.15, 0.17).lerp(Vec3::new(0.48, 0.5, 0.54), fbm(p * 5.0));
    base.lerp(Vec3::new(0.045, 0.05, 0.06), craters * 0.8)
        * lighting(n, view, u, 0.12, 0.82, 24.0, 0.18)
}

fn lighting(
    n: Vec3,
    view: Vec3,
    u: &Uniforms,
    ambient: f32,
    diffuse_strength: f32,
    shininess: f32,
    specular_strength: f32,
) -> Vec3 {
    let light = u.light_direction.normalized();
    let diffuse = n.dot(light).max(0.0);
    let half = (light + view).normalized();
    let specular = n.dot(half).max(0.0).powf(shininess) * specular_strength;
    let rim = (1.0 - n.dot(view).max(0.0)).powf(3.0) * 0.18;
    Vec3::new(
        ambient + diffuse * diffuse_strength + specular + rim,
        ambient + diffuse * diffuse_strength + specular + rim,
        ambient + diffuse * diffuse_strength + specular + rim,
    )
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn hash(p: Vec3) -> f32 {
    ((p.dot(Vec3::new(127.1, 311.7, 74.7))).sin() * 43_758.547)
        .fract()
        .abs()
}
fn noise(p: Vec3) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (Vec3::new(3.0, 3.0, 3.0) - f * 2.0);
    let x00 = mix(hash(i), hash(i + Vec3::new(1.0, 0.0, 0.0)), u.x);
    let x10 = mix(
        hash(i + Vec3::new(0.0, 1.0, 0.0)),
        hash(i + Vec3::new(1.0, 1.0, 0.0)),
        u.x,
    );
    let x01 = mix(
        hash(i + Vec3::new(0.0, 0.0, 1.0)),
        hash(i + Vec3::new(1.0, 0.0, 1.0)),
        u.x,
    );
    let x11 = mix(
        hash(i + Vec3::new(0.0, 1.0, 1.0)),
        hash(i + Vec3::new(1.0, 1.0, 1.0)),
        u.x,
    );
    mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z)
}
fn mix(a: f32, b: f32, amount: f32) -> f32 {
    a + (b - a) * amount
}
fn fbm(mut p: Vec3) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    for _ in 0..4 {
        value += noise(p) * amplitude;
        p = p * 2.03 + Vec3::new(4.7, 1.3, 8.1);
        amplitude *= 0.5;
    }
    value
}
