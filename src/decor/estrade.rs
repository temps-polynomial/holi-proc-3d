//! Estrade centrale et canons à poudre : un canon par couleur de la palette, placé et orienté
//! comme le panache correspondant du volume de poudre.

use bevy::prelude::*;

use crate::PALETTE;
use crate::bascules::{Bascule, piece};
use crate::poudre::{disposition_canon, vers_monde};

/// Système Startup : l'estrade, puis un tambour et un canon peint par couleur.
// Commands : modifications différées du monde. ResMut<Assets<T>> : écriture dans la collection
// d'assets de type T ; add : stocke l'asset (ici une primitive Cylinder convertie en Mesh, ou
// un StandardMaterial) et renvoie son Handle.
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.add
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
// Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Cylinder.html
pub fn creer_estrade(mut commands: Commands, mut maillages: ResMut<Assets<Mesh>>, mut standards: ResMut<Assets<StandardMaterial>>) {
    // spawn_scene(bsn! {...}) : crée une entité. piece(...) : maillage + matériau + calque
    // (voir bascules.rs). StandardMaterial { ..default() } : matériau PBR, champs non cités par défaut.
    // Color::srgb : couleur en sRGB. Transform::from_xyz : position seule.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/camera/camera_orbit.rs#L57-L66
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
    // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
    // Doc : https://docs.rs/bevy/latest/bevy/utils/fn.default.html
    // Doc : https://docs.rs/bevy/latest/bevy/color/enum.Color.html#method.srgb
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.from_xyz
    commands.spawn_scene(bsn! {
        piece(maillages.add(Cylinder::new(2.2, 0.16)), standards.add(StandardMaterial {
            base_color: Color::srgb(0.93, 0.86, 0.74),
            perceptual_roughness: 0.75,
            ..default()
        }), Bascule::Estrade)
        Transform::from_xyz(0.0, 0.08, 0.0)
    });
    let canon = maillages.add(Cylinder::new(0.17, 0.75));
    let tambour = maillages.add(Cylinder::new(0.32, 0.36));
    let mat_tambour = standards.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.94, 0.9),
        perceptual_roughness: 0.5,
        ..default()
    });
    for (i, coul) in PALETTE.iter().enumerate() {
        let (pos, visee) = disposition_canon(i);
        let m = vers_monde(pos.x, 0.0, pos.z);
        let peinture = standards.add(StandardMaterial {
            base_color: Color::srgb(coul[0], coul[1], coul[2]),
            perceptual_roughness: 0.3,
            ..default()
        });
        let d = visee.normalize();
        // spawn_scene_list(bsn_list![a, b]) : une entité par élément (séparés par des virgules).
        // {expr} dans bsn! : valeur calculée par une expression Rust.
        // Transform { ... } : seuls les champs cités changent (échelle par défaut ici).
        // Quat::from_rotation_arc(a, b) : rotation qui amène la direction a sur b.
        // Vec3 : vecteur 3D (glam) ; opérateurs et méthodes (normalize...) sur la même page.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L13-L37
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/math/render_primitives.rs#L655-L658
        // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene_list
        // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn_list.html
        // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html
        // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Quat.html#method.from_rotation_arc
        // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
        commands.spawn_scene_list(bsn_list![
            piece(tambour.clone(), mat_tambour.clone(), Bascule::Estrade) Transform::from_xyz(m.x, 0.18, m.z),
            piece(canon.clone(), peinture, Bascule::Estrade)
                Transform { translation: {Vec3::new(m.x, 0.36, m.z) + d * 0.3}, rotation: Quat::from_rotation_arc(Vec3::Y, d) },
        ]);
    }
}
