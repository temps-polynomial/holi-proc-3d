//! Ballons au bout de leur fil, qui montent, descendent et se balancent.

use std::f32::consts::TAU;

use bevy::prelude::*;

use super::AleaDecor;
use crate::PALETTE;
use crate::bascules::{Bascule, piece};

/// Paramètres de l'animation d'un ballon.
// Component : donnée attachée à une entité. Default + Clone : requis par `bsn!`.
// Book : https://bevy.org/learn/book/storing-data/entities-components/#defining-components
// Doc : https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html
#[derive(Component, Default, Clone)]
pub struct Ballon {
    phase: f32,
    y_base: f32,
}

/// Système Startup : 14 ballons, chacun avec son fil en entité enfant.
// Commands : modifications différées du monde. ResMut<T> : écriture dans une ressource
// (collections d'assets, générateur partagé). add : stocke l'asset et renvoie son Handle.
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.add
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
// Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Sphere.html
// Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Cylinder.html
pub fn creer_ballons(
    mut commands: Commands,
    mut maillages: ResMut<Assets<Mesh>>,
    mut standards: ResMut<Assets<StandardMaterial>>,
    mut alea: ResMut<AleaDecor>,
) {
    let ballon = maillages.add(Sphere::new(0.26));
    let fil = maillages.add(Cylinder::new(0.008, 1.0));
    // StandardMaterial { ..default() } : matériau PBR, champs non cités par défaut.
    // Color::srgb : couleur en sRGB.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/camera/camera_orbit.rs#L57-L66
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
    // Doc : https://docs.rs/bevy/latest/bevy/utils/fn.default.html
    // Doc : https://docs.rs/bevy/latest/bevy/color/enum.Color.html#method.srgb
    let blanc = standards.add(StandardMaterial { base_color: Color::srgb(0.95, 0.95, 0.95), ..default() });
    for i in 0..14 {
        // alea.entre(...) : ResMut<AleaDecor> -> AleaDecor -> Alea, par Deref/DerefMut (voir decor/mod.rs).
        // Doc : https://docs.rs/bevy/latest/bevy/prelude/derive.DerefMut.html
        // Rust : https://doc.rust-lang.org/book/ch15-02-deref.html#using-deref-coercion-in-functions-and-methods
        let a = i as f32 / 14.0 * TAU + alea.entre(-0.15, 0.15);
        let r = alea.entre(4.2, 9.0);
        let y = alea.entre(1.4, 2.8);
        let c = PALETTE[(i * 3) % PALETTE.len()];
        // clearcoat : couche vernie brillante par-dessus le matériau.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/clearcoat.rs#L104-L106
        // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html#structfield.clearcoat
        let mat = standards.add(StandardMaterial {
            base_color: Color::srgb(c[0], c[1], c[2]),
            perceptual_roughness: 0.2,
            clearcoat: 0.9,
            ..default()
        });
        let longueur = y / 1.15;
        // spawn_scene(bsn! {...}) : crée une entité. piece(...) : maillage + matériau + calque
        // (voir bascules.rs). {expr} : valeur calculée par une expression Rust.
        // Transform { ... } : seuls les champs cités changent (rotation par défaut ici).
        // Children [ ... ] : entités enfants ; leur Transform est relatif au parent.
        // Mesh3d / MeshMaterial3d::<M> : maillage et matériau (type M explicite dans bsn!).
        // Book : https://bevy.org/learn/book/storing-data/relations/#adding-children-declaratively
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/scene/bsn.rs#L24-L34
        // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
        // Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
        // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/hierarchy/struct.Children.html
        // Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh3d.html
        // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MeshMaterial3d.html
        // Vec3 : vecteur 3D (glam) ; opérateurs et méthodes sur la même page.
        // Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
        commands.spawn_scene(bsn! {
            piece(ballon.clone(), mat, Bascule::Decor) Ballon { phase: {alea.entre(0.0, TAU)}, y_base: y }
            Transform { translation: Vec3::new(a.cos() * r, y, a.sin() * r), scale: Vec3::new(1.0, 1.15, 1.0) }
            Children [
                Mesh3d({fil.clone()}) MeshMaterial3d::<StandardMaterial>({blanc.clone()})
                Transform { translation: Vec3::new(0.0, -0.25 - longueur / 2.0, 0.0), scale: Vec3::new(1.0, longueur, 1.0) }
            ]
        });
    }
}

/// Système Update : les ballons montent, descendent et se balancent.
// Res<Time> : temps ; elapsed_secs = secondes depuis le lancement.
// Query<(&mut A, &B)> : itère sur toutes les entités portant A et B (A modifiable).
// Quat::from_rotation_z : rotation autour de l'axe z.
// Book : https://bevy.org/learn/book/storing-data/queries/#mutable-and-immutable-query-data
// Book : https://bevy.org/learn/book/the-game-loop/game-time/
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.Res.html
// Doc : https://docs.rs/bevy/latest/bevy/time/struct.Time.html#method.elapsed_secs
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Query.html
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Quat.html#method.from_rotation_z
pub fn animer_ballons(temps: Res<Time>, mut ballons: Query<(&mut Transform, &Ballon)>) {
    let t = temps.elapsed_secs();
    for (mut tr, b) in &mut ballons {
        tr.translation.y = b.y_base + 0.12 * (t * 0.9 + b.phase).sin();
        tr.rotation = Quat::from_rotation_z(0.08 * (t * 0.7 + b.phase * 1.3).sin());
    }
}
