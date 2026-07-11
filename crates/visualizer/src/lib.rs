use std::collections::{HashMap, HashSet};

use protocol::{
    AttributeControl, EngineSnapshot, FixtureId, GeometrySummary, PatchedFixtureSummary,
    StageObjectId, StageObjectKind, StageObjectSummary, Vec3, VideoOutputId, VideoOutputKind,
    VideoOutputSummary,
};
use serde::{Deserialize, Serialize};

const FULL_SCALE: f32 = 65_535.0;
const MODEL_MESH_SEGMENTS: usize = 16;
const MODEL_SPHERE_LATITUDE_SEGMENTS: usize = 8;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VisualizerConfig {
    pub beam_length: f32,
    pub beam_radius: f32,
    pub minimum_beam_intensity: f32,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            beam_length: 12.0,
            beam_radius: 0.18,
            minimum_beam_intensity: 0.01,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizerScene {
    pub fixtures: Vec<FixtureNode>,
    pub fixture_geometries: Vec<FixtureGeometryNode>,
    pub fixture_models: Vec<FixtureModelNode>,
    pub beams: Vec<BeamNode>,
    pub video_surfaces: Vec<VideoSurfaceNode>,
    pub stage_objects: Vec<StageObjectNode>,
    pub bounds: StageBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizerRenderPayload {
    pub scene: VisualizerScene,
    pub model_render_plans: Vec<FixtureModelRenderPlan>,
    pub primitive_meshes: Vec<FixtureModelPrimitiveMesh>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureNode {
    pub id: FixtureId,
    pub label: String,
    pub position: Vec3,
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub roll_deg: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeamNode {
    pub fixture_id: FixtureId,
    pub geometry_name: Option<String>,
    pub beam_type: Option<String>,
    pub beam_angle_deg: Option<f32>,
    pub field_angle_deg: Option<f32>,
    pub origin: Vec3,
    pub direction: Vec3,
    pub length: f32,
    pub radius: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureGeometryNode {
    pub fixture_id: FixtureId,
    pub profile_source_path: String,
    pub name: String,
    pub kind: String,
    pub parent: Option<String>,
    pub model_name: Option<String>,
    pub model_file: Option<String>,
    pub model_primitive: Option<String>,
    pub model_mesh_kind: GeometryModelMeshKind,
    pub model_dimensions: Option<Vec3>,
    pub position: Vec3,
    pub local_position: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub direction: Vec3,
    pub local_right: Vec3,
    pub local_up: Vec3,
    pub local_direction: Vec3,
    pub beam_type: Option<String>,
    pub beam_angle_deg: Option<f32>,
    pub field_angle_deg: Option<f32>,
    pub beam_radius: Option<f32>,
    pub mapped_channel_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeometryModelMeshKind {
    None,
    Box,
    Cylinder,
    Sphere,
    Plane,
    Mesh,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureModelNode {
    pub fixture_id: FixtureId,
    pub profile_source_path: String,
    pub geometry_name: String,
    pub mesh_kind: GeometryModelMeshKind,
    pub model_name: Option<String>,
    pub model_file: Option<String>,
    pub model_primitive: Option<String>,
    pub dimensions: Option<Vec3>,
    pub bounding_radius: f32,
    pub position: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub direction: Vec3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FixtureModelDrawKind {
    BuiltInPrimitive,
    ExternalMesh,
    BoundsFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureModelRenderPlan {
    pub fixture_id: FixtureId,
    pub profile_source_path: String,
    pub geometry_name: String,
    pub draw_kind: FixtureModelDrawKind,
    pub fallback_mesh_kind: GeometryModelMeshKind,
    pub model_file: Option<String>,
    pub dimensions: Option<Vec3>,
    pub bounding_radius: f32,
    pub position: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub direction: Vec3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ModelPrimitiveVertex {
    pub position: Vec3,
    pub normal: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureModelPrimitiveMesh {
    pub fixture_id: FixtureId,
    pub geometry_name: String,
    pub mesh_kind: GeometryModelMeshKind,
    pub vertices: Vec<ModelPrimitiveVertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjModelMeshError {
    Utf8,
    MissingPositions,
    MissingFaces,
    InvalidPosition { line: usize },
    InvalidNormal { line: usize },
    InvalidFace { line: usize },
    InvalidIndex { line: usize, token: String },
    IndexOutOfRange { line: usize, token: String },
    TooManyVertices,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreeDsModelMeshError {
    InvalidChunkHeader { offset: usize },
    InvalidChunkLength { offset: usize },
    MissingPositions,
    MissingFaces,
    InvalidVertexList { offset: usize },
    InvalidFaceList { offset: usize },
    FaceIndexOutOfRange { offset: usize, index: u16 },
    TooManyVertices,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlbModelMeshError {
    InvalidHeader,
    UnsupportedVersion { version: u32 },
    InvalidChunk { offset: usize },
    MissingJsonChunk,
    MissingBinChunk,
    InvalidJson,
    MissingMeshPrimitive,
    MissingPositionAccessor,
    InvalidAccessor { index: usize },
    InvalidBufferView { index: usize },
    UnsupportedPositionAccessor { index: usize },
    UnsupportedIndexAccessor { index: usize },
    MissingPositions,
    MissingFaces,
    IndexOutOfRange { index: u32 },
    TooManyVertices,
}

#[derive(Debug, Clone, Copy)]
struct ObjFaceVertex {
    position_index: usize,
    normal_index: Option<usize>,
}

#[derive(Debug, Clone)]
struct GltfPrimitiveData {
    positions: Vec<Vec3>,
    indices: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
struct GltfMeshInstance {
    mesh_index: usize,
    transform: GltfNodeTransform,
}

#[derive(Debug, Clone)]
struct GltfAccessorInfo {
    buffer_view: usize,
    byte_offset: usize,
    component_type: u32,
    count: usize,
    item_type: String,
}

#[derive(Debug, Clone, Copy)]
struct GltfBufferViewInfo {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct GltfNodeTransform {
    right: Vec3,
    up: Vec3,
    direction: Vec3,
    origin: Vec3,
}

#[derive(Debug, Clone, Copy)]
struct GlbChunk<'a> {
    chunk_type: u32,
    data: &'a [u8],
}

const GLB_MAGIC: u32 = 0x4654_6c67;
const GLB_VERSION_2: u32 = 2;
const GLB_JSON_CHUNK: u32 = 0x4e4f_534a;
const GLB_BIN_CHUNK: u32 = 0x004e_4942;

#[derive(Debug, Clone)]
struct ThreeDsMeshData {
    positions: Vec<Vec3>,
    faces: Vec<[usize; 3]>,
    transform: Option<ThreeDsMeshTransform>,
}

#[derive(Debug, Clone, Copy)]
struct ThreeDsMeshTransform {
    right: Vec3,
    up: Vec3,
    direction: Vec3,
    origin: Vec3,
}

#[derive(Debug, Clone, Copy)]
struct ThreeDsChunk {
    id: u16,
    payload_start: usize,
    end: usize,
}

const THREE_DS_MAIN_CHUNK: u16 = 0x4d4d;
const THREE_DS_EDITOR_CHUNK: u16 = 0x3d3d;
const THREE_DS_OBJECT_BLOCK_CHUNK: u16 = 0x4000;
const THREE_DS_TRIANGULAR_MESH_CHUNK: u16 = 0x4100;
const THREE_DS_VERTEX_LIST_CHUNK: u16 = 0x4110;
const THREE_DS_FACE_LIST_CHUNK: u16 = 0x4120;
const THREE_DS_MESH_MATRIX_CHUNK: u16 = 0x4160;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoSurfaceNode {
    pub id: VideoOutputId,
    pub label: String,
    pub kind: VideoOutputKind,
    pub composition_id: protocol::CompositionId,
    pub position: Vec3,
    pub width: f32,
    pub height: f32,
    pub rotation_deg: f32,
    pub opacity: f32,
    pub enabled: bool,
    pub blackout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageObjectNode {
    pub id: StageObjectId,
    pub label: String,
    pub kind: StageObjectKind,
    pub position: Vec3,
    pub width: f32,
    pub depth: f32,
    pub rotation_deg: f32,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct StageBounds {
    pub min: Vec3,
    pub max: Vec3,
}

pub fn build_visualizer_scene(
    snapshot: &EngineSnapshot,
    config: VisualizerConfig,
) -> VisualizerScene {
    let dmx_frames = dmx_preview_frames(snapshot);
    let fixtures = snapshot
        .fixtures
        .iter()
        .map(|fixture| fixture_node_from_summary(fixture, &dmx_frames))
        .collect::<Vec<_>>();
    let fixture_geometries = snapshot
        .fixtures
        .iter()
        .flat_map(fixture_geometry_nodes_from_summary)
        .collect::<Vec<_>>();
    let fixture_models = fixture_model_nodes_from_geometry_nodes(&fixture_geometries);
    let beams = fixtures
        .iter()
        .filter(|fixture| fixture.intensity >= config.minimum_beam_intensity)
        .map(|fixture| {
            let origin_geometry = beam_origin_geometry(fixture.id, &fixture_geometries);
            BeamNode {
                fixture_id: fixture.id,
                geometry_name: origin_geometry.map(|geometry| geometry.name.clone()),
                beam_type: origin_geometry.and_then(|geometry| geometry.beam_type.clone()),
                beam_angle_deg: origin_geometry.and_then(|geometry| geometry.beam_angle_deg),
                field_angle_deg: origin_geometry.and_then(|geometry| geometry.field_angle_deg),
                origin: origin_geometry
                    .map(|geometry| geometry.position)
                    .unwrap_or(fixture.position),
                direction: origin_geometry
                    .map(|geometry| {
                        normalize(rotate_vec3(
                            geometry.local_direction,
                            fixture.pitch_deg,
                            fixture.yaw_deg,
                            fixture.roll_deg,
                        ))
                    })
                    .unwrap_or_else(|| beam_direction(fixture.yaw_deg, fixture.pitch_deg)),
                length: config.beam_length.max(0.0),
                radius: beam_radius(origin_geometry, config),
                color: fixture.color,
                intensity: fixture.intensity,
            }
        })
        .collect::<Vec<_>>();
    let video_surfaces = snapshot
        .video
        .outputs
        .iter()
        .map(video_surface_node_from_summary)
        .collect::<Vec<_>>();
    let stage_objects = snapshot
        .stage_objects
        .iter()
        .map(stage_object_node_from_summary)
        .collect::<Vec<_>>();
    let bounds = stage_bounds(
        snapshot.fixtures.as_slice(),
        fixture_geometries.as_slice(),
        fixture_models.as_slice(),
        video_surfaces.as_slice(),
        stage_objects.as_slice(),
    );

    VisualizerScene {
        fixtures,
        fixture_geometries,
        fixture_models,
        beams,
        video_surfaces,
        stage_objects,
        bounds,
    }
}

pub fn build_visualizer_render_payload(
    snapshot: &EngineSnapshot,
    config: VisualizerConfig,
) -> VisualizerRenderPayload {
    let scene = build_visualizer_scene(snapshot, config);
    let model_render_plans = build_fixture_model_render_plans(&scene);
    let primitive_meshes = build_fixture_model_primitive_meshes(&model_render_plans);
    VisualizerRenderPayload {
        scene,
        model_render_plans,
        primitive_meshes,
    }
}

pub fn build_fixture_model_render_plans(scene: &VisualizerScene) -> Vec<FixtureModelRenderPlan> {
    scene
        .fixture_models
        .iter()
        .filter_map(fixture_model_render_plan)
        .collect()
}

pub fn build_fixture_model_primitive_meshes(
    plans: &[FixtureModelRenderPlan],
) -> Vec<FixtureModelPrimitiveMesh> {
    plans
        .iter()
        .filter_map(build_fixture_model_primitive_mesh)
        .collect()
}

pub fn build_fixture_model_primitive_mesh(
    plan: &FixtureModelRenderPlan,
) -> Option<FixtureModelPrimitiveMesh> {
    if plan.draw_kind == FixtureModelDrawKind::ExternalMesh {
        return None;
    }
    let dimensions = sanitized_dimensions(plan.dimensions)?;
    let mesh_kind = match plan.fallback_mesh_kind {
        GeometryModelMeshKind::Cylinder => GeometryModelMeshKind::Cylinder,
        GeometryModelMeshKind::Sphere => GeometryModelMeshKind::Sphere,
        GeometryModelMeshKind::Plane => GeometryModelMeshKind::Plane,
        GeometryModelMeshKind::Box
        | GeometryModelMeshKind::Mesh
        | GeometryModelMeshKind::Unknown
        | GeometryModelMeshKind::None => GeometryModelMeshKind::Box,
    };
    let (vertices, indices) = match mesh_kind {
        GeometryModelMeshKind::Cylinder => primitive_cylinder_mesh(plan, dimensions),
        GeometryModelMeshKind::Sphere => primitive_sphere_mesh(plan, dimensions),
        GeometryModelMeshKind::Plane => primitive_plane_mesh(plan, dimensions),
        _ => primitive_box_mesh(plan, dimensions),
    };
    Some(FixtureModelPrimitiveMesh {
        fixture_id: plan.fixture_id,
        geometry_name: plan.geometry_name.clone(),
        mesh_kind,
        vertices,
        indices,
    })
}

pub fn build_fixture_model_obj_mesh(
    plan: &FixtureModelRenderPlan,
    obj_bytes: &[u8],
) -> Result<FixtureModelPrimitiveMesh, ObjModelMeshError> {
    let obj_text = std::str::from_utf8(obj_bytes).map_err(|_| ObjModelMeshError::Utf8)?;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut faces: Vec<Vec<ObjFaceVertex>> = Vec::new();

    for (line_index, raw_line) in obj_text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line
            .split_once('#')
            .map(|(content, _)| content)
            .unwrap_or(raw_line)
            .trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };
        match kind {
            "v" => {
                let values = parts.collect::<Vec<_>>();
                if values.len() < 3 {
                    return Err(ObjModelMeshError::InvalidPosition { line: line_number });
                }
                positions.push(Vec3 {
                    x: parse_obj_f32(values[0], line_number)
                        .map_err(|_| ObjModelMeshError::InvalidPosition { line: line_number })?,
                    y: parse_obj_f32(values[1], line_number)
                        .map_err(|_| ObjModelMeshError::InvalidPosition { line: line_number })?,
                    z: parse_obj_f32(values[2], line_number)
                        .map_err(|_| ObjModelMeshError::InvalidPosition { line: line_number })?,
                });
            }
            "vn" => {
                let values = parts.collect::<Vec<_>>();
                if values.len() < 3 {
                    return Err(ObjModelMeshError::InvalidNormal { line: line_number });
                }
                normals.push(normalize(Vec3 {
                    x: parse_obj_f32(values[0], line_number)
                        .map_err(|_| ObjModelMeshError::InvalidNormal { line: line_number })?,
                    y: parse_obj_f32(values[1], line_number)
                        .map_err(|_| ObjModelMeshError::InvalidNormal { line: line_number })?,
                    z: parse_obj_f32(values[2], line_number)
                        .map_err(|_| ObjModelMeshError::InvalidNormal { line: line_number })?,
                }));
            }
            "f" => {
                let face = parts
                    .map(|token| {
                        parse_obj_face_vertex(token, positions.len(), normals.len(), line_number)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if face.len() < 3 {
                    return Err(ObjModelMeshError::InvalidFace { line: line_number });
                }
                faces.push(face);
            }
            _ => {}
        }
    }

    if positions.is_empty() {
        return Err(ObjModelMeshError::MissingPositions);
    }
    if faces.is_empty() {
        return Err(ObjModelMeshError::MissingFaces);
    }

    let (bounds_min, bounds_max) = mesh_position_bounds(&positions);
    let bounds_center = scale_vec3(add_vec3(bounds_min, bounds_max), 0.5);
    let bounds_size = sub_vec3(bounds_max, bounds_min);
    let scale = mesh_model_scale(bounds_size, plan.dimensions);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for face in faces {
        for triangle_index in 1..face.len() - 1 {
            push_obj_triangle(
                plan,
                &positions,
                &normals,
                [face[0], face[triangle_index], face[triangle_index + 1]],
                bounds_center,
                scale,
                &mut vertices,
                &mut indices,
            )?;
        }
    }

    Ok(FixtureModelPrimitiveMesh {
        fixture_id: plan.fixture_id,
        geometry_name: plan.geometry_name.clone(),
        mesh_kind: GeometryModelMeshKind::Mesh,
        vertices,
        indices,
    })
}

pub fn build_fixture_model_3ds_mesh(
    plan: &FixtureModelRenderPlan,
    three_ds_bytes: &[u8],
) -> Result<FixtureModelPrimitiveMesh, ThreeDsModelMeshError> {
    let meshes = parse_three_ds_meshes(three_ds_bytes)?;
    let all_positions = meshes
        .iter()
        .flat_map(|mesh| {
            mesh.positions
                .iter()
                .copied()
                .map(|position| transform_three_ds_position(position, mesh.transform))
        })
        .collect::<Vec<_>>();
    if all_positions.is_empty() {
        return Err(ThreeDsModelMeshError::MissingPositions);
    }
    if meshes.iter().all(|mesh| mesh.faces.is_empty()) {
        return Err(ThreeDsModelMeshError::MissingFaces);
    }

    let (bounds_min, bounds_max) = mesh_position_bounds(&all_positions);
    let bounds_center = scale_vec3(add_vec3(bounds_min, bounds_max), 0.5);
    let bounds_size = sub_vec3(bounds_max, bounds_min);
    let scale = mesh_model_scale(bounds_size, plan.dimensions);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for mesh in meshes {
        for face in mesh.faces {
            let start = u32::try_from(vertices.len())
                .map_err(|_| ThreeDsModelMeshError::TooManyVertices)?;
            let world_positions = face.map(|index| {
                transform_mesh_position(
                    plan,
                    transform_three_ds_position(mesh.positions[index], mesh.transform),
                    bounds_center,
                    scale,
                )
            });
            let normal = normalize(cross_vec3(
                sub_vec3(world_positions[1], world_positions[0]),
                sub_vec3(world_positions[2], world_positions[0]),
            ));
            vertices
                .extend(world_positions.map(|position| ModelPrimitiveVertex { position, normal }));
            indices.extend([start, start + 1, start + 2]);
        }
    }

    Ok(FixtureModelPrimitiveMesh {
        fixture_id: plan.fixture_id,
        geometry_name: plan.geometry_name.clone(),
        mesh_kind: GeometryModelMeshKind::Mesh,
        vertices,
        indices,
    })
}

pub fn build_fixture_model_glb_mesh(
    plan: &FixtureModelRenderPlan,
    glb_bytes: &[u8],
) -> Result<FixtureModelPrimitiveMesh, GlbModelMeshError> {
    let primitives = parse_glb_primitives(glb_bytes)?;
    let all_positions = primitives
        .iter()
        .flat_map(|primitive| primitive.positions.iter().copied())
        .collect::<Vec<_>>();
    if all_positions.is_empty() {
        return Err(GlbModelMeshError::MissingPositions);
    }
    if primitives
        .iter()
        .all(|primitive| primitive.indices.is_empty())
    {
        return Err(GlbModelMeshError::MissingFaces);
    }

    let (bounds_min, bounds_max) = mesh_position_bounds(&all_positions);
    let bounds_center = scale_vec3(add_vec3(bounds_min, bounds_max), 0.5);
    let bounds_size = sub_vec3(bounds_max, bounds_min);
    let scale = mesh_model_scale(bounds_size, plan.dimensions);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for primitive in primitives {
        for triangle in primitive.indices.chunks_exact(3) {
            let start =
                u32::try_from(vertices.len()).map_err(|_| GlbModelMeshError::TooManyVertices)?;
            let mut world_positions = [Vec3::default(); 3];
            for (index, position_index) in triangle.iter().copied().enumerate() {
                let Some(position) = primitive.positions.get(position_index as usize).copied()
                else {
                    return Err(GlbModelMeshError::IndexOutOfRange {
                        index: position_index,
                    });
                };
                world_positions[index] =
                    transform_mesh_position(plan, position, bounds_center, scale);
            }
            let normal = normalize(cross_vec3(
                sub_vec3(world_positions[1], world_positions[0]),
                sub_vec3(world_positions[2], world_positions[0]),
            ));
            vertices
                .extend(world_positions.map(|position| ModelPrimitiveVertex { position, normal }));
            indices.extend([start, start + 1, start + 2]);
        }
    }

    Ok(FixtureModelPrimitiveMesh {
        fixture_id: plan.fixture_id,
        geometry_name: plan.geometry_name.clone(),
        mesh_kind: GeometryModelMeshKind::Mesh,
        vertices,
        indices,
    })
}

fn beam_origin_geometry(
    fixture_id: FixtureId,
    fixture_geometries: &[FixtureGeometryNode],
) -> Option<&FixtureGeometryNode> {
    fixture_geometries
        .iter()
        .filter(|geometry| geometry.fixture_id == fixture_id)
        .find(|geometry| geometry.kind.eq_ignore_ascii_case("beam"))
        .or_else(|| {
            fixture_geometries
                .iter()
                .filter(|geometry| geometry.fixture_id == fixture_id)
                .find(|geometry| {
                    let name = geometry.name.to_ascii_lowercase();
                    name.contains("beam") || name.contains("lens")
                })
        })
        .or_else(|| {
            fixture_geometries
                .iter()
                .filter(|geometry| geometry.fixture_id == fixture_id)
                .find(|geometry| geometry.mapped_channel_count > 0)
        })
}

fn video_surface_node_from_summary(output: &VideoOutputSummary) -> VideoSurfaceNode {
    let mapping = output.mapping;
    let aspect = if mapping.aspect_ratio.is_finite() && mapping.aspect_ratio > 0.0 {
        mapping.aspect_ratio
    } else {
        output.width as f32 / output.height.max(1) as f32
    }
    .clamp(0.1, 10.0);
    let base_height = 2.0;
    let scale_x = finite_or(mapping.scale_x, 1.0).clamp(0.01, 8.0);
    let scale_y = finite_or(mapping.scale_y, 1.0).clamp(0.01, 8.0);

    VideoSurfaceNode {
        id: output.id,
        label: output.label.clone(),
        kind: output.kind.clone(),
        composition_id: output.composition_id,
        position: Vec3 {
            x: finite_or(mapping.stage_x, 0.0),
            y: finite_or(mapping.stage_y, 0.0),
            z: finite_or(mapping.stage_z, 0.0),
        },
        width: (base_height * aspect * scale_x).max(0.01),
        height: (base_height * scale_y).max(0.01),
        rotation_deg: finite_or(mapping.rotation_deg, 0.0),
        opacity: finite_or(output.opacity, 1.0).clamp(0.0, 1.0),
        enabled: output.enabled,
        blackout: output.blackout,
    }
}

fn stage_object_node_from_summary(object: &StageObjectSummary) -> StageObjectNode {
    StageObjectNode {
        id: object.id,
        label: object.label.clone(),
        kind: object.kind,
        position: Vec3 {
            x: finite_or(object.x, 0.0),
            y: 0.0,
            z: finite_or(object.z, 0.0),
        },
        width: finite_or(object.width, 0.0).abs().max(0.01),
        depth: finite_or(object.depth, 0.0).abs().max(0.01),
        rotation_deg: finite_or(object.rotation_deg, 0.0),
        color: object
            .color
            .as_ref()
            .map(|color| color.trim().to_string())
            .filter(|color| !color.is_empty()),
    }
}

fn fixture_geometry_nodes_from_summary(
    fixture: &PatchedFixtureSummary,
) -> Vec<FixtureGeometryNode> {
    if fixture.geometries.is_empty() {
        return Vec::new();
    }

    let geometry_by_name = fixture
        .geometries
        .iter()
        .map(|geometry| (geometry.name.as_str(), geometry))
        .collect::<HashMap<_, _>>();
    let mapped_channel_counts = mapped_channel_counts_by_geometry(fixture);

    fixture
        .geometries
        .iter()
        .map(|geometry| {
            let cumulative = cumulative_geometry_matrix(geometry, &geometry_by_name);
            let local_position = matrix_translation(cumulative);
            let local_right = matrix_right(cumulative);
            let local_up = matrix_up(cumulative);
            let local_direction = matrix_forward(cumulative);
            let right = normalize(rotate_vec3(
                local_right,
                fixture.rotation.pitch,
                fixture.rotation.yaw,
                fixture.rotation.roll,
            ));
            let up = normalize(rotate_vec3(
                local_up,
                fixture.rotation.pitch,
                fixture.rotation.yaw,
                fixture.rotation.roll,
            ));
            let direction = normalize(rotate_vec3(
                local_direction,
                fixture.rotation.pitch,
                fixture.rotation.yaw,
                fixture.rotation.roll,
            ));
            FixtureGeometryNode {
                fixture_id: fixture.id,
                profile_source_path: fixture.profile_source_path.clone(),
                name: geometry.name.clone(),
                kind: geometry.kind.clone(),
                parent: geometry.parent.clone(),
                model_name: geometry.model_name.clone(),
                model_file: geometry.model_file.clone(),
                model_primitive: geometry.model_primitive.clone(),
                model_mesh_kind: model_mesh_kind(
                    geometry.model_primitive.as_deref(),
                    geometry.model_file.as_deref(),
                    geometry.model_dimensions,
                ),
                model_dimensions: geometry.model_dimensions,
                position: add_vec3(
                    fixture.position,
                    rotate_vec3(
                        local_position,
                        fixture.rotation.pitch,
                        fixture.rotation.yaw,
                        fixture.rotation.roll,
                    ),
                ),
                local_position,
                right,
                up,
                direction,
                local_right,
                local_up,
                local_direction,
                beam_type: geometry.beam_type.clone(),
                beam_angle_deg: geometry.beam_angle_deg,
                field_angle_deg: geometry.field_angle_deg,
                beam_radius: geometry.beam_radius,
                mapped_channel_count: mapped_channel_counts
                    .get(geometry.name.as_str())
                    .copied()
                    .unwrap_or(0),
            }
        })
        .collect()
}

fn beam_radius(origin_geometry: Option<&FixtureGeometryNode>, config: VisualizerConfig) -> f32 {
    let fallback_radius = config.beam_radius.max(0.0);
    let Some(geometry) = origin_geometry else {
        return fallback_radius;
    };
    let base_radius = geometry
        .beam_radius
        .filter(|radius| radius.is_finite() && *radius >= 0.0)
        .unwrap_or(fallback_radius);
    let angle_radius = geometry
        .field_angle_deg
        .or(geometry.beam_angle_deg)
        .filter(|angle| angle.is_finite() && *angle > 0.0 && *angle < 179.0)
        .map(|angle| (angle.to_radians() * 0.5).tan().abs() * config.beam_length.max(0.0))
        .unwrap_or(0.0);
    (base_radius + angle_radius).max(0.0)
}

fn fixture_model_nodes_from_geometry_nodes(
    geometries: &[FixtureGeometryNode],
) -> Vec<FixtureModelNode> {
    geometries
        .iter()
        .filter(|geometry| geometry.model_mesh_kind != GeometryModelMeshKind::None)
        .filter(|geometry| {
            geometry.model_dimensions.is_some()
                || geometry
                    .model_file
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_some()
        })
        .map(|geometry| FixtureModelNode {
            fixture_id: geometry.fixture_id,
            profile_source_path: geometry.profile_source_path.clone(),
            geometry_name: geometry.name.clone(),
            mesh_kind: geometry.model_mesh_kind,
            model_name: geometry.model_name.clone(),
            model_file: geometry.model_file.clone(),
            model_primitive: geometry.model_primitive.clone(),
            dimensions: geometry.model_dimensions,
            bounding_radius: model_bounding_radius(geometry.model_dimensions),
            position: geometry.position,
            right: geometry.right,
            up: geometry.up,
            direction: geometry.direction,
        })
        .collect()
}

fn model_bounding_radius(dimensions: Option<Vec3>) -> f32 {
    let Some(dimensions) = dimensions else {
        return 0.0;
    };
    if !dimensions.x.is_finite() || !dimensions.y.is_finite() || !dimensions.z.is_finite() {
        return 0.0;
    }
    let x = dimensions.x.max(0.0);
    let y = dimensions.y.max(0.0);
    let z = dimensions.z.max(0.0);
    ((x * x + y * y + z * z).sqrt() * 0.5).max(0.0)
}

fn fixture_model_render_plan(model: &FixtureModelNode) -> Option<FixtureModelRenderPlan> {
    let model_file = model
        .model_file
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let draw_kind = if model_file.is_some() {
        FixtureModelDrawKind::ExternalMesh
    } else if matches!(
        model.mesh_kind,
        GeometryModelMeshKind::Box
            | GeometryModelMeshKind::Cylinder
            | GeometryModelMeshKind::Sphere
            | GeometryModelMeshKind::Plane
    ) {
        FixtureModelDrawKind::BuiltInPrimitive
    } else if model.dimensions.is_some() {
        FixtureModelDrawKind::BoundsFallback
    } else {
        return None;
    };

    Some(FixtureModelRenderPlan {
        fixture_id: model.fixture_id,
        profile_source_path: model.profile_source_path.clone(),
        geometry_name: model.geometry_name.clone(),
        draw_kind,
        fallback_mesh_kind: match model.mesh_kind {
            GeometryModelMeshKind::None | GeometryModelMeshKind::Unknown => {
                GeometryModelMeshKind::Box
            }
            mesh_kind => mesh_kind,
        },
        model_file,
        dimensions: model.dimensions,
        bounding_radius: model.bounding_radius,
        position: model.position,
        right: model.right,
        up: model.up,
        direction: model.direction,
    })
}

fn sanitized_dimensions(dimensions: Option<Vec3>) -> Option<Vec3> {
    let dimensions = dimensions?;
    if !dimensions.x.is_finite() || !dimensions.y.is_finite() || !dimensions.z.is_finite() {
        return None;
    }
    let sanitized = Vec3 {
        x: dimensions.x.max(0.0),
        y: dimensions.y.max(0.0),
        z: dimensions.z.max(0.0),
    };
    (sanitized.x > 0.0 || sanitized.y > 0.0 || sanitized.z > 0.0).then_some(sanitized)
}

fn primitive_box_mesh(
    plan: &FixtureModelRenderPlan,
    dimensions: Vec3,
) -> (Vec<ModelPrimitiveVertex>, Vec<u32>) {
    let half_x = (dimensions.x.max(0.001)) * 0.5;
    let half_y = (dimensions.y.max(0.001)) * 0.5;
    let half_z = (dimensions.z.max(0.001)) * 0.5;
    let faces = [
        (plan.direction, plan.right, plan.up, half_z, half_x, half_y),
        (
            scale_vec3(plan.direction, -1.0),
            scale_vec3(plan.right, -1.0),
            plan.up,
            half_z,
            half_x,
            half_y,
        ),
        (
            plan.right,
            scale_vec3(plan.direction, -1.0),
            plan.up,
            half_x,
            half_z,
            half_y,
        ),
        (
            scale_vec3(plan.right, -1.0),
            plan.direction,
            plan.up,
            half_x,
            half_z,
            half_y,
        ),
        (
            plan.up,
            plan.right,
            scale_vec3(plan.direction, -1.0),
            half_y,
            half_x,
            half_z,
        ),
        (
            scale_vec3(plan.up, -1.0),
            plan.right,
            plan.direction,
            half_y,
            half_x,
            half_z,
        ),
    ];
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (normal, axis_u, axis_v, normal_offset, half_u, half_v) in faces {
        let face_center = add_vec3(plan.position, scale_vec3(normal, normal_offset));
        let start = vertices.len() as u32;
        vertices.extend([
            primitive_vertex(face_center, axis_u, axis_v, -half_u, -half_v, normal),
            primitive_vertex(face_center, axis_u, axis_v, half_u, -half_v, normal),
            primitive_vertex(face_center, axis_u, axis_v, half_u, half_v, normal),
            primitive_vertex(face_center, axis_u, axis_v, -half_u, half_v, normal),
        ]);
        indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }
    (vertices, indices)
}

fn primitive_plane_mesh(
    plan: &FixtureModelRenderPlan,
    dimensions: Vec3,
) -> (Vec<ModelPrimitiveVertex>, Vec<u32>) {
    let half_x = (dimensions.x.max(0.001)) * 0.5;
    let half_z = (dimensions.z.max(dimensions.y).max(0.001)) * 0.5;
    let normal = plan.up;
    let vertices = vec![
        primitive_vertex(
            plan.position,
            plan.right,
            plan.direction,
            -half_x,
            -half_z,
            normal,
        ),
        primitive_vertex(
            plan.position,
            plan.right,
            plan.direction,
            half_x,
            -half_z,
            normal,
        ),
        primitive_vertex(
            plan.position,
            plan.right,
            plan.direction,
            half_x,
            half_z,
            normal,
        ),
        primitive_vertex(
            plan.position,
            plan.right,
            plan.direction,
            -half_x,
            half_z,
            normal,
        ),
    ];
    (vertices, vec![0, 1, 2, 0, 2, 3])
}

fn primitive_cylinder_mesh(
    plan: &FixtureModelRenderPlan,
    dimensions: Vec3,
) -> (Vec<ModelPrimitiveVertex>, Vec<u32>) {
    let radius = (dimensions.x.max(dimensions.z).max(0.001)) * 0.5;
    let half_height = (dimensions.y.max(0.001)) * 0.5;
    let mut vertices = Vec::with_capacity((MODEL_MESH_SEGMENTS + 1) * 6 + 2);
    let mut indices = Vec::with_capacity(MODEL_MESH_SEGMENTS * 12);

    for index in 0..=MODEL_MESH_SEGMENTS {
        let radial = cylinder_radial(plan, index);
        vertices.push(ModelPrimitiveVertex {
            position: add_vec3(
                add_vec3(plan.position, scale_vec3(plan.up, -half_height)),
                scale_vec3(radial, radius),
            ),
            normal: radial,
        });
        vertices.push(ModelPrimitiveVertex {
            position: add_vec3(
                add_vec3(plan.position, scale_vec3(plan.up, half_height)),
                scale_vec3(radial, radius),
            ),
            normal: radial,
        });
    }

    for index in 0..MODEL_MESH_SEGMENTS {
        let base = (index * 2) as u32;
        let next = base + 2;
        indices.extend([base, next, base + 1, base + 1, next, next + 1]);
    }

    let bottom_center = vertices.len() as u32;
    vertices.push(ModelPrimitiveVertex {
        position: add_vec3(plan.position, scale_vec3(plan.up, -half_height)),
        normal: scale_vec3(plan.up, -1.0),
    });
    let top_center = vertices.len() as u32;
    vertices.push(ModelPrimitiveVertex {
        position: add_vec3(plan.position, scale_vec3(plan.up, half_height)),
        normal: plan.up,
    });
    let bottom_ring_start = vertices.len() as u32;
    for index in 0..=MODEL_MESH_SEGMENTS {
        let radial = cylinder_radial(plan, index);
        vertices.push(ModelPrimitiveVertex {
            position: add_vec3(
                add_vec3(plan.position, scale_vec3(plan.up, -half_height)),
                scale_vec3(radial, radius),
            ),
            normal: scale_vec3(plan.up, -1.0),
        });
    }
    let top_ring_start = vertices.len() as u32;
    for index in 0..=MODEL_MESH_SEGMENTS {
        let radial = cylinder_radial(plan, index);
        vertices.push(ModelPrimitiveVertex {
            position: add_vec3(
                add_vec3(plan.position, scale_vec3(plan.up, half_height)),
                scale_vec3(radial, radius),
            ),
            normal: plan.up,
        });
    }
    for index in 0..MODEL_MESH_SEGMENTS as u32 {
        indices.extend([
            bottom_center,
            bottom_ring_start + index + 1,
            bottom_ring_start + index,
        ]);
        indices.extend([
            top_center,
            top_ring_start + index,
            top_ring_start + index + 1,
        ]);
    }
    (vertices, indices)
}

fn primitive_sphere_mesh(
    plan: &FixtureModelRenderPlan,
    dimensions: Vec3,
) -> (Vec<ModelPrimitiveVertex>, Vec<u32>) {
    let radius_x = (dimensions.x.max(0.001)) * 0.5;
    let radius_y = (dimensions.y.max(0.001)) * 0.5;
    let radius_z = (dimensions.z.max(0.001)) * 0.5;
    let longitude_segments = MODEL_MESH_SEGMENTS;
    let latitude_segments = MODEL_SPHERE_LATITUDE_SEGMENTS;
    let mut vertices = Vec::with_capacity((latitude_segments + 1) * (longitude_segments + 1));
    let mut indices = Vec::with_capacity(latitude_segments * longitude_segments * 6);

    for lat in 0..=latitude_segments {
        let theta = std::f32::consts::PI * lat as f32 / latitude_segments as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        for lon in 0..=longitude_segments {
            let phi = std::f32::consts::TAU * lon as f32 / longitude_segments as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();
            let normal = normalize(add_vec3(
                add_vec3(
                    scale_vec3(plan.right, sin_theta * cos_phi),
                    scale_vec3(plan.up, cos_theta),
                ),
                scale_vec3(plan.direction, sin_theta * sin_phi),
            ));
            vertices.push(ModelPrimitiveVertex {
                position: add_vec3(
                    add_vec3(
                        add_vec3(
                            plan.position,
                            scale_vec3(plan.right, sin_theta * cos_phi * radius_x),
                        ),
                        scale_vec3(plan.up, cos_theta * radius_y),
                    ),
                    scale_vec3(plan.direction, sin_theta * sin_phi * radius_z),
                ),
                normal,
            });
        }
    }

    let stride = longitude_segments as u32 + 1;
    for lat in 0..latitude_segments as u32 {
        for lon in 0..longitude_segments as u32 {
            let first = lat * stride + lon;
            let second = first + stride;
            indices.extend([first, second, first + 1, first + 1, second, second + 1]);
        }
    }
    (vertices, indices)
}

fn cylinder_radial(plan: &FixtureModelRenderPlan, index: usize) -> Vec3 {
    let angle = std::f32::consts::TAU * index as f32 / MODEL_MESH_SEGMENTS as f32;
    normalize(add_vec3(
        scale_vec3(plan.right, angle.cos()),
        scale_vec3(plan.direction, angle.sin()),
    ))
}

fn primitive_vertex(
    center: Vec3,
    axis_u: Vec3,
    axis_v: Vec3,
    offset_u: f32,
    offset_v: f32,
    normal: Vec3,
) -> ModelPrimitiveVertex {
    ModelPrimitiveVertex {
        position: add_vec3(
            add_vec3(center, scale_vec3(axis_u, offset_u)),
            scale_vec3(axis_v, offset_v),
        ),
        normal,
    }
}

fn parse_obj_f32(value: &str, _line: usize) -> Result<f32, ()> {
    value.parse::<f32>().map_err(|_| ()).and_then(|parsed| {
        if parsed.is_finite() {
            Ok(parsed)
        } else {
            Err(())
        }
    })
}

fn parse_obj_face_vertex(
    token: &str,
    position_count: usize,
    normal_count: usize,
    line: usize,
) -> Result<ObjFaceVertex, ObjModelMeshError> {
    let mut segments = token.split('/');
    let position_token = segments.next().unwrap_or_default();
    if position_token.is_empty() {
        return Err(ObjModelMeshError::InvalidIndex {
            line,
            token: token.to_string(),
        });
    }
    let position_index = resolve_obj_index(position_token, position_count, line, token)?;
    let _texture_index = segments.next();
    let normal_index = segments
        .next()
        .filter(|normal_token| !normal_token.is_empty())
        .map(|normal_token| resolve_obj_index(normal_token, normal_count, line, token))
        .transpose()?;
    if segments.next().is_some() {
        return Err(ObjModelMeshError::InvalidFace { line });
    }
    Ok(ObjFaceVertex {
        position_index,
        normal_index,
    })
}

fn resolve_obj_index(
    raw_index: &str,
    item_count: usize,
    line: usize,
    token: &str,
) -> Result<usize, ObjModelMeshError> {
    let parsed = raw_index
        .parse::<isize>()
        .map_err(|_| ObjModelMeshError::InvalidIndex {
            line,
            token: token.to_string(),
        })?;
    if parsed == 0 {
        return Err(ObjModelMeshError::InvalidIndex {
            line,
            token: token.to_string(),
        });
    }
    let resolved = if parsed > 0 {
        parsed - 1
    } else {
        item_count as isize + parsed
    };
    if resolved < 0 || resolved as usize >= item_count {
        return Err(ObjModelMeshError::IndexOutOfRange {
            line,
            token: token.to_string(),
        });
    }
    Ok(resolved as usize)
}

fn mesh_position_bounds(positions: &[Vec3]) -> (Vec3, Vec3) {
    let first = positions[0];
    positions
        .iter()
        .copied()
        .fold((first, first), |(min, max), position| {
            (
                Vec3 {
                    x: min.x.min(position.x),
                    y: min.y.min(position.y),
                    z: min.z.min(position.z),
                },
                Vec3 {
                    x: max.x.max(position.x),
                    y: max.y.max(position.y),
                    z: max.z.max(position.z),
                },
            )
        })
}

fn mesh_model_scale(bounds_size: Vec3, dimensions: Option<Vec3>) -> Vec3 {
    let Some(dimensions) = sanitized_dimensions(dimensions) else {
        return Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        };
    };
    Vec3 {
        x: axis_scale(bounds_size.x, dimensions.x),
        y: axis_scale(bounds_size.y, dimensions.y),
        z: axis_scale(bounds_size.z, dimensions.z),
    }
}

fn axis_scale(source: f32, target: f32) -> f32 {
    if source.abs() <= f32::EPSILON || !source.is_finite() || !target.is_finite() {
        1.0
    } else {
        target / source.abs().max(0.001)
    }
}

fn push_obj_triangle(
    plan: &FixtureModelRenderPlan,
    positions: &[Vec3],
    normals: &[Vec3],
    triangle: [ObjFaceVertex; 3],
    bounds_center: Vec3,
    scale: Vec3,
    vertices: &mut Vec<ModelPrimitiveVertex>,
    indices: &mut Vec<u32>,
) -> Result<(), ObjModelMeshError> {
    let start = u32::try_from(vertices.len()).map_err(|_| ObjModelMeshError::TooManyVertices)?;
    let world_positions = triangle.map(|vertex| {
        transform_mesh_position(plan, positions[vertex.position_index], bounds_center, scale)
    });
    let face_normal = normalize(cross_vec3(
        sub_vec3(world_positions[1], world_positions[0]),
        sub_vec3(world_positions[2], world_positions[0]),
    ));
    for (face_vertex, position) in triangle.into_iter().zip(world_positions) {
        let normal = face_vertex
            .normal_index
            .map(|index| transform_mesh_normal(plan, normals[index]))
            .unwrap_or(face_normal);
        vertices.push(ModelPrimitiveVertex { position, normal });
    }
    indices.extend([start, start + 1, start + 2]);
    Ok(())
}

fn transform_mesh_position(
    plan: &FixtureModelRenderPlan,
    position: Vec3,
    bounds_center: Vec3,
    scale: Vec3,
) -> Vec3 {
    let offset = Vec3 {
        x: (position.x - bounds_center.x) * scale.x,
        y: (position.y - bounds_center.y) * scale.y,
        z: (position.z - bounds_center.z) * scale.z,
    };
    add_vec3(
        add_vec3(
            add_vec3(plan.position, scale_vec3(plan.right, offset.x)),
            scale_vec3(plan.up, offset.y),
        ),
        scale_vec3(plan.direction, offset.z),
    )
}

fn transform_mesh_normal(plan: &FixtureModelRenderPlan, normal: Vec3) -> Vec3 {
    normalize(add_vec3(
        add_vec3(
            scale_vec3(plan.right, normal.x),
            scale_vec3(plan.up, normal.y),
        ),
        scale_vec3(plan.direction, normal.z),
    ))
}

fn parse_glb_primitives(bytes: &[u8]) -> Result<Vec<GltfPrimitiveData>, GlbModelMeshError> {
    let (json_bytes, bin_bytes) = parse_glb_chunks(bytes)?;
    let json_text = std::str::from_utf8(json_bytes).map_err(|_| GlbModelMeshError::InvalidJson)?;
    let json: serde_json::Value =
        serde_json::from_str(json_text).map_err(|_| GlbModelMeshError::InvalidJson)?;
    let accessors = parse_gltf_accessors(&json)?;
    let buffer_views = parse_gltf_buffer_views(&json)?;
    let mesh_primitives = parse_gltf_mesh_primitives(&json, bin_bytes, &accessors, &buffer_views)?;
    let mesh_instances = parse_gltf_mesh_instances(&json)?;
    let mut primitives = if mesh_instances.is_empty() {
        mesh_primitives
            .iter()
            .flat_map(|mesh| mesh.iter().cloned())
            .collect::<Vec<_>>()
    } else {
        let mut transformed_primitives = Vec::new();
        for instance in mesh_instances {
            let mesh = mesh_primitives
                .get(instance.mesh_index)
                .ok_or(GlbModelMeshError::InvalidJson)?;
            transformed_primitives.extend(
                mesh.iter()
                    .map(|primitive| primitive.transformed(instance.transform)),
            );
        }
        transformed_primitives
    };

    if primitives.is_empty() {
        primitives = mesh_primitives
            .iter()
            .flat_map(|mesh| mesh.iter().cloned())
            .collect();
    }
    if primitives.is_empty() {
        return Err(GlbModelMeshError::MissingPositionAccessor);
    }
    Ok(primitives)
}

fn parse_gltf_mesh_primitives(
    json: &serde_json::Value,
    bin_bytes: &[u8],
    accessors: &[GltfAccessorInfo],
    buffer_views: &[GltfBufferViewInfo],
) -> Result<Vec<Vec<GltfPrimitiveData>>, GlbModelMeshError> {
    let meshes = json
        .get("meshes")
        .and_then(|value| value.as_array())
        .ok_or(GlbModelMeshError::MissingMeshPrimitive)?;
    let mut meshes_primitives = Vec::with_capacity(meshes.len());
    for mesh in meshes {
        let mut primitives = Vec::new();
        let Some(primitive_values) = mesh.get("primitives").and_then(|value| value.as_array())
        else {
            meshes_primitives.push(primitives);
            continue;
        };
        for primitive in primitive_values {
            let Some(position_accessor_index) = primitive
                .get("attributes")
                .and_then(|attributes| attributes.get("POSITION"))
                .and_then(|value| value.as_u64())
                .map(|value| value as usize)
            else {
                continue;
            };
            let positions = read_gltf_positions(
                bin_bytes,
                &accessors,
                &buffer_views,
                position_accessor_index,
            )?;
            let indices = primitive
                .get("indices")
                .and_then(|value| value.as_u64())
                .map(|index| {
                    read_gltf_indices(bin_bytes, &accessors, &buffer_views, index as usize)
                })
                .transpose()?
                .unwrap_or_else(|| (0..positions.len() as u32).collect());
            primitives.push(GltfPrimitiveData { positions, indices });
        }
        meshes_primitives.push(primitives);
    }
    if meshes_primitives.iter().all(Vec::is_empty) {
        return Err(GlbModelMeshError::MissingPositionAccessor);
    }
    Ok(meshes_primitives)
}

impl GltfPrimitiveData {
    fn transformed(&self, transform: GltfNodeTransform) -> Self {
        Self {
            positions: self
                .positions
                .iter()
                .copied()
                .map(|position| transform.transform_position(position))
                .collect(),
            indices: self.indices.clone(),
        }
    }
}

impl GltfNodeTransform {
    fn identity() -> Self {
        Self {
            right: Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            up: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            direction: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            origin: Vec3::default(),
        }
    }

    fn from_gltf_matrix(matrix: [f32; 16]) -> Self {
        Self {
            right: Vec3 {
                x: matrix[0],
                y: matrix[1],
                z: matrix[2],
            },
            up: Vec3 {
                x: matrix[4],
                y: matrix[5],
                z: matrix[6],
            },
            direction: Vec3 {
                x: matrix[8],
                y: matrix[9],
                z: matrix[10],
            },
            origin: Vec3 {
                x: matrix[12],
                y: matrix[13],
                z: matrix[14],
            },
        }
    }

    fn from_trs(translation: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> Self {
        Self {
            right: scale_vec3(
                rotate_vec3_by_gltf_quat(
                    Vec3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    rotation,
                ),
                scale[0],
            ),
            up: scale_vec3(
                rotate_vec3_by_gltf_quat(
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                    rotation,
                ),
                scale[1],
            ),
            direction: scale_vec3(
                rotate_vec3_by_gltf_quat(
                    Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    },
                    rotation,
                ),
                scale[2],
            ),
            origin: Vec3 {
                x: translation[0],
                y: translation[1],
                z: translation[2],
            },
        }
    }

    fn combine(self, child: Self) -> Self {
        Self {
            right: self.transform_vector(child.right),
            up: self.transform_vector(child.up),
            direction: self.transform_vector(child.direction),
            origin: self.transform_position(child.origin),
        }
    }

    fn transform_position(self, position: Vec3) -> Vec3 {
        add_vec3(
            add_vec3(
                add_vec3(self.origin, scale_vec3(self.right, position.x)),
                scale_vec3(self.up, position.y),
            ),
            scale_vec3(self.direction, position.z),
        )
    }

    fn transform_vector(self, vector: Vec3) -> Vec3 {
        add_vec3(
            add_vec3(
                scale_vec3(self.right, vector.x),
                scale_vec3(self.up, vector.y),
            ),
            scale_vec3(self.direction, vector.z),
        )
    }
}

fn parse_gltf_mesh_instances(
    json: &serde_json::Value,
) -> Result<Vec<GltfMeshInstance>, GlbModelMeshError> {
    let Some(nodes) = json.get("nodes").and_then(|value| value.as_array()) else {
        return Ok(Vec::new());
    };
    if nodes.is_empty() {
        return Ok(Vec::new());
    }

    let roots = parse_gltf_scene_roots(json, nodes.len())?;
    let mut instances = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for root in roots {
        collect_gltf_mesh_instances(
            nodes,
            root,
            GltfNodeTransform::identity(),
            &mut instances,
            &mut visiting,
            &mut visited,
        )?;
    }
    Ok(instances)
}

fn parse_gltf_scene_roots(
    json: &serde_json::Value,
    node_count: usize,
) -> Result<Vec<usize>, GlbModelMeshError> {
    let Some(scenes) = json.get("scenes").and_then(|value| value.as_array()) else {
        return Ok((0..node_count).collect());
    };
    if scenes.is_empty() {
        return Ok((0..node_count).collect());
    }
    let scene_index = json
        .get("scene")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    let scene = scenes
        .get(scene_index)
        .or_else(|| scenes.first())
        .ok_or(GlbModelMeshError::InvalidJson)?;
    match scene.get("nodes") {
        Some(value) => parse_gltf_index_array(value, node_count),
        None => Ok(Vec::new()),
    }
}

fn collect_gltf_mesh_instances(
    nodes: &[serde_json::Value],
    node_index: usize,
    parent_transform: GltfNodeTransform,
    instances: &mut Vec<GltfMeshInstance>,
    visiting: &mut HashSet<usize>,
    visited: &mut HashSet<usize>,
) -> Result<(), GlbModelMeshError> {
    if visited.contains(&node_index) {
        return Ok(());
    }
    let node = nodes
        .get(node_index)
        .ok_or(GlbModelMeshError::InvalidJson)?;
    if !visiting.insert(node_index) {
        return Err(GlbModelMeshError::InvalidJson);
    }

    let transform = parent_transform.combine(parse_gltf_node_transform(node, node_index)?);
    if let Some(mesh_index) = node
        .get("mesh")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
    {
        instances.push(GltfMeshInstance {
            mesh_index,
            transform,
        });
    }
    if let Some(children) = node.get("children") {
        for child in parse_gltf_index_array(children, nodes.len())? {
            collect_gltf_mesh_instances(nodes, child, transform, instances, visiting, visited)?;
        }
    }

    visiting.remove(&node_index);
    visited.insert(node_index);
    Ok(())
}

fn parse_gltf_node_transform(
    node: &serde_json::Value,
    node_index: usize,
) -> Result<GltfNodeTransform, GlbModelMeshError> {
    if let Some(matrix) = node.get("matrix") {
        return Ok(GltfNodeTransform::from_gltf_matrix(parse_gltf_f32_array::<
            16,
        >(
            matrix, node_index
        )?));
    }
    let translation = match node.get("translation") {
        Some(value) => parse_gltf_f32_array::<3>(value, node_index)?,
        None => [0.0, 0.0, 0.0],
    };
    let rotation = match node.get("rotation") {
        Some(value) => parse_gltf_f32_array::<4>(value, node_index)?,
        None => [0.0, 0.0, 0.0, 1.0],
    };
    let scale = match node.get("scale") {
        Some(value) => parse_gltf_f32_array::<3>(value, node_index)?,
        None => [1.0, 1.0, 1.0],
    };
    Ok(GltfNodeTransform::from_trs(translation, rotation, scale))
}

fn parse_gltf_index_array(
    value: &serde_json::Value,
    item_count: usize,
) -> Result<Vec<usize>, GlbModelMeshError> {
    let values = value.as_array().ok_or(GlbModelMeshError::InvalidJson)?;
    values
        .iter()
        .map(|value| {
            let index = value
                .as_u64()
                .map(|value| value as usize)
                .ok_or(GlbModelMeshError::InvalidJson)?;
            if index >= item_count {
                return Err(GlbModelMeshError::InvalidJson);
            }
            Ok(index)
        })
        .collect()
}

fn parse_gltf_f32_array<const N: usize>(
    value: &serde_json::Value,
    _node_index: usize,
) -> Result<[f32; N], GlbModelMeshError> {
    let values = value.as_array().ok_or(GlbModelMeshError::InvalidJson)?;
    if values.len() != N {
        return Err(GlbModelMeshError::InvalidJson);
    }
    let mut result = [0.0; N];
    for (index, target) in result.iter_mut().enumerate() {
        let value = values[index]
            .as_f64()
            .ok_or(GlbModelMeshError::InvalidJson)? as f32;
        if !value.is_finite() {
            return Err(GlbModelMeshError::InvalidJson);
        }
        *target = value;
    }
    Ok(result)
}

fn rotate_vec3_by_gltf_quat(vector: Vec3, rotation: [f32; 4]) -> Vec3 {
    let mut x = rotation[0];
    let mut y = rotation[1];
    let mut z = rotation[2];
    let mut w = rotation[3];
    let length = (x * x + y * y + z * z + w * w).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        x = 0.0;
        y = 0.0;
        z = 0.0;
        w = 1.0;
    } else {
        x /= length;
        y /= length;
        z /= length;
        w /= length;
    }
    let q = Vec3 { x, y, z };
    let t = scale_vec3(cross_vec3(q, vector), 2.0);
    add_vec3(add_vec3(vector, scale_vec3(t, w)), cross_vec3(q, t))
}

fn parse_glb_chunks(bytes: &[u8]) -> Result<(&[u8], &[u8]), GlbModelMeshError> {
    if bytes.len() < 12 {
        return Err(GlbModelMeshError::InvalidHeader);
    }
    let magic = read_u32_le(bytes, 0).ok_or(GlbModelMeshError::InvalidHeader)?;
    if magic != GLB_MAGIC {
        return Err(GlbModelMeshError::InvalidHeader);
    }
    let version = read_u32_le(bytes, 4).ok_or(GlbModelMeshError::InvalidHeader)?;
    if version != GLB_VERSION_2 {
        return Err(GlbModelMeshError::UnsupportedVersion { version });
    }
    let declared_length = read_u32_le(bytes, 8).ok_or(GlbModelMeshError::InvalidHeader)? as usize;
    if declared_length > bytes.len() || declared_length < 12 {
        return Err(GlbModelMeshError::InvalidHeader);
    }
    let mut chunks = Vec::new();
    let mut offset = 12;
    while offset < declared_length {
        if offset + 8 > declared_length {
            return Err(GlbModelMeshError::InvalidChunk { offset });
        }
        let chunk_length =
            read_u32_le(bytes, offset).ok_or(GlbModelMeshError::InvalidChunk { offset })? as usize;
        let chunk_type = read_u32_le(bytes, offset + 4)
            .ok_or(GlbModelMeshError::InvalidChunk { offset: offset + 4 })?;
        let data_start = offset + 8;
        let data_end = data_start
            .checked_add(chunk_length)
            .ok_or(GlbModelMeshError::InvalidChunk { offset })?;
        if data_end > declared_length || data_end > bytes.len() {
            return Err(GlbModelMeshError::InvalidChunk { offset });
        }
        chunks.push(GlbChunk {
            chunk_type,
            data: &bytes[data_start..data_end],
        });
        offset = data_end;
    }
    let json = chunks
        .iter()
        .find(|chunk| chunk.chunk_type == GLB_JSON_CHUNK)
        .map(|chunk| chunk.data)
        .ok_or(GlbModelMeshError::MissingJsonChunk)?;
    let bin = chunks
        .iter()
        .find(|chunk| chunk.chunk_type == GLB_BIN_CHUNK)
        .map(|chunk| chunk.data)
        .ok_or(GlbModelMeshError::MissingBinChunk)?;
    Ok((json, bin))
}

fn parse_gltf_accessors(
    json: &serde_json::Value,
) -> Result<Vec<GltfAccessorInfo>, GlbModelMeshError> {
    let accessors = json
        .get("accessors")
        .and_then(|value| value.as_array())
        .ok_or(GlbModelMeshError::MissingPositionAccessor)?;
    accessors
        .iter()
        .enumerate()
        .map(|(index, accessor)| {
            let item_type = accessor
                .get("type")
                .and_then(|value| value.as_str())
                .ok_or(GlbModelMeshError::InvalidAccessor { index })?;
            Ok(GltfAccessorInfo {
                buffer_view: accessor
                    .get("bufferView")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize)
                    .ok_or(GlbModelMeshError::InvalidAccessor { index })?,
                byte_offset: accessor
                    .get("byteOffset")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize,
                component_type: accessor
                    .get("componentType")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as u32)
                    .ok_or(GlbModelMeshError::InvalidAccessor { index })?,
                count: accessor
                    .get("count")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize)
                    .ok_or(GlbModelMeshError::InvalidAccessor { index })?,
                item_type: item_type.to_string(),
            })
        })
        .collect()
}

fn parse_gltf_buffer_views(
    json: &serde_json::Value,
) -> Result<Vec<GltfBufferViewInfo>, GlbModelMeshError> {
    let buffer_views = json
        .get("bufferViews")
        .and_then(|value| value.as_array())
        .ok_or(GlbModelMeshError::MissingPositionAccessor)?;
    buffer_views
        .iter()
        .enumerate()
        .map(|(index, view)| {
            Ok(GltfBufferViewInfo {
                buffer: view
                    .get("buffer")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize,
                byte_offset: view
                    .get("byteOffset")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize,
                byte_length: view
                    .get("byteLength")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize)
                    .ok_or(GlbModelMeshError::InvalidBufferView { index })?,
                byte_stride: view
                    .get("byteStride")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as usize),
            })
        })
        .collect()
}

fn read_gltf_positions(
    bin: &[u8],
    accessors: &[GltfAccessorInfo],
    buffer_views: &[GltfBufferViewInfo],
    accessor_index: usize,
) -> Result<Vec<Vec3>, GlbModelMeshError> {
    let accessor = accessors
        .get(accessor_index)
        .ok_or(GlbModelMeshError::InvalidAccessor {
            index: accessor_index,
        })?;
    if accessor.component_type != 5126 || accessor.item_type != "VEC3" {
        return Err(GlbModelMeshError::UnsupportedPositionAccessor {
            index: accessor_index,
        });
    }
    let view =
        *buffer_views
            .get(accessor.buffer_view)
            .ok_or(GlbModelMeshError::InvalidBufferView {
                index: accessor.buffer_view,
            })?;
    if view.buffer != 0 {
        return Err(GlbModelMeshError::InvalidBufferView {
            index: accessor.buffer_view,
        });
    }
    let stride = view.byte_stride.unwrap_or(12);
    if stride < 12 {
        return Err(GlbModelMeshError::UnsupportedPositionAccessor {
            index: accessor_index,
        });
    }
    let start = view.byte_offset + accessor.byte_offset;
    let view_end = view.byte_offset.checked_add(view.byte_length).ok_or(
        GlbModelMeshError::InvalidBufferView {
            index: accessor.buffer_view,
        },
    )?;
    let mut positions = Vec::with_capacity(accessor.count);
    for item in 0..accessor.count {
        let offset = start + item * stride;
        if offset + 12 > view_end || offset + 12 > bin.len() {
            return Err(GlbModelMeshError::InvalidAccessor {
                index: accessor_index,
            });
        }
        let x = read_f32_le(bin, offset).ok_or(GlbModelMeshError::InvalidAccessor {
            index: accessor_index,
        })?;
        let y = read_f32_le(bin, offset + 4).ok_or(GlbModelMeshError::InvalidAccessor {
            index: accessor_index,
        })?;
        let z = read_f32_le(bin, offset + 8).ok_or(GlbModelMeshError::InvalidAccessor {
            index: accessor_index,
        })?;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(GlbModelMeshError::InvalidAccessor {
                index: accessor_index,
            });
        }
        positions.push(Vec3 { x, y, z });
    }
    Ok(positions)
}

fn read_gltf_indices(
    bin: &[u8],
    accessors: &[GltfAccessorInfo],
    buffer_views: &[GltfBufferViewInfo],
    accessor_index: usize,
) -> Result<Vec<u32>, GlbModelMeshError> {
    let accessor = accessors
        .get(accessor_index)
        .ok_or(GlbModelMeshError::InvalidAccessor {
            index: accessor_index,
        })?;
    if accessor.item_type != "SCALAR" {
        return Err(GlbModelMeshError::UnsupportedIndexAccessor {
            index: accessor_index,
        });
    }
    let component_size = match accessor.component_type {
        5121 => 1,
        5123 => 2,
        5125 => 4,
        _ => {
            return Err(GlbModelMeshError::UnsupportedIndexAccessor {
                index: accessor_index,
            })
        }
    };
    let view =
        *buffer_views
            .get(accessor.buffer_view)
            .ok_or(GlbModelMeshError::InvalidBufferView {
                index: accessor.buffer_view,
            })?;
    if view.buffer != 0 {
        return Err(GlbModelMeshError::InvalidBufferView {
            index: accessor.buffer_view,
        });
    }
    let stride = view.byte_stride.unwrap_or(component_size);
    if stride < component_size {
        return Err(GlbModelMeshError::UnsupportedIndexAccessor {
            index: accessor_index,
        });
    }
    let start = view.byte_offset + accessor.byte_offset;
    let view_end = view.byte_offset.checked_add(view.byte_length).ok_or(
        GlbModelMeshError::InvalidBufferView {
            index: accessor.buffer_view,
        },
    )?;
    let mut indices = Vec::with_capacity(accessor.count);
    for item in 0..accessor.count {
        let offset = start + item * stride;
        if offset + component_size > view_end || offset + component_size > bin.len() {
            return Err(GlbModelMeshError::InvalidAccessor {
                index: accessor_index,
            });
        }
        let index = match accessor.component_type {
            5121 => bin[offset] as u32,
            5123 => read_u16_le(bin, offset).ok_or(GlbModelMeshError::InvalidAccessor {
                index: accessor_index,
            })? as u32,
            5125 => read_u32_le(bin, offset).ok_or(GlbModelMeshError::InvalidAccessor {
                index: accessor_index,
            })?,
            _ => unreachable!(),
        };
        indices.push(index);
    }
    Ok(indices)
}

fn parse_three_ds_meshes(bytes: &[u8]) -> Result<Vec<ThreeDsMeshData>, ThreeDsModelMeshError> {
    let mut meshes = Vec::new();
    parse_three_ds_chunks(bytes, 0, bytes.len(), &mut meshes)?;
    Ok(meshes)
}

fn parse_three_ds_chunks(
    bytes: &[u8],
    start: usize,
    end: usize,
    meshes: &mut Vec<ThreeDsMeshData>,
) -> Result<(), ThreeDsModelMeshError> {
    let mut offset = start;
    while offset < end {
        let chunk = read_three_ds_chunk(bytes, offset, end)?;
        match chunk.id {
            THREE_DS_TRIANGULAR_MESH_CHUNK => {
                if let Some(mesh) =
                    parse_three_ds_triangular_mesh(bytes, chunk.payload_start, chunk.end)?
                {
                    meshes.push(mesh);
                }
            }
            THREE_DS_OBJECT_BLOCK_CHUNK => {
                let object_payload = skip_three_ds_c_string(bytes, chunk.payload_start, chunk.end);
                parse_three_ds_chunks(bytes, object_payload, chunk.end, meshes)?;
            }
            THREE_DS_MAIN_CHUNK | THREE_DS_EDITOR_CHUNK => {
                parse_three_ds_chunks(bytes, chunk.payload_start, chunk.end, meshes)?;
            }
            _ => {}
        }
        offset = chunk.end;
    }
    Ok(())
}

fn parse_three_ds_triangular_mesh(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<Option<ThreeDsMeshData>, ThreeDsModelMeshError> {
    let mut positions = Vec::new();
    let mut faces = Vec::new();
    let mut transform = None;
    let mut offset = start;
    while offset < end {
        let chunk = read_three_ds_chunk(bytes, offset, end)?;
        match chunk.id {
            THREE_DS_VERTEX_LIST_CHUNK => {
                positions = parse_three_ds_vertices(bytes, chunk.payload_start, chunk.end)?;
            }
            THREE_DS_FACE_LIST_CHUNK => {
                faces =
                    parse_three_ds_faces(bytes, chunk.payload_start, chunk.end, positions.len())?;
            }
            THREE_DS_MESH_MATRIX_CHUNK => {
                transform = Some(parse_three_ds_mesh_transform(
                    bytes,
                    chunk.payload_start,
                    chunk.end,
                )?);
            }
            _ => {}
        }
        offset = chunk.end;
    }
    Ok(
        (!positions.is_empty() || !faces.is_empty()).then_some(ThreeDsMeshData {
            positions,
            faces,
            transform,
        }),
    )
}

fn parse_three_ds_vertices(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<Vec<Vec3>, ThreeDsModelMeshError> {
    let Some(count) = read_u16_le(bytes, start) else {
        return Err(ThreeDsModelMeshError::InvalidVertexList { offset: start });
    };
    let mut offset = start + 2;
    let mut positions = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if offset + 12 > end {
            return Err(ThreeDsModelMeshError::InvalidVertexList { offset });
        }
        let x = read_f32_le(bytes, offset)
            .ok_or(ThreeDsModelMeshError::InvalidVertexList { offset })?;
        let y = read_f32_le(bytes, offset + 4)
            .ok_or(ThreeDsModelMeshError::InvalidVertexList { offset: offset + 4 })?;
        let z = read_f32_le(bytes, offset + 8)
            .ok_or(ThreeDsModelMeshError::InvalidVertexList { offset: offset + 8 })?;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(ThreeDsModelMeshError::InvalidVertexList { offset });
        }
        positions.push(Vec3 { x, y, z });
        offset += 12;
    }
    Ok(positions)
}

fn parse_three_ds_faces(
    bytes: &[u8],
    start: usize,
    end: usize,
    position_count: usize,
) -> Result<Vec<[usize; 3]>, ThreeDsModelMeshError> {
    let Some(count) = read_u16_le(bytes, start) else {
        return Err(ThreeDsModelMeshError::InvalidFaceList { offset: start });
    };
    let mut offset = start + 2;
    let mut faces = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if offset + 8 > end {
            return Err(ThreeDsModelMeshError::InvalidFaceList { offset });
        }
        let a =
            read_u16_le(bytes, offset).ok_or(ThreeDsModelMeshError::InvalidFaceList { offset })?;
        let b = read_u16_le(bytes, offset + 2)
            .ok_or(ThreeDsModelMeshError::InvalidFaceList { offset: offset + 2 })?;
        let c = read_u16_le(bytes, offset + 4)
            .ok_or(ThreeDsModelMeshError::InvalidFaceList { offset: offset + 4 })?;
        for index in [a, b, c] {
            if index as usize >= position_count {
                return Err(ThreeDsModelMeshError::FaceIndexOutOfRange { offset, index });
            }
        }
        faces.push([a as usize, b as usize, c as usize]);
        offset += 8;
    }
    Ok(faces)
}

fn parse_three_ds_mesh_transform(
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<ThreeDsMeshTransform, ThreeDsModelMeshError> {
    if start + 48 > end {
        return Err(ThreeDsModelMeshError::InvalidChunkLength { offset: start });
    }
    let mut values = [0.0; 12];
    for (index, value) in values.iter_mut().enumerate() {
        let offset = start + index * 4;
        *value = read_f32_le(bytes, offset)
            .ok_or(ThreeDsModelMeshError::InvalidChunkLength { offset })?;
        if !value.is_finite() {
            return Err(ThreeDsModelMeshError::InvalidChunkLength { offset });
        }
    }
    Ok(ThreeDsMeshTransform {
        right: Vec3 {
            x: values[0],
            y: values[1],
            z: values[2],
        },
        up: Vec3 {
            x: values[3],
            y: values[4],
            z: values[5],
        },
        direction: Vec3 {
            x: values[6],
            y: values[7],
            z: values[8],
        },
        origin: Vec3 {
            x: values[9],
            y: values[10],
            z: values[11],
        },
    })
}

fn transform_three_ds_position(position: Vec3, transform: Option<ThreeDsMeshTransform>) -> Vec3 {
    let Some(transform) = transform else {
        return position;
    };
    add_vec3(
        add_vec3(
            add_vec3(transform.origin, scale_vec3(transform.right, position.x)),
            scale_vec3(transform.up, position.y),
        ),
        scale_vec3(transform.direction, position.z),
    )
}

fn read_three_ds_chunk(
    bytes: &[u8],
    offset: usize,
    parent_end: usize,
) -> Result<ThreeDsChunk, ThreeDsModelMeshError> {
    if offset + 6 > parent_end || offset + 6 > bytes.len() {
        return Err(ThreeDsModelMeshError::InvalidChunkHeader { offset });
    }
    let id =
        read_u16_le(bytes, offset).ok_or(ThreeDsModelMeshError::InvalidChunkHeader { offset })?;
    let length = read_u32_le(bytes, offset + 2)
        .ok_or(ThreeDsModelMeshError::InvalidChunkHeader { offset })? as usize;
    if length < 6 {
        return Err(ThreeDsModelMeshError::InvalidChunkLength { offset });
    }
    let end = offset
        .checked_add(length)
        .ok_or(ThreeDsModelMeshError::InvalidChunkLength { offset })?;
    if end > parent_end || end > bytes.len() {
        return Err(ThreeDsModelMeshError::InvalidChunkLength { offset });
    }
    Ok(ThreeDsChunk {
        id,
        payload_start: offset + 6,
        end,
    })
}

fn skip_three_ds_c_string(bytes: &[u8], start: usize, end: usize) -> usize {
    bytes[start..end]
        .iter()
        .position(|byte| *byte == 0)
        .map(|index| start + index + 1)
        .unwrap_or(end)
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let data = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([data[0], data[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let data = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn read_f32_le(bytes: &[u8], offset: usize) -> Option<f32> {
    let data = bytes.get(offset..offset + 4)?;
    Some(f32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn model_mesh_kind(
    primitive: Option<&str>,
    model_file: Option<&str>,
    dimensions: Option<Vec3>,
) -> GeometryModelMeshKind {
    let primitive = primitive.map(|value| {
        value
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '_', '-'], "")
    });
    if let Some(primitive) = primitive.as_deref().filter(|value| !value.is_empty()) {
        if matches!(primitive, "none" | "undefined" | "null") {
            return GeometryModelMeshKind::None;
        }
        if primitive.contains("cylinder")
            || primitive.contains("tube")
            || primitive.contains("barrel")
        {
            return GeometryModelMeshKind::Cylinder;
        }
        if primitive.contains("sphere") || primitive.contains("ball") {
            return GeometryModelMeshKind::Sphere;
        }
        if primitive.contains("plane")
            || primitive.contains("quad")
            || primitive.contains("rectangle")
        {
            return GeometryModelMeshKind::Plane;
        }
        if primitive.contains("cube")
            || primitive.contains("box")
            || primitive.contains("base")
            || primitive.contains("yoke")
            || primitive.contains("head")
        {
            return GeometryModelMeshKind::Box;
        }
        return GeometryModelMeshKind::Unknown;
    }

    if model_file
        .and_then(|file| (!file.trim().is_empty()).then_some(file))
        .is_some()
    {
        return GeometryModelMeshKind::Mesh;
    }
    if dimensions
        .filter(|dimensions| {
            dimensions.x.is_finite()
                && dimensions.y.is_finite()
                && dimensions.z.is_finite()
                && (dimensions.x > 0.0 || dimensions.y > 0.0 || dimensions.z > 0.0)
        })
        .is_some()
    {
        return GeometryModelMeshKind::Box;
    }
    GeometryModelMeshKind::None
}

fn mapped_channel_counts_by_geometry(fixture: &PatchedFixtureSummary) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for control in &fixture.controls {
        let Some(geometry) = control
            .geometry
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        *counts.entry(geometry).or_insert(0) += 1;
    }
    counts
}

fn cumulative_geometry_matrix(
    geometry: &GeometrySummary,
    geometry_by_name: &HashMap<&str, &GeometrySummary>,
) -> [f32; 16] {
    let mut lineage = Vec::new();
    let mut current = Some(geometry);
    let mut seen = HashSet::new();

    while let Some(node) = current {
        if !seen.insert(node.name.as_str()) || lineage.len() >= 32 {
            break;
        }
        lineage.push(node.matrix);
        current = node
            .parent
            .as_deref()
            .and_then(|parent| geometry_by_name.get(parent).copied());
    }

    lineage
        .iter()
        .rev()
        .fold(identity_matrix(), |accumulated, matrix| {
            multiply_matrix(accumulated, *matrix)
        })
}

fn multiply_matrix(left: [f32; 16], right: [f32; 16]) -> [f32; 16] {
    let mut result = [0.0; 16];
    for row in 0..4 {
        for column in 0..4 {
            result[row * 4 + column] = (0..4)
                .map(|index| left[row * 4 + index] * right[index * 4 + column])
                .sum();
        }
    }
    result
}

fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

fn matrix_translation(matrix: [f32; 16]) -> Vec3 {
    Vec3 {
        x: finite_or(matrix[3], 0.0),
        y: finite_or(matrix[7], 0.0),
        z: finite_or(matrix[11], 0.0),
    }
}

fn matrix_right(matrix: [f32; 16]) -> Vec3 {
    normalize(Vec3 {
        x: finite_or(matrix[0], 1.0),
        y: finite_or(matrix[4], 0.0),
        z: finite_or(matrix[8], 0.0),
    })
}

fn matrix_up(matrix: [f32; 16]) -> Vec3 {
    normalize(Vec3 {
        x: finite_or(matrix[1], 0.0),
        y: finite_or(matrix[5], 1.0),
        z: finite_or(matrix[9], 0.0),
    })
}

fn matrix_forward(matrix: [f32; 16]) -> Vec3 {
    normalize(Vec3 {
        x: finite_or(matrix[2], 0.0),
        y: finite_or(matrix[6], 0.0),
        z: finite_or(matrix[10], 1.0),
    })
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

fn sub_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3 {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn scale_vec3(vector: Vec3, scale: f32) -> Vec3 {
    Vec3 {
        x: vector.x * scale,
        y: vector.y * scale,
        z: vector.z * scale,
    }
}

fn cross_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3 {
        x: left.y * right.z - left.z * right.y,
        y: left.z * right.x - left.x * right.z,
        z: left.x * right.y - left.y * right.x,
    }
}

fn rotate_vec3(vector: Vec3, pitch_deg: f32, yaw_deg: f32, roll_deg: f32) -> Vec3 {
    let pitch = finite_or(pitch_deg, 0.0).to_radians();
    let yaw = finite_or(yaw_deg, 0.0).to_radians();
    let roll = finite_or(roll_deg, 0.0).to_radians();

    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let (sin_roll, cos_roll) = roll.sin_cos();

    let pitched = Vec3 {
        x: vector.x,
        y: vector.y * cos_pitch - vector.z * sin_pitch,
        z: vector.y * sin_pitch + vector.z * cos_pitch,
    };
    let yawed = Vec3 {
        x: pitched.x * cos_yaw + pitched.z * sin_yaw,
        y: pitched.y,
        z: -pitched.x * sin_yaw + pitched.z * cos_yaw,
    };
    Vec3 {
        x: yawed.x * cos_roll - yawed.y * sin_roll,
        y: yawed.x * sin_roll + yawed.y * cos_roll,
        z: yawed.z,
    }
}

fn fixture_node_from_summary(
    fixture: &PatchedFixtureSummary,
    dmx_frames: &HashMap<u16, Vec<u8>>,
) -> FixtureNode {
    let dimmer = read_dmx_attribute(fixture, &["Dimmer", "Intensity"], dmx_frames)
        .or_else(|| read_attribute(fixture, &["Dimmer", "Intensity"]))
        .unwrap_or(0);
    let pan_deg = read_dmx_attribute(fixture, &["Pan"], dmx_frames)
        .or_else(|| read_attribute(fixture, &["Pan"]))
        .map(|value| ((value as f32 - 32_768.0) / FULL_SCALE) * 540.0)
        .unwrap_or(0.0);
    let tilt_deg = read_dmx_attribute(fixture, &["Tilt"], dmx_frames)
        .or_else(|| read_attribute(fixture, &["Tilt"]))
        .map(|value| ((value as f32 - 32_768.0) / FULL_SCALE) * 270.0)
        .unwrap_or(0.0);
    let red = read_dmx_attribute(fixture, &["ColorRed", "Red", "ColorAdd_R"], dmx_frames)
        .or_else(|| read_attribute(fixture, &["ColorRed", "Red", "ColorAdd_R"]))
        .map(normalized_u16)
        .unwrap_or(0.34);
    let green = read_dmx_attribute(fixture, &["ColorGreen", "Green", "ColorAdd_G"], dmx_frames)
        .or_else(|| read_attribute(fixture, &["ColorGreen", "Green", "ColorAdd_G"]))
        .map(normalized_u16)
        .unwrap_or(0.65);
    let blue = read_dmx_attribute(fixture, &["ColorBlue", "Blue", "ColorAdd_B"], dmx_frames)
        .or_else(|| read_attribute(fixture, &["ColorBlue", "Blue", "ColorAdd_B"]))
        .map(normalized_u16)
        .unwrap_or(0.96);
    let white = read_dmx_attribute(fixture, &["ColorWhite", "White", "ColorAdd_W"], dmx_frames)
        .or_else(|| read_attribute(fixture, &["ColorWhite", "White", "ColorAdd_W"]))
        .map(normalized_u16)
        .unwrap_or(0.0);

    FixtureNode {
        id: fixture.id,
        label: fixture.label.clone(),
        position: fixture.position,
        yaw_deg: fixture.rotation.yaw + pan_deg,
        pitch_deg: fixture.rotation.pitch + tilt_deg,
        roll_deg: fixture.rotation.roll,
        color: [
            (red + white).min(1.0),
            (green + white).min(1.0),
            (blue + white).min(1.0),
        ],
        intensity: normalized_u16(dimmer),
    }
}

fn dmx_preview_frames(snapshot: &EngineSnapshot) -> HashMap<u16, Vec<u8>> {
    let mut frames = snapshot
        .dmx_previews
        .iter()
        .map(|preview| (preview.universe, preview.values.clone()))
        .collect::<HashMap<_, _>>();
    frames
        .entry(snapshot.output.universe)
        .or_insert_with(|| snapshot.dmx_preview.clone());
    frames
}

fn read_dmx_attribute(
    fixture: &PatchedFixtureSummary,
    names: &[&str],
    dmx_frames: &HashMap<u16, Vec<u8>>,
) -> Option<u16> {
    fixture
        .controls
        .iter()
        .find(|control| {
            names
                .iter()
                .any(|name| control.attribute.eq_ignore_ascii_case(name))
        })
        .and_then(|control| read_dmx_control(fixture, control, dmx_frames))
}

fn read_dmx_control(
    fixture: &PatchedFixtureSummary,
    control: &AttributeControl,
    dmx_frames: &HashMap<u16, Vec<u8>>,
) -> Option<u16> {
    let frame = dmx_frames.get(&fixture.universe)?;
    let coarse_offset = *control.offsets.first()?;
    let coarse_index = dmx_index(fixture.address, coarse_offset)?;
    let coarse = *frame.get(coarse_index)? as u16;
    if control.resolution == protocol::AttributeResolution::SixteenBit {
        let fine = control
            .offsets
            .get(1)
            .and_then(|offset| dmx_index(fixture.address, *offset))
            .and_then(|index| frame.get(index).copied())
            .unwrap_or(0) as u16;
        Some((coarse << 8) | fine)
    } else {
        Some(coarse * 257)
    }
}

fn dmx_index(address: u16, offset: u16) -> Option<usize> {
    let channel = address.checked_add(offset)?.checked_sub(1)?;
    (channel <= 512).then_some(channel as usize - 1)
}

fn read_attribute(fixture: &PatchedFixtureSummary, names: &[&str]) -> Option<u16> {
    fixture.attribute_values.iter().find_map(|value| {
        names
            .iter()
            .any(|name| value.attribute.eq_ignore_ascii_case(name))
            .then_some(value.value)
    })
}

fn normalized_u16(value: u16) -> f32 {
    (value as f32 / FULL_SCALE).clamp(0.0, 1.0)
}

fn beam_direction(yaw_deg: f32, pitch_deg: f32) -> Vec3 {
    let yaw = yaw_deg.to_radians();
    let pitch = pitch_deg.to_radians();
    let horizontal = pitch.cos();
    normalize(Vec3 {
        x: yaw.sin() * horizontal,
        y: -pitch.sin(),
        z: yaw.cos() * horizontal,
    })
}

fn normalize(vector: Vec3) -> Vec3 {
    let length = (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt();
    if length <= f32::EPSILON {
        return Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
    }
    Vec3 {
        x: vector.x / length,
        y: vector.y / length,
        z: vector.z / length,
    }
}

fn stage_bounds(
    fixtures: &[PatchedFixtureSummary],
    fixture_geometries: &[FixtureGeometryNode],
    fixture_models: &[FixtureModelNode],
    video_surfaces: &[VideoSurfaceNode],
    stage_objects: &[StageObjectNode],
) -> StageBounds {
    let model_bounds = fixture_models.iter().flat_map(|model| {
        let radius = model.bounding_radius.max(0.0);
        [
            Vec3 {
                x: model.position.x - radius,
                y: model.position.y - radius,
                z: model.position.z - radius,
            },
            Vec3 {
                x: model.position.x + radius,
                y: model.position.y + radius,
                z: model.position.z + radius,
            },
        ]
    });
    let object_bounds = stage_objects.iter().flat_map(stage_object_bounds_points);
    let mut positions = fixtures
        .iter()
        .map(|fixture| fixture.position)
        .chain(fixture_geometries.iter().map(|geometry| geometry.position))
        .chain(model_bounds)
        .chain(video_surfaces.iter().map(|surface| surface.position))
        .chain(object_bounds);
    let Some(first) = positions.next() else {
        return StageBounds {
            min: Vec3::default(),
            max: Vec3::default(),
        };
    };
    positions.fold(
        StageBounds {
            min: first,
            max: first,
        },
        |bounds, position| StageBounds {
            min: Vec3 {
                x: bounds.min.x.min(position.x),
                y: bounds.min.y.min(position.y),
                z: bounds.min.z.min(position.z),
            },
            max: Vec3 {
                x: bounds.max.x.max(position.x),
                y: bounds.max.y.max(position.y),
                z: bounds.max.z.max(position.z),
            },
        },
    )
}

fn stage_object_bounds_points(object: &StageObjectNode) -> [Vec3; 4] {
    let half_width = object.width.max(0.0) / 2.0;
    let half_depth = object.depth.max(0.0) / 2.0;
    let radians = object.rotation_deg.to_radians();
    let (sin, cos) = radians.sin_cos();
    [
        (-half_width, -half_depth),
        (half_width, -half_depth),
        (half_width, half_depth),
        (-half_width, half_depth),
    ]
    .map(|(local_x, local_z)| Vec3 {
        x: object.position.x + local_x * cos - local_z * sin,
        y: object.position.y,
        z: object.position.z + local_x * sin + local_z * cos,
    })
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{
        AttributeControl, AttributeResolution, AttributeValueSummary, DmxUniversePreview,
        EngineSnapshot, GeometrySummary, Rotation3, StageObjectKind, StageObjectSummary,
        VideoOutputAspectMode, VideoOutputKind, VideoOutputMapping, VideoOutputSummary,
        VideoSnapshot,
    };

    #[test]
    fn builds_fixture_nodes_and_beams_from_engine_snapshot() {
        let snapshot = snapshot_with_dmx_preview(vec![fixture(
            1,
            "Key",
            Vec3 {
                x: -2.0,
                y: 5.0,
                z: 1.0,
            },
            vec![
                ("Dimmer", 65_535),
                ("Pan", 32_768),
                ("Tilt", 32_768),
                ("ColorRed", 65_535),
                ("ColorGreen", 0),
                ("ColorBlue", 0),
            ],
        )]);

        let scene = build_visualizer_scene(&snapshot, VisualizerConfig::default());

        assert_eq!(scene.fixtures.len(), 1);
        assert_eq!(scene.beams.len(), 1);
        assert_eq!(scene.fixtures[0].label, "Key");
        assert_eq!(scene.fixtures[0].color, [1.0, 0.0, 0.0]);
        assert_eq!(scene.fixtures[0].intensity, 1.0);
        assert!((scene.beams[0].direction.z - 1.0).abs() < 0.001);
        assert_eq!(scene.bounds.min.x, -2.0);
        assert_eq!(scene.bounds.max.y, 5.0);
    }

    #[test]
    fn dmx_preview_overrides_stale_fixture_attribute_values() {
        let mut fixture = fixture(
            1,
            "Live",
            Vec3::default(),
            vec![
                ("Dimmer", 0),
                ("Pan", 32_768),
                ("Tilt", 32_768),
                ("ColorRed", 0),
                ("ColorGreen", 0),
                ("ColorBlue", 65_535),
            ],
        );
        let mut frame = vec![0; 512];
        frame[0] = 255;
        frame[3] = 255;
        fixture.attribute_values = vec![
            AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 0,
            },
            AttributeValueSummary {
                attribute: "ColorRed".to_string(),
                value: 0,
            },
        ];
        let snapshot = EngineSnapshot {
            fixtures: vec![fixture],
            dmx_preview: frame.clone(),
            dmx_previews: vec![DmxUniversePreview {
                universe: 0,
                values: frame,
            }],
            ..EngineSnapshot::default()
        };

        let scene = build_visualizer_scene(&snapshot, VisualizerConfig::default());

        assert_eq!(scene.fixtures[0].intensity, 1.0);
        assert_eq!(scene.fixtures[0].color, [1.0, 0.0, 0.0]);
        assert_eq!(scene.beams.len(), 1);
    }

    #[test]
    fn omits_beams_below_configured_intensity_threshold() {
        let snapshot = snapshot_with_dmx_preview(vec![fixture(
            1,
            "Dim",
            Vec3::default(),
            vec![("Dimmer", 10), ("Pan", 32_768), ("Tilt", 32_768)],
        )]);

        let scene = build_visualizer_scene(
            &snapshot,
            VisualizerConfig {
                minimum_beam_intensity: 0.5,
                ..VisualizerConfig::default()
            },
        );

        assert_eq!(scene.fixtures.len(), 1);
        assert!(scene.beams.is_empty());
    }

    #[test]
    fn includes_video_output_surfaces_in_stage_scene() {
        let snapshot = EngineSnapshot {
            video: VideoSnapshot {
                outputs: vec![VideoOutputSummary {
                    id: 7,
                    label: "Main Projector".to_string(),
                    kind: VideoOutputKind::Display,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: true,
                    monitor_id: Some(1),
                    width: 1920,
                    height: 1080,
                    endpoint_name: None,
                    opacity: 0.65,
                    blackout: false,
                    mapping: VideoOutputMapping {
                        stage_x: 3.0,
                        stage_y: 2.0,
                        stage_z: -4.0,
                        scale_x: 1.5,
                        scale_y: 0.5,
                        rotation_deg: 15.0,
                        aspect_ratio: 16.0 / 9.0,
                        aspect_mode: VideoOutputAspectMode::Fit,
                        ..Default::default()
                    },
                }],
                ..VideoSnapshot::default()
            },
            ..EngineSnapshot::default()
        };

        let scene = build_visualizer_scene(&snapshot, VisualizerConfig::default());

        assert!(scene.fixtures.is_empty());
        assert_eq!(scene.video_surfaces.len(), 1);
        let surface = &scene.video_surfaces[0];
        assert_eq!(surface.id, 7);
        assert_eq!(surface.label, "Main Projector");
        assert_eq!(
            surface.position,
            Vec3 {
                x: 3.0,
                y: 2.0,
                z: -4.0
            }
        );
        assert!((surface.width - 5.333_333).abs() < 0.001);
        assert!((surface.height - 1.0).abs() < 0.001);
        assert_eq!(surface.rotation_deg, 15.0);
        assert_eq!(
            scene.bounds.min,
            Vec3 {
                x: 3.0,
                y: 2.0,
                z: -4.0
            }
        );
        assert_eq!(
            scene.bounds.max,
            Vec3 {
                x: 3.0,
                y: 2.0,
                z: -4.0
            }
        );
    }

    #[test]
    fn includes_stage_reference_objects_in_stage_scene() {
        let snapshot = EngineSnapshot {
            stage_objects: vec![StageObjectSummary {
                id: 3,
                label: "Front Truss".to_string(),
                kind: StageObjectKind::Truss,
                x: -2.0,
                z: 5.0,
                width: 4.0,
                depth: 1.0,
                rotation_deg: 0.0,
                color: Some("#f2c14e".to_string()),
            }],
            ..EngineSnapshot::default()
        };

        let scene = build_visualizer_scene(&snapshot, VisualizerConfig::default());

        assert!(scene.fixtures.is_empty());
        assert_eq!(scene.stage_objects.len(), 1);
        let object = &scene.stage_objects[0];
        assert_eq!(object.id, 3);
        assert_eq!(object.label, "Front Truss");
        assert_eq!(object.kind, StageObjectKind::Truss);
        assert_eq!(
            object.position,
            Vec3 {
                x: -2.0,
                y: 0.0,
                z: 5.0
            }
        );
        assert_eq!(object.color.as_deref(), Some("#f2c14e"));
        assert_eq!(
            scene.bounds.min,
            Vec3 {
                x: -4.0,
                y: 0.0,
                z: 4.5
            }
        );
        assert_eq!(
            scene.bounds.max,
            Vec3 {
                x: 0.0,
                y: 0.0,
                z: 5.5
            }
        );
    }

    #[test]
    fn includes_gdtf_geometry_nodes_in_stage_scene() {
        let mut fixture = fixture(
            1,
            "Mover",
            Vec3 {
                x: 10.0,
                y: 1.0,
                z: -2.0,
            },
            vec![("Dimmer", 65_535), ("Pan", 32_768), ("Tilt", 32_768)],
        );
        fixture.controls[0].geometry = Some("Lens".to_string());
        fixture.geometries = vec![
            geometry("Body", "Geometry", None, 0.0, 0.0, 0.0),
            geometry("Head", "Axis", Some("Body"), 1.0, 2.0, 3.0),
            geometry("Lens", "Beam", Some("Head"), 0.0, 0.5, 1.0),
        ];

        let scene = build_visualizer_scene(
            &snapshot_with_dmx_preview(vec![fixture]),
            VisualizerConfig::default(),
        );

        assert_eq!(scene.fixture_geometries.len(), 3);
        let lens = scene
            .fixture_geometries
            .iter()
            .find(|geometry| geometry.name == "Lens")
            .unwrap();
        assert_eq!(lens.kind, "Beam");
        assert_eq!(lens.parent, Some("Head".to_string()));
        assert_eq!(lens.mapped_channel_count, 1);
        assert_eq!(
            lens.local_position,
            Vec3 {
                x: 1.0,
                y: 2.5,
                z: 4.0,
            }
        );
        assert_eq!(
            lens.position,
            Vec3 {
                x: 11.0,
                y: 3.5,
                z: 2.0,
            }
        );
        assert_eq!(
            lens.local_right,
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }
        );
        assert_eq!(
            lens.local_up,
            Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            }
        );
        assert_eq!(scene.beams[0].geometry_name, Some("Lens".to_string()));
        assert_eq!(scene.beams[0].origin, lens.position);
        assert_eq!(scene.bounds.max.y, 3.5);
    }

    #[test]
    fn uses_gdtf_beam_geometry_orientation_for_beam_direction() {
        let mut fixture = fixture(1, "Side Emitter", Vec3::default(), vec![("Dimmer", 65_535)]);
        fixture.geometries = vec![GeometrySummary {
            name: "SideLens".to_string(),
            kind: "Beam".to_string(),
            parent: None,
            matrix: [
                0.0, 0.0, 1.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                -1.0, 0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
            model_name: Some("SideLensModel".to_string()),
            model_file: Some("models/side-lens.glb".to_string()),
            model_primitive: Some("Cylinder".to_string()),
            model_dimensions: Some(Vec3 {
                x: 0.2,
                y: 0.1,
                z: 0.2,
            }),
            beam_type: Some("Wash".to_string()),
            beam_angle_deg: Some(10.0),
            field_angle_deg: None,
            beam_radius: Some(0.2),
        }];

        let scene = build_visualizer_scene(
            &snapshot_with_dmx_preview(vec![fixture]),
            VisualizerConfig::default(),
        );

        assert_eq!(scene.beams[0].geometry_name, Some("SideLens".to_string()));
        assert_eq!(scene.beams[0].beam_type.as_deref(), Some("Wash"));
        assert_eq!(scene.beams[0].beam_angle_deg, Some(10.0));
        assert_eq!(
            scene.fixture_geometries[0].model_file.as_deref(),
            Some("models/side-lens.glb")
        );
        assert_eq!(
            scene.fixture_geometries[0].model_mesh_kind,
            GeometryModelMeshKind::Cylinder
        );
        assert_eq!(
            scene.fixture_geometries[0].model_dimensions,
            Some(Vec3 {
                x: 0.2,
                y: 0.1,
                z: 0.2,
            })
        );
        assert_eq!(scene.fixture_models.len(), 1);
        let model = &scene.fixture_models[0];
        assert_eq!(model.fixture_id, 1);
        assert_eq!(model.profile_source_path, "memory://visualizer.fixture");
        assert_eq!(model.geometry_name, "SideLens");
        assert_eq!(model.mesh_kind, GeometryModelMeshKind::Cylinder);
        assert_eq!(model.model_file.as_deref(), Some("models/side-lens.glb"));
        assert_eq!(
            model.dimensions,
            Some(Vec3 {
                x: 0.2,
                y: 0.1,
                z: 0.2,
            })
        );
        assert!((model.bounding_radius - 0.15).abs() < 0.001);
        assert!((model.direction.x - 1.0).abs() < 0.001);
        assert!((scene.bounds.max.x - 0.15).abs() < 0.001);
        assert!((scene.bounds.min.x + 0.15).abs() < 0.001);
        let render_plans = build_fixture_model_render_plans(&scene);
        assert_eq!(render_plans.len(), 1);
        assert_eq!(
            render_plans[0].draw_kind,
            FixtureModelDrawKind::ExternalMesh
        );
        assert_eq!(
            render_plans[0].fallback_mesh_kind,
            GeometryModelMeshKind::Cylinder
        );
        assert_eq!(
            render_plans[0].model_file.as_deref(),
            Some("models/side-lens.glb")
        );
        assert_eq!(
            render_plans[0].profile_source_path,
            "memory://visualizer.fixture"
        );
        assert!((scene.fixture_geometries[0].local_direction.x - 1.0).abs() < 0.001);
        assert!((scene.fixture_geometries[0].local_right.z + 1.0).abs() < 0.001);
        assert!((scene.fixture_geometries[0].local_up.y - 1.0).abs() < 0.001);
        assert!((scene.beams[0].direction.x - 1.0).abs() < 0.001);
        assert!(scene.beams[0].direction.z.abs() < 0.001);
        let expected_radius = 0.2 + (10.0_f32.to_radians() * 0.5).tan() * 12.0;
        assert!((scene.beams[0].radius - expected_radius).abs() < 0.001);
    }

    #[test]
    fn classifies_fixture_model_render_plans_for_primitives_and_fallbacks() {
        let sphere = fixture_model_node(GeometryModelMeshKind::Sphere, None, Some(unit_vec3()));
        let unknown = fixture_model_node(GeometryModelMeshKind::Unknown, None, Some(unit_vec3()));
        let missing = fixture_model_node(GeometryModelMeshKind::Unknown, None, None);

        let sphere_plan = fixture_model_render_plan(&sphere).unwrap();
        let unknown_plan = fixture_model_render_plan(&unknown).unwrap();

        assert_eq!(
            sphere_plan.draw_kind,
            FixtureModelDrawKind::BuiltInPrimitive
        );
        assert_eq!(
            sphere_plan.fallback_mesh_kind,
            GeometryModelMeshKind::Sphere
        );
        assert_eq!(unknown_plan.draw_kind, FixtureModelDrawKind::BoundsFallback);
        assert_eq!(unknown_plan.fallback_mesh_kind, GeometryModelMeshKind::Box);
        assert!(fixture_model_render_plan(&missing).is_none());
    }

    #[test]
    fn builds_builtin_primitive_mesh_vertices_and_indices() {
        let box_plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Box,
            None,
            Some(Vec3 {
                x: 2.0,
                y: 4.0,
                z: 6.0,
            }),
        ))
        .unwrap();
        let plane_plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Plane,
            None,
            Some(unit_vec3()),
        ))
        .unwrap();
        let cylinder_plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Cylinder,
            None,
            Some(unit_vec3()),
        ))
        .unwrap();
        let sphere_plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Sphere,
            None,
            Some(unit_vec3()),
        ))
        .unwrap();
        let external_plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Cylinder,
            Some("models/lens.glb"),
            Some(unit_vec3()),
        ))
        .unwrap();

        let box_mesh = build_fixture_model_primitive_mesh(&box_plan).unwrap();
        let plane_mesh = build_fixture_model_primitive_mesh(&plane_plan).unwrap();
        let cylinder_mesh = build_fixture_model_primitive_mesh(&cylinder_plan).unwrap();
        let sphere_mesh = build_fixture_model_primitive_mesh(&sphere_plan).unwrap();
        let batch = build_fixture_model_primitive_meshes(&[
            box_plan,
            plane_plan,
            cylinder_plan,
            sphere_plan,
            external_plan.clone(),
        ]);

        assert_eq!(box_mesh.mesh_kind, GeometryModelMeshKind::Box);
        assert_eq!(box_mesh.vertices.len(), 24);
        assert_eq!(box_mesh.indices.len(), 36);
        let (min, max) = mesh_bounds(&box_mesh);
        assert_close(min.x, -1.0);
        assert_close(max.x, 1.0);
        assert_close(min.y, -2.0);
        assert_close(max.y, 2.0);
        assert_close(min.z, -3.0);
        assert_close(max.z, 3.0);

        assert_eq!(plane_mesh.vertices.len(), 4);
        assert_eq!(plane_mesh.indices.len(), 6);
        assert_eq!(cylinder_mesh.vertices.len(), 70);
        assert_eq!(cylinder_mesh.indices.len(), 192);
        assert_eq!(sphere_mesh.vertices.len(), 153);
        assert_eq!(sphere_mesh.indices.len(), 768);
        assert!(build_fixture_model_primitive_mesh(&external_plan).is_none());
        assert_eq!(batch.len(), 4);
    }

    #[test]
    fn builds_obj_mesh_scaled_to_model_dimensions() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/head.obj"),
            Some(Vec3 {
                x: 2.0,
                y: 4.0,
                z: 1.0,
            }),
        ))
        .unwrap();
        let mesh = build_fixture_model_obj_mesh(
            &plan,
            b"
v -1 -2 0
v 1 -2 0
v 0 2 0
vn 0 0 1
f 1//1 2//1 3//1
",
        )
        .unwrap();

        assert_eq!(mesh.mesh_kind, GeometryModelMeshKind::Mesh);
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        let (min, max) = mesh_bounds(&mesh);
        assert_close(min.x, -1.0);
        assert_close(max.x, 1.0);
        assert_close(min.y, -2.0);
        assert_close(max.y, 2.0);
        assert_close(min.z, 0.0);
        assert_close(max.z, 0.0);
        assert_close(mesh.vertices[0].normal.z, 1.0);
    }

    #[test]
    fn triangulates_obj_polygon_faces_and_negative_indices() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/plate.obj"),
            Some(Vec3 {
                x: 2.0,
                y: 2.0,
                z: 0.0,
            }),
        ))
        .unwrap();
        let mesh = build_fixture_model_obj_mesh(
            &plan,
            b"
v -1 -1 0
v 1 -1 0
v 1 1 0
v -1 1 0
f -4 -3 -2 -1
",
        )
        .unwrap();

        assert_eq!(mesh.vertices.len(), 6);
        assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5]);
        let (min, max) = mesh_bounds(&mesh);
        assert_close(min.x, -1.0);
        assert_close(max.x, 1.0);
        assert_close(min.y, -1.0);
        assert_close(max.y, 1.0);
    }

    #[test]
    fn rejects_obj_faces_with_out_of_range_indices() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/broken.obj"),
            Some(unit_vec3()),
        ))
        .unwrap();
        let error = build_fixture_model_obj_mesh(&plan, b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n")
            .unwrap_err();

        assert_eq!(
            error,
            ObjModelMeshError::IndexOutOfRange {
                line: 4,
                token: "4".to_string(),
            }
        );
    }

    #[test]
    fn builds_3ds_mesh_scaled_to_model_dimensions() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/head.3ds"),
            Some(Vec3 {
                x: 2.0,
                y: 2.0,
                z: 1.0,
            }),
        ))
        .unwrap();
        let mesh = build_fixture_model_3ds_mesh(
            &plan,
            &minimal_3ds_bytes(
                &[
                    Vec3 {
                        x: -1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[[0, 1, 2]],
            ),
        )
        .unwrap();

        assert_eq!(mesh.mesh_kind, GeometryModelMeshKind::Mesh);
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        let (min, max) = mesh_bounds(&mesh);
        assert_close(min.x, -1.0);
        assert_close(max.x, 1.0);
        assert_close(min.y, -1.0);
        assert_close(max.y, 1.0);
        assert_close(min.z, 0.0);
        assert_close(max.z, 0.0);
        assert_close(mesh.vertices[0].normal.z, 1.0);
    }

    #[test]
    fn applies_3ds_mesh_matrix_before_model_dimensions() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/offset-head.3ds"),
            Some(Vec3 {
                x: 2.0,
                y: 2.0,
                z: 1.0,
            }),
        ))
        .unwrap();
        let mesh = build_fixture_model_3ds_mesh(
            &plan,
            &minimal_3ds_bytes_with_transform(
                &[
                    Vec3 {
                        x: -1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[[0, 1, 2]],
                ThreeDsMeshTransform {
                    right: Vec3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    up: Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    },
                    direction: Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                    origin: Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                },
            ),
        )
        .unwrap();

        let (min, max) = mesh_bounds(&mesh);
        assert_close(min.x, -1.0);
        assert_close(max.x, 1.0);
        assert_close(min.y, 0.0);
        assert_close(max.y, 0.0);
        assert_close(min.z, -0.5);
        assert_close(max.z, 0.5);
        assert_close(mesh.vertices[0].normal.y, -1.0);
    }

    #[test]
    fn rejects_3ds_faces_with_out_of_range_indices() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/broken.3ds"),
            Some(unit_vec3()),
        ))
        .unwrap();
        let error = build_fixture_model_3ds_mesh(
            &plan,
            &minimal_3ds_bytes(
                &[
                    Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[[0, 1, 4]],
            ),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ThreeDsModelMeshError::FaceIndexOutOfRange { index: 4, .. }
        ));
    }

    #[test]
    fn builds_glb_mesh_scaled_to_model_dimensions() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/head.glb"),
            Some(Vec3 {
                x: 2.0,
                y: 2.0,
                z: 1.0,
            }),
        ))
        .unwrap();
        let mesh = build_fixture_model_glb_mesh(
            &plan,
            &minimal_glb_bytes(
                &[
                    Vec3 {
                        x: -1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[0, 1, 2],
            ),
        )
        .unwrap();

        assert_eq!(mesh.mesh_kind, GeometryModelMeshKind::Mesh);
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        let (min, max) = mesh_bounds(&mesh);
        assert_close(min.x, -1.0);
        assert_close(max.x, 1.0);
        assert_close(min.y, -1.0);
        assert_close(max.y, 1.0);
        assert_close(min.z, 0.0);
        assert_close(max.z, 0.0);
        assert_close(mesh.vertices[0].normal.z, 1.0);
    }

    #[test]
    fn applies_glb_node_trs_transform_to_mesh_positions() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/head.glb"),
            Some(Vec3 {
                x: 2.0,
                y: 2.0,
                z: 1.0,
            }),
        ))
        .unwrap();
        let mesh = build_fixture_model_glb_mesh(
            &plan,
            &minimal_glb_bytes_with_extra(
                &[
                    Vec3 {
                        x: -1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[0, 1, 2],
                serde_json::json!({
                    "nodes": [
                        {
                            "mesh": 0,
                            "rotation": [0.70710677, 0.0, 0.0, 0.70710677]
                        }
                    ],
                    "scenes": [{ "nodes": [0] }],
                    "scene": 0
                }),
            ),
        )
        .unwrap();

        let (min, max) = mesh_bounds(&mesh);
        assert_close(min.x, -1.0);
        assert_close(max.x, 1.0);
        assert_close(min.y, 0.0);
        assert_close(max.y, 0.0);
        assert_close(min.z, -0.5);
        assert_close(max.z, 0.5);
        assert_close(mesh.vertices[0].normal.y, -1.0);
    }

    #[test]
    fn applies_glb_node_matrix_transform_to_mesh_positions() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/head.glb"),
            Some(Vec3 {
                x: 2.0,
                y: 2.0,
                z: 1.0,
            }),
        ))
        .unwrap();
        let mesh = build_fixture_model_glb_mesh(
            &plan,
            &minimal_glb_bytes_with_extra(
                &[
                    Vec3 {
                        x: -1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[0, 1, 2],
                serde_json::json!({
                    "nodes": [
                        {
                            "mesh": 0,
                            "matrix": [
                                1.0, 0.0, 0.0, 0.0,
                                0.0, 0.0, 1.0, 0.0,
                                0.0, -1.0, 0.0, 0.0,
                                0.0, 0.0, 0.0, 1.0
                            ]
                        }
                    ],
                    "scenes": [{ "nodes": [0] }],
                    "scene": 0
                }),
            ),
        )
        .unwrap();

        let (min, max) = mesh_bounds(&mesh);
        assert_close(min.x, -1.0);
        assert_close(max.x, 1.0);
        assert_close(min.y, 0.0);
        assert_close(max.y, 0.0);
        assert_close(min.z, -0.5);
        assert_close(max.z, 0.5);
        assert_close(mesh.vertices[0].normal.y, -1.0);
    }

    #[test]
    fn rejects_glb_indices_with_out_of_range_positions() {
        let plan = fixture_model_render_plan(&fixture_model_node(
            GeometryModelMeshKind::Mesh,
            Some("models/broken.glb"),
            Some(unit_vec3()),
        ))
        .unwrap();
        let error = build_fixture_model_glb_mesh(
            &plan,
            &minimal_glb_bytes(
                &[
                    Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[0, 1, 4],
            ),
        )
        .unwrap_err();

        assert_eq!(error, GlbModelMeshError::IndexOutOfRange { index: 4 });
    }

    #[test]
    fn builds_visualizer_render_payload_with_scene_plans_and_primitive_meshes() {
        let mut fixture = fixture(1, "Body", Vec3::default(), vec![("Dimmer", 65_535)]);
        fixture.geometries = vec![
            GeometrySummary {
                name: "Base".to_string(),
                kind: "Geometry".to_string(),
                parent: None,
                matrix: [
                    1.0, 0.0, 0.0, 0.0, //
                    0.0, 1.0, 0.0, 0.0, //
                    0.0, 0.0, 1.0, 0.0, //
                    0.0, 0.0, 0.0, 1.0,
                ],
                model_name: Some("BaseModel".to_string()),
                model_file: None,
                model_primitive: Some("Cube".to_string()),
                model_dimensions: Some(unit_vec3()),
                beam_type: None,
                beam_angle_deg: None,
                field_angle_deg: None,
                beam_radius: None,
            },
            GeometrySummary {
                name: "Head".to_string(),
                kind: "Geometry".to_string(),
                parent: None,
                matrix: [
                    1.0, 0.0, 0.0, 0.0, //
                    0.0, 1.0, 0.0, 1.0, //
                    0.0, 0.0, 1.0, 0.0, //
                    0.0, 0.0, 0.0, 1.0,
                ],
                model_name: Some("HeadModel".to_string()),
                model_file: Some("models/head.glb".to_string()),
                model_primitive: Some("Sphere".to_string()),
                model_dimensions: Some(unit_vec3()),
                beam_type: None,
                beam_angle_deg: None,
                field_angle_deg: None,
                beam_radius: None,
            },
        ];

        let payload = build_visualizer_render_payload(
            &snapshot_with_dmx_preview(vec![fixture]),
            VisualizerConfig::default(),
        );

        assert_eq!(payload.scene.fixture_models.len(), 2);
        assert_eq!(payload.model_render_plans.len(), 2);
        assert_eq!(payload.primitive_meshes.len(), 1);
        assert_eq!(
            payload.primitive_meshes[0].geometry_name,
            "Base".to_string()
        );
        assert!(payload
            .model_render_plans
            .iter()
            .any(|plan| plan.draw_kind == FixtureModelDrawKind::ExternalMesh));
    }

    #[test]
    fn normalizes_model_mesh_kind_from_primitive_file_and_dimensions() {
        assert_eq!(
            model_mesh_kind(Some("Cube"), None, None),
            GeometryModelMeshKind::Box
        );
        assert_eq!(
            model_mesh_kind(Some("Yoke"), None, None),
            GeometryModelMeshKind::Box
        );
        assert_eq!(
            model_mesh_kind(Some("Sphere"), None, None),
            GeometryModelMeshKind::Sphere
        );
        assert_eq!(
            model_mesh_kind(Some("Plane"), None, None),
            GeometryModelMeshKind::Plane
        );
        assert_eq!(
            model_mesh_kind(None, Some("models/body.glb"), None),
            GeometryModelMeshKind::Mesh
        );
        assert_eq!(
            model_mesh_kind(
                None,
                None,
                Some(Vec3 {
                    x: 1.0,
                    y: 0.5,
                    z: 1.0,
                }),
            ),
            GeometryModelMeshKind::Box
        );
        assert_eq!(
            model_mesh_kind(Some("VendorCustom"), None, None),
            GeometryModelMeshKind::Unknown
        );
    }

    fn unit_vec3() -> Vec3 {
        Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }

    fn minimal_3ds_bytes(positions: &[Vec3], faces: &[[u16; 3]]) -> Vec<u8> {
        minimal_3ds_bytes_with_optional_transform(positions, faces, None)
    }

    fn minimal_glb_bytes(positions: &[Vec3], indices: &[u16]) -> Vec<u8> {
        minimal_glb_bytes_with_extra(positions, indices, serde_json::json!({}))
    }

    fn minimal_glb_bytes_with_extra(
        positions: &[Vec3],
        indices: &[u16],
        extra: serde_json::Value,
    ) -> Vec<u8> {
        let mut bin = Vec::new();
        for position in positions {
            bin.extend(position.x.to_le_bytes());
            bin.extend(position.y.to_le_bytes());
            bin.extend(position.z.to_le_bytes());
        }
        let index_offset = bin.len();
        for index in indices {
            bin.extend(index.to_le_bytes());
        }
        let index_length = bin.len() - index_offset;
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let mut json = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": [{ "byteLength": bin.len() }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0, "byteLength": positions.len() * 12 },
                { "buffer": 0, "byteOffset": index_offset, "byteLength": index_length }
            ],
            "accessors": [
                {
                    "bufferView": 0,
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": positions.len(),
                    "type": "VEC3"
                },
                {
                    "bufferView": 1,
                    "byteOffset": 0,
                    "componentType": 5123,
                    "count": indices.len(),
                    "type": "SCALAR"
                }
            ],
            "meshes": [
                {
                    "primitives": [
                        {
                            "attributes": { "POSITION": 0 },
                            "indices": 1
                        }
                    ]
                }
            ]
        });
        let Some(json_object) = json.as_object_mut() else {
            unreachable!();
        };
        if let Some(extra_object) = extra.as_object() {
            for (key, value) in extra_object {
                json_object.insert(key.clone(), value.clone());
            }
        }
        let json = json.to_string();
        let json_chunk = glb_chunk(GLB_JSON_CHUNK, json.into_bytes(), 0x20);
        let bin_chunk = glb_chunk(GLB_BIN_CHUNK, bin, 0);
        let declared_length = 12 + json_chunk.len() + bin_chunk.len();
        let mut glb = Vec::with_capacity(declared_length);
        glb.extend(GLB_MAGIC.to_le_bytes());
        glb.extend(GLB_VERSION_2.to_le_bytes());
        glb.extend((declared_length as u32).to_le_bytes());
        glb.extend(json_chunk);
        glb.extend(bin_chunk);
        glb
    }

    fn glb_chunk(chunk_type: u32, mut data: Vec<u8>, padding: u8) -> Vec<u8> {
        while data.len() % 4 != 0 {
            data.push(padding);
        }
        let mut chunk = Vec::with_capacity(data.len() + 8);
        chunk.extend((data.len() as u32).to_le_bytes());
        chunk.extend(chunk_type.to_le_bytes());
        chunk.extend(data);
        chunk
    }

    fn minimal_3ds_bytes_with_transform(
        positions: &[Vec3],
        faces: &[[u16; 3]],
        transform: ThreeDsMeshTransform,
    ) -> Vec<u8> {
        minimal_3ds_bytes_with_optional_transform(positions, faces, Some(transform))
    }

    fn minimal_3ds_bytes_with_optional_transform(
        positions: &[Vec3],
        faces: &[[u16; 3]],
        transform: Option<ThreeDsMeshTransform>,
    ) -> Vec<u8> {
        let mut vertex_payload = Vec::new();
        vertex_payload.extend((positions.len() as u16).to_le_bytes());
        for position in positions {
            vertex_payload.extend(position.x.to_le_bytes());
            vertex_payload.extend(position.y.to_le_bytes());
            vertex_payload.extend(position.z.to_le_bytes());
        }

        let mut face_payload = Vec::new();
        face_payload.extend((faces.len() as u16).to_le_bytes());
        for [a, b, c] in faces {
            face_payload.extend(a.to_le_bytes());
            face_payload.extend(b.to_le_bytes());
            face_payload.extend(c.to_le_bytes());
            face_payload.extend(0u16.to_le_bytes());
        }

        let mut mesh_payload = Vec::new();
        mesh_payload.extend(three_ds_chunk(THREE_DS_VERTEX_LIST_CHUNK, vertex_payload));
        mesh_payload.extend(three_ds_chunk(THREE_DS_FACE_LIST_CHUNK, face_payload));
        if let Some(transform) = transform {
            let mut transform_payload = Vec::new();
            for value in [
                transform.right.x,
                transform.right.y,
                transform.right.z,
                transform.up.x,
                transform.up.y,
                transform.up.z,
                transform.direction.x,
                transform.direction.y,
                transform.direction.z,
                transform.origin.x,
                transform.origin.y,
                transform.origin.z,
            ] {
                transform_payload.extend(value.to_le_bytes());
            }
            mesh_payload.extend(three_ds_chunk(
                THREE_DS_MESH_MATRIX_CHUNK,
                transform_payload,
            ));
        }

        let mut object_payload = b"Mesh\0".to_vec();
        object_payload.extend(three_ds_chunk(THREE_DS_TRIANGULAR_MESH_CHUNK, mesh_payload));
        let editor_payload = three_ds_chunk(THREE_DS_OBJECT_BLOCK_CHUNK, object_payload);
        three_ds_chunk(
            THREE_DS_MAIN_CHUNK,
            three_ds_chunk(THREE_DS_EDITOR_CHUNK, editor_payload),
        )
    }

    fn three_ds_chunk(id: u16, payload: Vec<u8>) -> Vec<u8> {
        let mut chunk = Vec::with_capacity(payload.len() + 6);
        chunk.extend(id.to_le_bytes());
        chunk.extend(((payload.len() + 6) as u32).to_le_bytes());
        chunk.extend(payload);
        chunk
    }

    fn fixture_model_node(
        mesh_kind: GeometryModelMeshKind,
        model_file: Option<&str>,
        dimensions: Option<Vec3>,
    ) -> FixtureModelNode {
        FixtureModelNode {
            fixture_id: 1,
            profile_source_path: "memory://fixture-model-node".to_string(),
            geometry_name: "Model".to_string(),
            mesh_kind,
            model_name: None,
            model_file: model_file.map(str::to_string),
            model_primitive: None,
            dimensions,
            bounding_radius: model_bounding_radius(dimensions),
            position: Vec3::default(),
            right: Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            up: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            direction: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        }
    }

    fn mesh_bounds(mesh: &FixtureModelPrimitiveMesh) -> (Vec3, Vec3) {
        let first = mesh.vertices[0].position;
        mesh.vertices.iter().map(|vertex| vertex.position).fold(
            (first, first),
            |(min, max), position| {
                (
                    Vec3 {
                        x: min.x.min(position.x),
                        y: min.y.min(position.y),
                        z: min.z.min(position.z),
                    },
                    Vec3 {
                        x: max.x.max(position.x),
                        y: max.y.max(position.y),
                        z: max.z.max(position.z),
                    },
                )
            },
        )
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {actual} to be close to {expected}"
        );
    }

    fn snapshot_with_dmx_preview(fixtures: Vec<PatchedFixtureSummary>) -> EngineSnapshot {
        let mut frame = vec![0; 512];
        for fixture in &fixtures {
            for control in &fixture.controls {
                let Some(value) = read_attribute(fixture, &[control.attribute.as_str()]) else {
                    continue;
                };
                let Some(coarse_index) = control
                    .offsets
                    .first()
                    .and_then(|offset| dmx_index(fixture.address, *offset))
                else {
                    continue;
                };
                frame[coarse_index] = (value >> 8) as u8;
                if control.resolution == AttributeResolution::SixteenBit {
                    if let Some(fine_index) = control
                        .offsets
                        .get(1)
                        .and_then(|offset| dmx_index(fixture.address, *offset))
                    {
                        frame[fine_index] = (value & 0xff) as u8;
                    }
                }
            }
        }
        EngineSnapshot {
            fixtures,
            dmx_preview: frame.clone(),
            dmx_previews: vec![DmxUniversePreview {
                universe: 0,
                values: frame,
            }],
            ..EngineSnapshot::default()
        }
    }

    fn geometry(
        name: &str,
        kind: &str,
        parent: Option<&str>,
        x: f32,
        y: f32,
        z: f32,
    ) -> GeometrySummary {
        GeometrySummary {
            name: name.to_string(),
            kind: kind.to_string(),
            parent: parent.map(str::to_string),
            matrix: [
                1.0, 0.0, 0.0, x, //
                0.0, 1.0, 0.0, y, //
                0.0, 0.0, 1.0, z, //
                0.0, 0.0, 0.0, 1.0,
            ],
            model_name: None,
            model_file: None,
            model_primitive: None,
            model_dimensions: None,
            beam_type: None,
            beam_angle_deg: None,
            field_angle_deg: None,
            beam_radius: None,
        }
    }

    fn fixture(
        id: FixtureId,
        label: &str,
        position: Vec3,
        values: Vec<(&str, u16)>,
    ) -> PatchedFixtureSummary {
        let controls = values
            .iter()
            .enumerate()
            .map(|(index, (attribute, _))| AttributeControl {
                attribute: (*attribute).to_string(),
                channel_name: (*attribute).to_string(),
                geometry: None,
                offsets: vec![index as u16 + 1],
                resolution: AttributeResolution::EightBit,
                default_value: 0,
                functions: Vec::new(),
            })
            .collect::<Vec<_>>();
        let attribute_values = values
            .into_iter()
            .map(|(attribute, value)| AttributeValueSummary {
                attribute: attribute.to_string(),
                value,
            })
            .collect::<Vec<_>>();

        PatchedFixtureSummary {
            id,
            label: label.to_string(),
            profile_source_path: "memory://visualizer.fixture".to_string(),
            profile_name: "Fixture".to_string(),
            manufacturer: "Syndocal".to_string(),
            mode_name: "Default".to_string(),
            universe: 0,
            address: 1,
            group_ids: Vec::new(),
            position,
            rotation: Rotation3::default(),
            geometries: Vec::new(),
            controls,
            attribute_values,
            limits: Default::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        }
    }
}
