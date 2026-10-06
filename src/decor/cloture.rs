//! Clôture en bois autour de la prairie, et fanions : guirlandes qui pendent entre de grands mâts.

use std::f32::consts::{PI, TAU};

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::PALETTE;
use crate::bascules::{Bascule, piece};

/// Système Startup : anneau de clôture (poteaux + deux lisses), puis mâts, fils et fanions.
// Commands : modifications différées du monde. ResMut<Assets<T>> : écriture dans la collection
// d'assets de type T.
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
pub fn creer_cloture_et_fanions(mut commands: Commands, mut maillages: ResMut<Assets<Mesh>>, mut standards: ResMut<Assets<StandardMaterial>>) {
    // anneau de clôture en bois
    // add : stocke l'asset et renvoie son Handle. StandardMaterial { ..default() } : matériau PBR,
    // champs non cités par défaut. Color::srgb : couleur en sRGB.
    // Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/camera/camera_orbit.rs#L57-L66
    // Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.add
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
    // Doc : https://docs.rs/bevy/latest/bevy/utils/fn.default.html
    // Doc : https://docs.rs/bevy/latest/bevy/color/enum.Color.html#method.srgb
    let bois = standards.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.28, 0.17),
        perceptual_roughness: 0.85,
        ..default()
    });
    let poteaux = 48;
    let r_cloture = 14.0;
    let segment = 2.0 * r_cloture * (PI / poteaux as f32).sin();
    // Cuboid : primitive convertie en Mesh par `add`.
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Cuboid.html
    let poteau = maillages.add(Cuboid::new(0.12, 1.0, 0.12));
    let lisse = maillages.add(Cuboid::new(segment + 0.1, 0.07, 0.05));
    for i in 0..poteaux {
        let a = i as f32 / poteaux as f32 * TAU;
        // spawn_scene(bsn! {...}) : crée une entité. piece(...) : maillage + matériau + calque
        // (voir bascules.rs). Transform::from_xyz : position seule.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L21-L26
        // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
        // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
        // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.from_xyz
        commands.spawn_scene(bsn! { piece(poteau.clone(), bois.clone(), Bascule::Decor) Transform::from_xyz(a.cos() * r_cloture, 0.5, a.sin() * r_cloture) });
        let am = (i as f32 + 0.5) / poteaux as f32 * TAU;
        // Vec3 : vecteur 3D (glam) ; opérateurs et méthodes (normalize, length...) sur la même page.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
        // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
        let milieu = Vec3::new(am.cos(), 0.0, am.sin()) * r_cloture * (PI / poteaux as f32).cos();
        for y in [0.45, 0.8] {
            // Transform { ... } : seuls les champs cités changent (échelle par défaut ici).
            // Quat::from_rotation_y : rotation autour de l'axe vertical.
            // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html
            // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Quat.html#method.from_rotation_y
            commands.spawn_scene(bsn! {
                piece(lisse.clone(), bois.clone(), Bascule::Decor)
                Transform { translation: Vec3::new(milieu.x, y, milieu.z), rotation: Quat::from_rotation_y(-am + PI / 2.0) }
            });
        }
    }

    // fanions : guirlandes qui pendent entre de grands mâts (hors de l'orbite de la caméra)
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Cylinder.html
    let mat_mat = maillages.add(Cylinder::new(0.05, 3.4));
    let fil = maillages.add(Cylinder::new(0.008, 1.0));
    // Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Triangle3d.html
    let fanion = maillages.add(Triangle3d::new(Vec3::new(-0.13, 0.0, 0.0), Vec3::new(0.13, 0.0, 0.0), Vec3::new(0.0, -0.3, 0.0)));
    let blanc = standards.add(StandardMaterial { base_color: Color::srgb(0.95, 0.95, 0.95), ..default() });
    // double_sided + cull_mode: None : triangle visible et éclairé des deux côtés.
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html#structfield.double_sided
    let mats_fanion: Vec<_> = PALETTE
        .iter()
        .map(|c| {
            standards.add(StandardMaterial {
                base_color: Color::srgb(c[0], c[1], c[2]),
                perceptual_roughness: 0.6,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    let (mats, rm) = (10, 12.8);
    let sommets: Vec<Vec3> = (0..mats)
        .map(|i| {
            let a = i as f32 / mats as f32 * TAU + 0.1;
            let p = Vec3::new(a.cos() * rm, 0.0, a.sin() * rm);
            commands.spawn_scene(bsn! { piece(mat_mat.clone(), bois.clone(), Bascule::Decor) Transform::from_xyz(p.x, 1.7, p.z) });
            Vec3::new(p.x, 3.35, p.z)
        })
        .collect();
    let mut k = 0;
    for i in 0..mats {
        let (p0, p1) = (sommets[i], sommets[(i + 1) % mats]);
        let creux = |t: f32| p0 + (p1 - p0) * t - Vec3::new(0.0, 0.7 * 4.0 * t * (1.0 - t), 0.0);
        let lacet = (-(p1.z - p0.z)).atan2(p1.x - p0.x);
        let mut prec = p0;
        for j in 1..=10 {
            let cour = creux(j as f32 / 10.0);
            let d = cour - prec;
            // Cylindre unitaire étiré (scale.y) et orienté le long du segment.
            // {expr} dans bsn! : valeur calculée par une expression Rust.
            // Quat::from_rotation_arc(a, b) : rotation qui amène la direction a sur b.
            // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/math/render_primitives.rs#L655-L658
            // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Quat.html#method.from_rotation_arc
            commands.spawn_scene(bsn! {
                piece(fil.clone(), blanc.clone(), Bascule::Decor) Transform {
                    translation: {(cour + prec) * 0.5},
                    rotation: Quat::from_rotation_arc(Vec3::Y, d.normalize()),
                    scale: Vec3::new(1.0, d.length(), 1.0),
                }
            });
            prec = cour;
        }
        let nfanions = ((p1 - p0).length() / 0.42) as usize;
        for j in 1..nfanions {
            // NotShadowCaster : l'entité ne projette pas d'ombre.
            // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/shadow_caster_receiver.rs#L49-L54
            // Doc : https://docs.rs/bevy/latest/bevy/light/struct.NotShadowCaster.html
            commands.spawn_scene(bsn! {
                piece(fanion.clone(), mats_fanion[k % mats_fanion.len()].clone(), Bascule::Decor) NotShadowCaster
                Transform { translation: creux(j as f32 / nfanions as f32), rotation: Quat::from_rotation_y(lacet) }
            });
            k += 1;
        }
    }

}
