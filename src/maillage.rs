//! Assemblage d'un `Mesh` de triangles à partir de tableaux de sommets (herbe, rochers, arbres).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

/// Un tableau par attribut de sommet (tous de même longueur), plus les indices des triangles.
pub fn assembler(
    positions: Vec<[f32; 3]>,
    normales: Vec<[f32; 3]>,
    uvs: Option<Vec<[f32; 2]>>,
    couleurs: Option<Vec<[f32; 4]>>,
    indices: Vec<u32>,
) -> Mesh {
    // Maillage vide ; TriangleList = chaque triplet d'indices forme un triangle.
    // RENDER_WORLD seul : les sommets sont libérés côté CPU après envoi au GPU
    // (recommandé par la doc quand le CPU ne relit pas le maillage).
    // Book : https://bevy.org/learn/book/the-renderer/render-pipelines/#extract
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/generate_custom_mesh.rs#L106-L108
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html#method.new
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/enum.PrimitiveTopology.html
    // Doc : https://docs.rs/bevy/latest/bevy/asset/struct.RenderAssetUsages.html
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    // Attributs de sommet : position, normale, uv (facultatif), couleur (facultative).
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/generate_custom_mesh.rs#L109-L207
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html#method.insert_attribute
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html#associatedconstant.ATTRIBUTE_POSITION
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normales);
    if let Some(uv) = uvs {
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    }
    if let Some(c) = couleurs {
        // Couleur par sommet, multipliée par la couleur de base du StandardMaterial.
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
    }
    // Indices des triangles, en u32 (plus de 65 535 sommets possibles).
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/generate_custom_mesh.rs#L243
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html#method.insert_indices
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/enum.Indices.html
    m.insert_indices(Indices::U32(indices));
    m
}
