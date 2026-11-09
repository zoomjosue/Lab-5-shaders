use crate::{
    framebuffer::Framebuffer,
    shaders::{FragmentInput, Uniforms, Varyings, fragment_shader},
};

#[derive(Clone, Copy, Debug)]
pub struct ScreenVertex {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub inverse_w: f32,
    pub varyings: Varyings,
}

#[derive(Clone, Copy, Debug)]
pub struct RasterTriangle {
    pub vertices: [ScreenVertex; 3],
    pub uniform_index: usize,
}

fn edge(a: ScreenVertex, b: ScreenVertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

pub fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

pub fn draw_triangles_parallel(
    framebuffer: &mut Framebuffer,
    triangles: &[RasterTriangle],
    uniforms: &[Uniforms],
) {
    let width = framebuffer.width();
    let height = framebuffer.height();
    let workers = worker_count().min(height).max(1);
    let rows_per_worker = height.div_ceil(workers);
    let pixels_per_worker = rows_per_worker * width;
    let (pixels, depth) = framebuffer.render_buffers_mut();

    std::thread::scope(|scope| {
        for (worker, (pixel_rows, depth_rows)) in pixels
            .chunks_mut(pixels_per_worker)
            .zip(depth.chunks_mut(pixels_per_worker))
            .enumerate()
        {
            let y_start = worker * rows_per_worker;
            let y_end = (y_start + pixel_rows.len() / width).min(height);
            scope.spawn(move || {
                for triangle in triangles {
                    draw_triangle_rows(
                        pixel_rows,
                        depth_rows,
                        width,
                        height,
                        y_start,
                        y_end,
                        triangle.vertices,
                        &uniforms[triangle.uniform_index],
                    );
                }
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_triangle_rows(
    pixels: &mut [u32],
    depth_buffer: &mut [f32],
    width: usize,
    height: usize,
    y_start: usize,
    y_end: usize,
    v: [ScreenVertex; 3],
    uniforms: &Uniforms,
) {
    let [a, b, c] = v;
    let area = edge(a, b, c.x, c.y);
    if area.abs() <= f32::EPSILON {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
    let max_x =
        a.x.max(b.x)
            .max(c.x)
            .ceil()
            .min(width.saturating_sub(1) as f32) as i32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(y_start as f32) as i32;
    let max_y =
        a.y.max(b.y)
            .max(c.y)
            .ceil()
            .min(y_end.saturating_sub(1).min(height.saturating_sub(1)) as f32) as i32;
    if min_y > max_y {
        return;
    }
    let inverse_area = 1.0 / area;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let sx = x as f32 + 0.5;
            let sy = y as f32 + 0.5;
            let wa = edge(b, c, sx, sy) * inverse_area;
            let wb = edge(c, a, sx, sy) * inverse_area;
            let wc = 1.0 - wa - wb;
            if wa >= -0.00001 && wb >= -0.00001 && wc >= -0.00001 {
                let depth = wa * a.depth + wb * b.depth + wc * c.depth;
                let index = (y as usize - y_start) * width + x as usize;
                if depth >= depth_buffer[index] {
                    continue;
                }
                let inverse_w = wa * a.inverse_w + wb * b.inverse_w + wc * c.inverse_w;
                if inverse_w <= f32::EPSILON {
                    continue;
                }
                let weights = [
                    wa * a.inverse_w / inverse_w,
                    wb * b.inverse_w / inverse_w,
                    wc * c.inverse_w / inverse_w,
                ];
                let varyings =
                    Varyings::weighted_sum([a.varyings, b.varyings, c.varyings], weights);
                let color = fragment_shader(
                    FragmentInput {
                        object_position: varyings.object_position,
                        world_position: varyings.world_position,
                        world_normal: varyings.world_normal,
                    },
                    uniforms,
                );
                depth_buffer[index] = depth;
                pixels[index] = color.to_u32();
            }
        }
    }
}
