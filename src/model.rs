use crate::math::Vec3;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    fs,
    path::Path,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct Face {
    pub vertex_indices: Vec<usize>,
    pub normal_indices: Vec<Option<usize>>,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub faces: Vec<Face>,
}

#[derive(Debug)]
pub struct ObjError {
    line: usize,
    message: String,
}
impl ObjError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}
impl Display for ObjError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "línea {}: {}", self.line, self.message)
        }
    }
}
impl Error for ObjError {}

impl Model {
    pub fn load(path: impl AsRef<Path>) -> Result<Arc<Self>, Box<dyn Error>> {
        Ok(Arc::new(Self::from_obj_str(&fs::read_to_string(path)?)?))
    }
    pub fn from_obj_str(source: &str) -> Result<Self, ObjError> {
        let mut model = Self::default();
        for (line_index, raw) in source.lines().enumerate() {
            let number = line_index + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("v") => {
                    let v = parse_vec3(fields, number, "vértice")?;
                    model.vertices.push(v);
                }
                Some("vn") => {
                    let n = parse_vec3(fields, number, "normal")?;
                    model.normals.push(n);
                }
                Some("f") => {
                    let mut vertices = Vec::new();
                    let mut normals = Vec::new();
                    for item in fields {
                        let mut parts = item.split('/');
                        vertices.push(resolve_index(
                            parts.next().unwrap_or_default(),
                            model.vertices.len(),
                            number,
                        )?);
                        let _uv = parts.next();
                        normals.push(
                            parts
                                .next()
                                .filter(|v| !v.is_empty())
                                .map(|v| resolve_index(v, model.normals.len(), number))
                                .transpose()?,
                        );
                    }
                    if vertices.len() < 3 {
                        return Err(ObjError::new(
                            number,
                            "una cara necesita al menos tres vértices",
                        ));
                    }
                    model.faces.push(Face {
                        vertex_indices: vertices,
                        normal_indices: normals,
                    });
                }
                _ => {}
            }
        }
        if model.vertices.is_empty() || model.faces.is_empty() {
            return Err(ObjError::new(0, "el OBJ no contiene geometría"));
        }
        Ok(model)
    }
    pub fn triangle_count(&self) -> usize {
        self.faces.iter().map(|f| f.vertex_indices.len() - 2).sum()
    }
}

fn parse_vec3<'a>(
    fields: impl Iterator<Item = &'a str>,
    line: usize,
    kind: &str,
) -> Result<Vec3, ObjError> {
    let values = fields
        .take(3)
        .map(|v| {
            v.parse::<f32>()
                .map_err(|_| ObjError::new(line, format!("{kind} inválido")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != 3 {
        return Err(ObjError::new(
            line,
            format!("{kind} necesita tres coordenadas"),
        ));
    }
    Ok(Vec3::new(values[0], values[1], values[2]))
}
fn resolve_index(value: &str, count: usize, line: usize) -> Result<usize, ObjError> {
    let obj = value
        .parse::<isize>()
        .map_err(|_| ObjError::new(line, "indice OBJ inválido"))?;
    let index = if obj > 0 {
        (obj - 1) as usize
    } else if obj < 0 {
        count
            .checked_sub(obj.unsigned_abs())
            .ok_or_else(|| ObjError::new(line, "indice fuera de rango"))?
    } else {
        return Err(ObjError::new(line, "OBJ no permite el indice 0"));
    };
    if index >= count {
        return Err(ObjError::new(line, "indice fuera de rango"));
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::Model;
    #[test]
    fn parses_obj_faces() {
        let m =
            Model::from_obj_str("v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//1 3//1\n").unwrap();
        assert_eq!(m.triangle_count(), 1);
    }
}
