//! Décor : prairie, collines, rochers, buissons, arbres, estrade et canons, clôture, fanions
//! et ballons. Les systèmes de création s'exécutent dans un ordre fixe (`chain`) car ils tirent
//! leurs nombres du même générateur : la scène est identique à chaque lancement.

mod arbres;
mod ballons;
mod cloture;
mod estrade;
mod nature;
mod terrain;

use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

use crate::alea::Alea;
use nature::MateriauNature;

/// Plugin : regroupe ressources, matériaux et systèmes d'un domaine ; ajouté à l'App dans main.rs.
// Book : https://bevy.org/learn/book/modular-architecture/plugins/
// Doc : https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html
pub struct PluginDecor;

impl Plugin for PluginDecor {
    fn build(&self, app: &mut App) {
        // MaterialPlugin : enregistre le matériau MateriauNature et son pipeline de rendu.
        // insert_resource : ressource avec une valeur donnée (le générateur et sa graine).
        // chain : exécute les systèmes dans l'ordre listé ; les Commands de chacun (ex. la
        // ressource Collines) sont appliquées avant le suivant.
        // Book : https://bevy.org/learn/book/storing-data/resources/#initializing-resources
        // Book : https://bevy.org/learn/book/the-game-loop/schedules/#arranging-systems-in-schedules
        // Book : https://bevy.org/learn/book/control-flow/commands/#when-do-commands-take-effect
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material.rs#L18-L20
        // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MaterialPlugin.html
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.insert_resource
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_systems
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.IntoScheduleConfigs.html#method.chain
        app.add_plugins(MaterialPlugin::<MateriauNature>::default())
            .insert_resource(AleaDecor(Alea::nouveau(3)))
            .add_systems(
                Startup,
                (
                    terrain::creer_prairie_et_collines,
                    estrade::creer_estrade,
                    arbres::creer_arbres,
                    terrain::creer_rochers_et_buissons,
                    cloture::creer_cloture_et_fanions,
                    ballons::creer_ballons,
                )
                    .chain(),
            )
            .add_systems(Update, ballons::animer_ballons);
    }
}

/// Générateur aléatoire partagé par les systèmes de création du décor.
// Resource : donnée globale unique.
// Book : https://bevy.org/learn/book/storing-data/resources/
// Doc : https://docs.rs/bevy/latest/bevy/ecs/resource/trait.Resource.html
// Deref / DerefMut : la ressource se comporte comme l'Alea qu'elle contient ; les systèmes
// appellent `alea.entre(...)` directement sur ResMut<AleaDecor>, sans `.0`.
// Book : https://bevy.org/learn/book/storing-data/designing-components/#guidance-for-structuring-components
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ecs/message.rs#L22-L25
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ecs/message.rs#L40-L45
// Doc : https://docs.rs/bevy/latest/bevy/prelude/derive.Deref.html
// Doc : https://docs.rs/bevy/latest/bevy/prelude/derive.DerefMut.html
// Rust : https://doc.rust-lang.org/book/ch15-02-deref.html#implementing-the-deref-trait
#[derive(Resource, Deref, DerefMut)]
struct AleaDecor(Alea);
