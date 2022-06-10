use std::mem;

use glam::Vec3;
use plumber_core::vmf::loader::{BuiltBrushEntity, BuiltSolid, MergedSolids, SolidFace};
use pyo3::{prelude::*, types::PyList};

#[pyclass(module = "plumber", name = "MergedSolids")]
pub struct PyMergedSolids {
    no_draw: bool,
    position: [f32; 3],
    scale: [f32; 3],
    faces: Vec<SolidFace>,
    materials: Vec<String>,
    flat_vertices: Vec<f32>,
    flat_polygon_vertice_indices: Vec<usize>,
    flat_loop_uvs: Vec<f32>,
    flat_loop_colors: Vec<f32>,
}

#[pymethods]
impl PyMergedSolids {
    fn no_draw(&self) -> bool {
        self.no_draw
    }

    fn position(&self) -> [f32; 3] {
        self.position
    }

    fn scale(&self) -> [f32; 3] {
        self.scale
    }

    fn vertices(&mut self) -> Vec<f32> {
        mem::take(&mut self.flat_vertices)
    }

    fn loops_len(&self) -> usize {
        self.faces.iter().map(|f| f.vertice_indices.len()).sum()
    }

    fn polygons_len(&self) -> usize {
        self.faces.len()
    }

    fn polygon_loop_totals<'p>(&self, py: Python<'p>) -> &'p PyList {
        PyList::new(py, self.faces.iter().map(|f| f.vertice_indices.len()))
    }

    fn polygon_loop_starts<'p>(&self, py: Python<'p>) -> &'p PyList {
        let mut acc = 0;

        PyList::new(
            py,
            self.faces.iter().map(|f| {
                let acc_before = acc;
                acc += f.vertice_indices.len();
                acc_before
            }),
        )
    }

    fn polygon_vertices(&mut self) -> Vec<usize> {
        mem::take(&mut self.flat_polygon_vertice_indices)
    }

    fn polygon_material_indices<'p>(&self, py: Python<'p>) -> &'p PyList {
        PyList::new(py, self.faces.iter().map(|f| f.material_index))
    }

    fn loop_uvs(&mut self) -> Vec<f32> {
        mem::take(&mut self.flat_loop_uvs)
    }

    fn loop_colors(&mut self) -> Vec<f32> {
        mem::take(&mut self.flat_loop_colors)
    }

    fn materials(&mut self) -> Vec<String> {
        mem::take(&mut self.materials)
    }
}

impl PyMergedSolids {
    fn new(merged: MergedSolids) -> Self {
        let flat_vertices = merged.vertices.iter().flat_map(Vec3::to_array).collect();

        let flat_polygon_vertice_indices = merged
            .faces
            .iter()
            .flat_map(|f| &f.vertice_indices)
            .copied()
            .collect();

        let flat_loop_uvs = merged
            .faces
            .iter()
            .flat_map(|f| {
                f.vertice_uvs
                    .iter()
                    // blender has inverted v axis compared to Source
                    .flat_map(|uv| [uv.x, 1.0 - uv.y])
            })
            .collect();

        let flat_loop_colors = merged
            .faces
            .iter()
            .flat_map(|f| {
                f.vertice_alphas
                    .iter()
                    .flat_map(|&a| [a / 255., a / 255., a / 255., 1.0])
            })
            .collect();

        Self {
            no_draw: merged.materials.iter().all(|m| m.info.no_draw()),
            position: [0.0, 0.0, 0.0],
            scale: [merged.scale, merged.scale, merged.scale],
            faces: merged.faces,
            materials: merged
                .materials
                .into_iter()
                .map(|m| m.name.into_string())
                .collect(),
            flat_vertices,
            flat_polygon_vertice_indices,
            flat_loop_uvs,
            flat_loop_colors,
        }
    }
}

#[pyclass(module = "plumber", name = "BuiltSolid")]
pub struct PyBuiltSolid {
    id: i32,
    no_draw: bool,
    position: [f32; 3],
    scale: [f32; 3],
    faces: Vec<SolidFace>,
    materials: Vec<String>,
    flat_vertices: Vec<f32>,
    flat_polygon_vertice_indices: Vec<usize>,
    flat_loop_uvs: Vec<f32>,
    flat_loop_colors: Vec<f32>,
}

#[pymethods]
impl PyBuiltSolid {
    fn id(&self) -> i32 {
        self.id
    }

    fn no_draw(&self) -> bool {
        self.no_draw
    }

    fn position(&self) -> [f32; 3] {
        self.position
    }

    fn scale(&self) -> [f32; 3] {
        self.scale
    }

    fn vertices(&mut self) -> Vec<f32> {
        mem::take(&mut self.flat_vertices)
    }

    fn loops_len(&self) -> usize {
        self.faces.iter().map(|f| f.vertice_indices.len()).sum()
    }

    fn polygons_len(&self) -> usize {
        self.faces.len()
    }

    fn polygon_loop_totals<'p>(&self, py: Python<'p>) -> &'p PyList {
        PyList::new(py, self.faces.iter().map(|f| f.vertice_indices.len()))
    }

    fn polygon_loop_starts<'p>(&self, py: Python<'p>) -> &'p PyList {
        let mut acc = 0;

        PyList::new(
            py,
            self.faces.iter().map(|f| {
                let acc_before = acc;
                acc += f.vertice_indices.len();
                acc_before
            }),
        )
    }

    fn polygon_vertices(&mut self) -> Vec<usize> {
        mem::take(&mut self.flat_polygon_vertice_indices)
    }

    fn polygon_material_indices<'p>(&self, py: Python<'p>) -> &'p PyList {
        PyList::new(py, self.faces.iter().map(|f| f.material_index))
    }

    fn loop_uvs(&mut self) -> Vec<f32> {
        mem::take(&mut self.flat_loop_uvs)
    }

    fn loop_colors(&mut self) -> Vec<f32> {
        mem::take(&mut self.flat_loop_colors)
    }

    fn materials(&mut self) -> Vec<String> {
        mem::take(&mut self.materials)
    }
}

impl PyBuiltSolid {
    fn new(solid: BuiltSolid) -> Self {
        let flat_vertices = solid.vertices.iter().flat_map(Vec3::to_array).collect();

        let flat_polygon_vertice_indices = solid
            .faces
            .iter()
            .flat_map(|f| &f.vertice_indices)
            .copied()
            .collect();

        let flat_loop_uvs = solid
            .faces
            .iter()
            .flat_map(|f| {
                f.vertice_uvs
                    .iter()
                    // blender has inverted v axis compared to Source
                    .flat_map(|uv| [uv.x, 1.0 - uv.y])
            })
            .collect();

        let flat_loop_colors = solid
            .faces
            .iter()
            .flat_map(|f| {
                f.vertice_alphas
                    .iter()
                    .flat_map(|&a| [a / 255., a / 255., a / 255., 1.0])
            })
            .collect();

        Self {
            id: solid.id,
            no_draw: solid.materials.iter().all(|m| m.info.no_draw()),
            position: solid.position.to_array(),
            scale: [solid.scale, solid.scale, solid.scale],
            faces: solid.faces,
            materials: solid
                .materials
                .into_iter()
                .map(|m| m.name.into_string())
                .collect(),
            flat_vertices,
            flat_polygon_vertice_indices,
            flat_loop_uvs,
            flat_loop_colors,
        }
    }
}

#[pyclass(module = "plumber", name = "BuiltBrushEntity")]
pub struct PyBuiltBrushEntity {
    id: i32,
    class_name: String,
    merged_solids: Option<PyMergedSolids>,
    solids: Vec<PyBuiltSolid>,
}

#[pymethods]
impl PyBuiltBrushEntity {
    fn id(&self) -> i32 {
        self.id
    }

    fn class_name(&self) -> &str {
        &self.class_name
    }

    fn merged_solids(&mut self) -> Option<PyMergedSolids> {
        self.merged_solids.take()
    }

    fn solids(&mut self) -> Vec<PyBuiltSolid> {
        mem::take(&mut self.solids)
    }
}

impl PyBuiltBrushEntity {
    pub fn new(brush: BuiltBrushEntity) -> Self {
        Self {
            id: brush.id,
            class_name: brush.class_name.to_owned(),
            merged_solids: brush.merged_solids.map(PyMergedSolids::new),
            solids: brush.solids.into_iter().map(PyBuiltSolid::new).collect(),
        }
    }
}