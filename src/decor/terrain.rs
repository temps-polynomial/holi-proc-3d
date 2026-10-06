//! Sol : prairie, collines ondulées, rochers et buissons fleuris (amas d'ellipsoïdes).

use std::f32::consts::{PI, TAU};

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use super::AleaDecor;
use super::nature::{MateriauNature, nature};
use crate::bascules::{Bascule, piece};
use crate::maillage;

/// (centre, rayons) de chaque ellipsoïde de colline ; lu par les arbres pour se poser dessus.
// Resource : donnée globale unique, insérée par creer_prairie_et_collines.
// Book : https://bevy.org/learn/book/storing-data/resources/#dynamic-resources
// Doc : https://docs.rs/bevy/latest/bevy/ecs/resource/trait.Resource.html
// Vec3 : vecteur 3D (glam) ; opérateurs et méthodes (normalize, to_array...) sur la même page.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
#[derive(Resource)]
pub struct Collines(Vec<(Vec3, Vec3)>);

impl Collines {
    /// Hauteur du sol en (x, z) : la prairie, ou le sommet d'un ellipsoïde de colline.
    pub fn hauteur_sol(&self, x: f32, z: f32) -> f32 {
        self.0
            .iter()
            .map(|(c, r)| {
                let (dx, dz) = ((x - c.x) / r.x, (z - c.z) / r.z);
                let dedans = 1.0 - dx * dx - dz * dz;
                if dedans > 0.0 { c.y + r.y * dedans.sqrt() } else { 0.0 }
            })
            .fold(0.0, f32::max)
    }
}

/// Système Startup : la prairie, puis 14 collines en couronne ; insère la ressource Collines.
// Commands : modifications différées du monde. ResMut<T> : écriture dans une ressource
// (ici les collections d'assets et le générateur partagé). add : stocke l'asset, renvoie son Handle.
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.add
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
pub fn creer_prairie_et_collines(
    mut commands: Commands,
    mut maillages: ResMut<Assets<Mesh>>,
    mut mats_nature: ResMut<Assets<MateriauNature>>,
    mut alea: ResMut<AleaDecor>,
) {
    let alea = &mut alea.0;
    let prairie = mats_nature.add(nature(0.0, [0.10, 0.30, 0.03], [0.42, 0.66, 0.10], 3.0, 0.4, 1.0));
    let colline = mats_nature.add(nature(0.0, [0.14, 0.36, 0.05], [0.45, 0.66, 0.14], 3.0, 0.4, 0.0));

    // spawn_scene(bsn! {...}) : crée une entité ; #Nom ajoute un composant Name.
    // Mesh3d : maillage affiché ; MeshMaterial3d::<M> : son matériau (type M explicite dans bsn!).
    // asset_value(x) : ajoute x comme asset et passe son Handle (pas besoin de ResMut<Assets>).
    // Cuboid : primitive convertie en Mesh. Transform::from_xyz : position seule.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L21-L26
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
    // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
    // Doc : https://docs.rs/bevy/latest/bevy/ecs/name/struct.Name.html
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh3d.html
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MeshMaterial3d.html
    // Doc : https://docs.rs/bevy/latest/bevy/asset/fn.asset_value.html
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Cuboid.html
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.from_xyz
    commands.spawn_scene(bsn! {
        #Prairie Mesh3d(asset_value(Cuboid::new(900.0, 0.2, 900.0))) MeshMaterial3d::<MateriauNature>(prairie)
        Transform::from_xyz(0.0, -0.1, 0.0)
    });
    // Sphere::mesh().ico(n) : icosphère subdivisée n fois (erreur si n trop grand, d'où unwrap).
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ecs/iter_combinations.rs#L44
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Sphere.html
    // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.SphereMeshBuilder.html#method.ico
    let maillage_colline = maillages.add(Sphere::new(1.0).mesh().ico(4).unwrap());
    let mut collines = Vec::new();
    for i in 0..14 {
        let a = i as f32 / 14.0 * TAU + alea.entre(-0.2, 0.2);
        let r = alea.entre(30.0, 60.0);
        let d = alea.entre(70.0, 120.0);
        let (centre, rayons) = (Vec3::new(a.cos() * d, -r * 0.78, a.sin() * d), Vec3::new(r * 1.6, r, r * 1.3));
        collines.push((centre, rayons));
        // piece(...) : maillage + matériau + calque (voir bascules.rs).
        // Transform { ... } dans bsn! : seuls les champs cités changent, les autres gardent
        // leur valeur par défaut (rotation nulle ici). NotShadowCaster : pas d'ombre projetée.
        // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html
        // Doc : https://docs.rs/bevy/latest/bevy/light/struct.NotShadowCaster.html
        commands.spawn_scene(bsn! {
            piece(maillage_colline.clone(), colline.clone(), Bascule::Collines) Transform { translation: centre, scale: rayons } NotShadowCaster
        });
    }
    // Insertion différée de la ressource ; visible par les systèmes suivants de la chaîne.
    // Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html#method.insert_resource
    commands.insert_resource(Collines(collines));
}

/// Système Startup : rochers et buissons fleuris (chaque ensemble fusionné en un seul maillage).
pub fn creer_rochers_et_buissons(
    mut commands: Commands,
    mut maillages: ResMut<Assets<Mesh>>,
    mut mats_nature: ResMut<Assets<MateriauNature>>,
    mut alea: ResMut<AleaDecor>,
) {
    let alea = &mut alea.0;
    let buisson = mats_nature.add(nature(3.0, [0.08, 0.26, 0.04], [0.40, 0.64, 0.12], 5.0, 0.6, 1.0));
    let roche = mats_nature.add(nature(2.0, [0.20, 0.20, 0.20], [0.52, 0.50, 0.47], 2.5, 0.5, 1.0));
    let rochers: Vec<_> = (0..14)
        .map(|_| {
            let (a, r, s) = (alea.entre(0.0, TAU), alea.entre(4.5, 11.5), alea.entre(0.25, 0.7));
            (Vec3::new(a.cos() * r, 0.1 * s, a.sin() * r), Vec3::new(s * 1.3, s * 0.6, s), alea.entre(0.0, TAU))
        })
        .collect();
    commands.spawn_scene(bsn! { piece(maillages.add(maillage_ellipsoides(&rochers)), roche, Bascule::Rochers) });
    let mut buissons = Vec::new();
    for _ in 0..14 {
        let (a, r) = (alea.entre(0.0, TAU), alea.entre(5.0, 12.0));
        let (cx, cz) = (a.cos() * r, a.sin() * r);
        for _ in 0..alea.entier(3, 6) {
            let s = alea.entre(0.3, 0.55);
            buissons.push((
                Vec3::new(cx + alea.entre(-0.5, 0.5), s * 0.6, cz + alea.entre(-0.5, 0.5)),
                Vec3::new(s, s * 0.85, s),
                0.0,
            ));
        }
    }
    commands.spawn_scene(bsn! { piece(maillages.add(maillage_ellipsoides(&buissons)), buisson, Bascule::Rochers) });
}

/// Plusieurs ellipsoïdes (centre, rayons, lacet) fusionnés en un seul maillage.
fn maillage_ellipsoides(ellipsoides: &[(Vec3, Vec3, f32)]) -> Mesh {
    let (lat, lon) = (10usize, 16usize);
    let mut unitaire = Vec::new();
    for i in 0..=lat {
        let t = PI * i as f32 / lat as f32;
        for j in 0..=lon {
            let p = TAU * j as f32 / lon as f32;
            unitaire.push(Vec3::new(t.sin() * p.cos(), t.cos(), t.sin() * p.sin()));
        }
    }
    let mut tri = Vec::new();
    for i in 0..lat {
        for j in 0..lon {
            let a = (i * (lon + 1) + j) as u32;
            let b = ((i + 1) * (lon + 1) + j) as u32;
            tri.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    let (mut pos, mut nrm, mut idx) = (Vec::new(), Vec::new(), Vec::new());
    for (k, &(c, r, lacet)) in ellipsoides.iter().enumerate() {
        let (cy, sy) = (lacet.cos(), lacet.sin());
        let rot = |v: Vec3| Vec3::new(cy * v.x + sy * v.z, v.y, -sy * v.x + cy * v.z);
        for &u in &unitaire {
            pos.push((rot(u * r) + c).to_array());
            nrm.push(rot(u / r).normalize().to_array());
        }
        let decalage = (k * unitaire.len()) as u32;
        idx.extend(tri.iter().map(|i| i + decalage));
    }
    maillage::assembler(pos, nrm, None, None, idx)
}
