use crate::{
    math::{Mat4, Vec3},
    model::Model,
    shaders::ShaderKind,
};
use std::{error::Error, path::Path, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    VerdantWorld,
    MechanicalWorld,
    AtomicGiant,
    Sun,
}

impl SceneKind {
    pub const ALL: [Self; 4] = [
        Self::VerdantWorld,
        Self::MechanicalWorld,
        Self::AtomicGiant,
        Self::Sun,
    ];
    pub const fn number(self) -> usize {
        match self {
            Self::VerdantWorld => 1,
            Self::MechanicalWorld => 2,
            Self::AtomicGiant => 3,
            Self::Sun => 4,
        }
    }
    pub const fn title(self) -> &'static str {
        match self {
            Self::VerdantWorld => "1 · Viridia — planeta rocoso verde",
            Self::MechanicalWorld => "2 · Noctis — planeta rocoso mecánico + luna",
            Self::AtomicGiant => "3 · Azurón — gigante gaseoso con anillos",
            Self::Sun => "4 · Helios — sol procedural",
        }
    }
    pub fn from_number(value: usize) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.number() == value)
    }
}

#[derive(Clone)]
pub struct Object {
    pub model: Arc<Model>,
    pub model_matrix: Mat4,
    pub normal_matrix: Mat4,
    pub shader: ShaderKind,
}

impl Object {
    fn new(
        model: Arc<Model>,
        translation: Vec3,
        rotation: Vec3,
        scale: f32,
        shader: ShaderKind,
    ) -> Self {
        let rotation_matrix = Mat4::rotation_z(rotation.z)
            * Mat4::rotation_x(rotation.x)
            * Mat4::rotation_y(rotation.y);
        Self {
            model,
            model_matrix: Mat4::translation(translation)
                * rotation_matrix
                * Mat4::scale(Vec3::new(scale, scale, scale)),
            normal_matrix: rotation_matrix,
            shader,
        }
    }
}

pub struct Scene {
    pub kind: SceneKind,
    pub objects: Vec<Object>,
    pub camera_distance: f32,
}

pub fn load(kind: SceneKind, asset_dir: impl AsRef<Path>) -> Result<Scene, Box<dyn Error>> {
    let assets = asset_dir.as_ref();
    let sphere = Model::load(assets.join("sphere.obj"))?;
    let mut objects = Vec::new();
    let camera_distance = match kind {
        SceneKind::VerdantWorld => {
            objects.push(Object::new(
                sphere,
                Vec3::ZERO,
                Vec3::ZERO,
                1.25,
                ShaderKind::VerdantWorld,
            ));
            4.1
        }
        SceneKind::MechanicalWorld => {
            objects.push(Object::new(
                sphere.clone(),
                Vec3::ZERO,
                Vec3::new(0.0, -0.18, 0.0),
                1.12,
                ShaderKind::MechanicalWorld,
            ));
            objects.push(Object::new(
                sphere,
                Vec3::new(1.72, 0.72, -0.05),
                Vec3::ZERO,
                0.28,
                ShaderKind::Moon,
            ));
            4.9
        }
        SceneKind::AtomicGiant => {
            objects.push(Object::new(
                sphere,
                Vec3::ZERO,
                Vec3::new(0.05, 0.0, -0.08),
                1.02,
                ShaderKind::GasGiant,
            ));
            let ring_blue = Model::load(assets.join("ring_blue.obj"))?;
            let ring_green = Model::load(assets.join("ring_green.obj"))?;
            let ring_orange = Model::load(assets.join("ring_orange.obj"))?;
            objects.push(Object::new(
                ring_blue,
                Vec3::ZERO,
                Vec3::new(0.32, 0.08, 0.34),
                1.0,
                ShaderKind::RingBlue,
            ));
            objects.push(Object::new(
                ring_green,
                Vec3::ZERO,
                Vec3::new(-0.58, 0.18, -0.28),
                1.0,
                ShaderKind::RingGreen,
            ));
            objects.push(Object::new(
                ring_orange,
                Vec3::ZERO,
                Vec3::new(0.22, -0.62, 0.9),
                1.0,
                ShaderKind::RingOrange,
            ));
            5.35
        }
        SceneKind::Sun => {
            objects.push(Object::new(
                sphere,
                Vec3::ZERO,
                Vec3::ZERO,
                1.28,
                ShaderKind::Sun,
            ));
            4.15
        }
    };
    Ok(Scene {
        kind,
        objects,
        camera_distance,
    })
}
