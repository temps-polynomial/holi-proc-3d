// Balancement de l'herbe. uv.y = hauteur le long du brin (0 racine, 1 pointe) : les racines
// restent fixes, les pointes suivent la brise avec des rafales qui roulent. Partagé par la
// passe principale et le prépass, pour que la profondeur de champ voie les mêmes brins.

// mesh_functions : matrices et conversions du maillage (local -> monde).
// position_world_to_clip : monde -> espace de découpage (écran).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/custom_vertex_attribute.wgsl#L3-L30
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/mesh_functions.wgsl#L17-L19
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/mesh_functions.wgsl#L54-L56
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/mesh_functions.wgsl#L68
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/view_transformations.wgsl#L121-L125
#import bevy_pbr::{mesh_functions, view_transformations::position_world_to_clip}
// PREPASS_PIPELINE : défini quand ce shader sert au prépass ; Vertex/VertexOutput y diffèrent.
// Le code ci-dessous reprend les vertex shaders par défaut de Bevy, plus le balancement.
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/prepass/prepass.wgsl#L67-L191
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/mesh.wgsl#L36-L110
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/prepass/prepass_io.wgsl#L5-L79
// Source : https://github.com/bevyengine/bevy/blob/release-0.19.1/crates/bevy_pbr/src/render/forward_io.wgsl#L3-L56
#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::{Vertex, VertexOutput}
#else
#import bevy_pbr::forward_io::{Vertex, VertexOutput}
#endif

// Même ordre et mêmes types que `ParamsHerbe` (materiaux.rs).
struct Herbe {
    maintenant: f32,
    vent: vec4<f32>,
}

// binding 100 = #[uniform(100)] de l'extension (0-99 : StandardMaterial).
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/assets/shaders/extended_material.wgsl#L28-L29
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> herbe: Herbe;

// Décalage d'un point du brin (p en monde, h = hauteur relative).
fn balancement(p: vec3<f32>, h: f32) -> vec3<f32> {
    let t = herbe.maintenant;
    let v = herbe.vent;
    // grands fronts de rafale qui traversent la prairie + petit frémissement
    let front = dot(p.xz, normalize(v.xz + vec2(1e-4))) * 0.35 - t * 0.8;
    let rafale = 0.55 + 0.45 * sin(front) * sin(front * 0.37 + 1.7);
    let fremissement = sin(t * 2.3 + p.x * 3.7 + p.z * 2.9) * 0.12;
    let flexion = h * h * v.w * (rafale + fremissement);
    let cote = vec3(v.x, 0.0, v.z) * flexion;
    return vec3(cote.x, -abs(flexion) * 0.25, cote.z);
}

// @vertex fn vertex : point d'entrée attendu par Bevy pour le vertex shader du matériau.
@vertex
fn vertex(entree: Vertex) -> VertexOutput {
    var sortie: VertexOutput;
    // instance_index : désigne la transformée de cette entité dans les données de maillage.
    let monde_depuis_local = mesh_functions::get_world_from_local(entree.instance_index);
    var pos_monde = mesh_functions::mesh_position_local_to_world(monde_depuis_local, vec4<f32>(entree.position, 1.0));
// VERTEX_UVS_A, VERTEX_NORMALS... : définis selon les attributs présents dans le maillage.
#ifdef VERTEX_UVS_A
    pos_monde = vec4(pos_monde.xyz + balancement(pos_monde.xyz, entree.uv.y), 1.0);
#endif
    sortie.world_position = pos_monde;
    sortie.position = position_world_to_clip(pos_monde.xyz);

#ifdef PREPASS_PIPELINE
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    sortie.unclipped_depth = sortie.position.z;
    sortie.position.z = min(sortie.position.z, 1.0);
#endif
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
    sortie.world_normal = mesh_functions::mesh_normal_local_to_world(entree.normal, entree.instance_index);
#endif
#endif
#ifdef MOTION_VECTOR_PREPASS
    sortie.previous_world_position = sortie.world_position;
#endif
#ifdef VERTEX_UVS_A
    sortie.uv = entree.uv;
#endif
#else
#ifdef VERTEX_NORMALS
    sortie.world_normal = mesh_functions::mesh_normal_local_to_world(entree.normal, entree.instance_index);
#endif
#ifdef VERTEX_UVS_A
    sortie.uv = entree.uv;
#endif
#endif

#ifdef VERTEX_UVS_B
    sortie.uv_b = entree.uv_b;
#endif
#ifdef VERTEX_COLORS
    sortie.color = entree.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    sortie.instance_index = entree.instance_index;
#endif
    return sortie;
}
