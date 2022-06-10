use std::{path::PathBuf as StdPathBuf, str::FromStr, time::Instant};

use crossbeam_channel::Receiver;
use log::{error, info};
use pyo3::{
    exceptions::{PyIOError, PyRuntimeError, PyTypeError},
    prelude::*,
    types::PyDict,
};

use plumber_core::{
    asset::Importer,
    fs::GamePathBuf,
    model::loader::Settings as MdlSettings,
    vmf::loader::{
        BrushSetting, GeometrySettings, InvisibleSolids, MergeSolids, Settings as VmfSettings,
    },
};

use crate::{
    asset::{material::TextureInterpolation, BlenderAssetHandler, HandlerSettings, Message},
    filesystem::PyFileSystem,
};

#[pyclass(module = "plumber", name = "Importer")]
pub struct PyImporter {
    importer: Option<Importer<BlenderAssetHandler>>,
    receiver: Receiver<Message>,
    callback_obj: PyObject,
}

#[pymethods]
impl PyImporter {
    #[new]
    #[args(file_system, callback_obj, threads_suggestion, kwargs = "**")]
    fn new(
        file_system: &PyFileSystem,
        callback_obj: PyObject,
        threads_suggestion: usize,
        kwargs: Option<&PyDict>,
    ) -> PyResult<Self> {
        let (sender, receiver) = crossbeam_channel::bounded(16);

        let mut settings = HandlerSettings::default();

        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs.iter() {
                match key.extract()? {
                    "import_lights" => settings.import_lights = value.extract()?,
                    "light_factor" => settings.light.light_factor = value.extract()?,
                    "sun_factor" => settings.light.sun_factor = value.extract()?,
                    "ambient_factor" => settings.light.ambient_factor = value.extract()?,
                    "import_sky_camera" => settings.import_sky_camera = value.extract()?,
                    "sky_equi_height" => settings.sky_equi_height = value.extract()?,
                    "scale" => settings.scale = value.extract()?,
                    "target_fps" => settings.target_fps = value.extract()?,
                    "remove_animations" => settings.remove_animations = value.extract()?,
                    "simple_materials" => settings.material.simple_materials = value.extract()?,
                    "allow_culling" => settings.material.allow_culling = value.extract()?,
                    "editor_materials" => settings.material.editor_materials = value.extract()?,
                    "texture_interpolation" => {
                        settings.material.texture_interpolation =
                            TextureInterpolation::from_str(value.extract()?)?;
                    }
                    _ => return Err(PyTypeError::new_err("unexpected kwarg")),
                }
            }
        }

        let handler = BlenderAssetHandler { sender, settings };

        let start = Instant::now();
        info!(
            "opening file system of game `{}`...",
            file_system.file_system.name
        );

        let opened = file_system
            .file_system
            .open()
            .map_err(|e| PyIOError::new_err(e.to_string()))?;

        info!(
            "file system opened in {:.2} s",
            start.elapsed().as_secs_f32()
        );

        let importer = Some(Importer::new(opened, handler, threads_suggestion));

        Ok(Self {
            importer,
            receiver,
            callback_obj,
        })
    }

    #[args(bytes, kwargs = "**")]
    fn import_vmf(&mut self, py: Python, bytes: &[u8], kwargs: Option<&PyDict>) -> PyResult<()> {
        let importer = self.consume()?;

        let mut import_brushes = true;
        let mut geometry_settings = GeometrySettings::default();
        let mut settings = VmfSettings::default();

        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs.iter() {
                match key.extract()? {
                    "import_brushes" => {
                        import_brushes = value.extract()?;
                    }
                    "import_overlays" => {
                        geometry_settings.overlays(value.extract()?);
                    }
                    "epsilon" => {
                        geometry_settings.epsilon(value.extract()?);
                    }
                    "cut_threshold" => {
                        geometry_settings.cut_threshold(value.extract()?);
                    }
                    "merge_solids" => match value.extract()? {
                        "MERGE" => geometry_settings.merge_solids(MergeSolids::Merge),
                        "SEPARATE" => geometry_settings.merge_solids(MergeSolids::Separate),
                        _ => return Err(PyTypeError::new_err("unexpected kwarg value")),
                    },
                    "invisible_solids" => match value.extract()? {
                        "IMPORT" => geometry_settings.invisible_solids(InvisibleSolids::Import),
                        "SKIP" => geometry_settings.invisible_solids(InvisibleSolids::Skip),
                        _ => return Err(PyTypeError::new_err("unexpected kwarg value")),
                    },
                    "import_materials" => {
                        settings.import_materials(value.extract()?);
                    }
                    "import_props" => {
                        settings.import_props(value.extract()?);
                    }
                    "import_entities" => {
                        settings.import_entities(value.extract()?);
                    }
                    "import_sky" => {
                        settings.import_skybox(value.extract()?);
                    }
                    "scale" => {
                        settings.scale(value.extract()?);
                    }
                    _ => return Err(PyTypeError::new_err("unexpected kwarg")),
                }

                settings.brushes(if import_brushes {
                    BrushSetting::Import(geometry_settings)
                } else {
                    BrushSetting::Skip
                });
            }
        }

        let start = Instant::now();
        let file_size_mb = bytes.len() as f32 / (1024.0 * 1024.0);
        info!("importing vmf ({:.2} MB)...", file_size_mb);

        importer
            .import_vmf_blocking(bytes, &settings, || self.process_assets(py))
            .map_err(|e| PyIOError::new_err(e.to_string()))?;

        info!("vmf imported in {:.2} s", start.elapsed().as_secs_f32());

        Ok(())
    }

    #[args(path, from_game, kwargs = "**")]
    fn import_mdl(
        &mut self,
        py: Python,
        path: &str,
        from_game: bool,
        kwargs: Option<&PyDict>,
    ) -> PyResult<()> {
        let importer = self.consume()?;

        let path = if from_game {
            GamePathBuf::from(path).into()
        } else {
            StdPathBuf::from(path).into()
        };

        let (import_materials, settings) = Self::mdl_settings(kwargs)?;

        let start = Instant::now();
        info!("importing mdl `{}`...", path);

        importer
            .import_mdl_blocking(path, settings, import_materials, || self.process_assets(py))
            .map_err(|e| PyIOError::new_err(e.to_string()))?;

        info!("mdl imported in {:.2} s", start.elapsed().as_secs_f32());

        Ok(())
    }

    fn import_vmt(&mut self, py: Python, path: &str, from_game: bool) -> PyResult<()> {
        let importer = self.consume()?;

        let path = if from_game {
            GamePathBuf::from(path).into()
        } else {
            StdPathBuf::from(path).into()
        };

        let start = Instant::now();
        info!("importing vmt `{}`...", path);

        importer
            .import_vmt_blocking(&path, || self.process_assets(py))
            .map_err(|e| PyIOError::new_err(e.to_string()))?;

        info!("vmt imported in {:.2} s", start.elapsed().as_secs_f32());

        Ok(())
    }

    #[args(path, kwargs = "**")]
    fn stage_mdl(&mut self, path: &str, kwargs: Option<&PyDict>) -> PyResult<()> {
        let importer = self.borrow()?;

        let (import_materials, settings) = Self::mdl_settings(kwargs)?;

        importer.import_mdl(GamePathBuf::from(path).into(), settings, import_materials);

        Ok(())
    }

    fn import_assets(&mut self, py: Python) {
        // drop the importer, causing the asset channel to disconnect
        // if we don't do this, process_assets will hang forever waiting for new assets to be sent
        self.importer = None;

        self.process_assets(py);
    }
}

impl PyImporter {
    fn consume(&mut self) -> PyResult<Importer<BlenderAssetHandler>> {
        self.importer
            .take()
            .ok_or_else(|| PyRuntimeError::new_err("Importer already consumed"))
    }

    fn borrow(&mut self) -> PyResult<&mut Importer<BlenderAssetHandler>> {
        self.importer
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Importer already consumed"))
    }

    fn process_assets(&self, py: Python) {
        let callback_ref = self.callback_obj.as_ref(py);

        for asset in self.receiver.iter() {
            let result = match asset {
                Message::Material(material) => callback_ref.call_method1("material", (material,)),
                Message::Texture(texture) => callback_ref.call_method1("texture", (texture,)),
                Message::Model(model) => callback_ref.call_method1("model", (model,)),
                Message::Brush(brush) => callback_ref.call_method1("brush", (brush,)),
                Message::Overlay(overlay) => callback_ref.call_method1("overlay", (overlay,)),
                Message::Prop(prop) => callback_ref.call_method1("prop", (prop,)),
                Message::Light(light) => callback_ref.call_method1("light", (light,)),
                Message::SpotLight(light) => callback_ref.call_method1("spot_light", (light,)),
                Message::EnvLight(light) => callback_ref.call_method1("env_light", (light,)),
                Message::SkyCamera(sky_camera) => {
                    callback_ref.call_method1("sky_camera", (sky_camera,))
                }
                Message::SkyEqui(sky_equi) => callback_ref.call_method1("sky_equi", (sky_equi,)),
            };

            if let Err(err) = result {
                err.print(py);
                error!("Asset importing errored: {}", err);
            }
        }
    }

    fn mdl_settings(kwargs: Option<&PyDict>) -> PyResult<(bool, MdlSettings)> {
        let mut import_materials = true;
        let mut settings = MdlSettings::default();

        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs.iter() {
                match key.extract()? {
                    "import_animations" => settings.import_animations(value.extract()?),
                    "import_materials" => import_materials = value.extract()?,
                    _ => return Err(PyTypeError::new_err("unexpected kwarg")),
                }
            }
        }

        Ok((import_materials, settings))
    }
}