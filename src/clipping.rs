use crate::{math::Vec4, shaders::VertexOutput};

type ClipPlane = fn(Vec4) -> f32;
const PLANES: [ClipPlane; 6] = [
    |p| p.x + p.w,
    |p| p.w - p.x,
    |p| p.y + p.w,
    |p| p.w - p.y,
    |p| p.z + p.w,
    |p| p.w - p.z,
];

pub fn clip_triangle(triangle: [VertexOutput; 3]) -> Vec<[VertexOutput; 3]> {
    let mut polygon = triangle.to_vec();
    for plane in PLANES {
        if polygon.is_empty() {
            break;
        }
        let input = std::mem::take(&mut polygon);
        let mut previous = *input.last().expect("polígono no vacío");
        let mut previous_distance = plane(previous.clip_position);
        for current in input {
            let distance = plane(current.clip_position);
            let previous_inside = previous_distance >= 0.0;
            let current_inside = distance >= 0.0;
            if previous_inside != current_inside {
                polygon.push(
                    previous.lerp(current, previous_distance / (previous_distance - distance)),
                );
            }
            if current_inside {
                polygon.push(current);
            }
            previous = current;
            previous_distance = distance;
        }
    }
    if polygon.len() < 3 {
        return Vec::new();
    }
    (1..polygon.len() - 1)
        .map(|i| [polygon[0], polygon[i], polygon[i + 1]])
        .collect()
}
