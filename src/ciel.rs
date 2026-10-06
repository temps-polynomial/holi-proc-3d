//! Ciel et lumière : dôme de ciel procédural (`holi_ciel.wgsl`), soleil (lumière directionnelle
//! dont les ombres suivent une bascule), lumière ambiante et couleur de fond.

use bevy::light::NotShadowCaster;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use crate::bascules::{Bascule, Bascules, SaisieBascules, piece};

/// Plugin : regroupe ressources, matériaux et systèmes d'un domaine ; ajouté à l'App dans main.rs.
// Book : https://bevy.org/learn/book/modular-architecture/plugins/
// Doc : https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html
pub struct PluginCiel;

impl Plugin for PluginCiel {
    fn build(&self, app: &mut App) {
        // MaterialPlugin : enregistre le matériau personnalisé et son pipeline de rendu.
        // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_material.rs#L12
        // Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MaterialPlugin.html
        app.add_plugins(MaterialPlugin::<CielHoli>::default())
            // Ressource avec une valeur donnée. ClearColor : couleur de fond par défaut des caméras.
            // Book : https://bevy.org/learn/book/storing-data/resources/#initializing-resources
            // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/window/clear_color.rs#L9
            // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.insert_resource
            // Doc : https://docs.rs/bevy/latest/bevy/camera/struct.ClearColor.html
            // Doc : https://docs.rs/bevy/latest/bevy/color/enum.Color.html#method.srgb
            .insert_resource(ClearColor(Color::srgb(0.6, 0.75, 0.95)))
            // Lumière ambiante appliquée à toute la scène ; `..default()` complète les autres champs.
            // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/motion_blur.rs#L60-L64
            // Doc : https://docs.rs/bevy/latest/bevy/light/struct.GlobalAmbientLight.html
            // Doc : https://docs.rs/bevy/latest/bevy/utils/fn.default.html
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.62, 0.75, 1.0),
                brightness: 900.0,
                ..default()
            })
            // Startup : une fois au lancement ; Update + run_if : seulement si Bascules a changé.
            // Book : https://bevy.org/learn/book/the-game-loop/schedules/#the-standard-bevy-schedules
            // Book : https://bevy.org/learn/book/control-flow/run-conditions/#run-conditions
            // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_systems
            // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.IntoScheduleConfigs.html#method.run_if
            // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/common_conditions/fn.resource_changed.html
            .add_systems(Startup, creer_ciel)
            .add_systems(Update, appliquer_ombres.after(SaisieBascules).run_if(resource_changed::<Bascules>));
    }
}

/// Direction vers le soleil (normalisée), partagée avec la poudre.
// Vec3 : vecteur 3D (glam) ; méthodes (normalize...) sur la même page.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/transforms/transform.rs#L107-L110
// Doc : https://docs.rs/bevy/latest/bevy/math/struct.Vec3.html
pub fn dir_soleil() -> Vec3 {
    Vec3::new(-0.85, 0.7, 0.05).normalize()
}

/// Marqueur du soleil, pour le retrouver dans une requête.
// Component : donnée attachée à une entité. Default + Clone : requis par `bsn!`.
// Book : https://bevy.org/learn/book/storing-data/entities-components/#defining-components
// Doc : https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html
#[derive(Component, Default, Clone)]
struct Soleil;

/// Paramètres du ciel (struct `CielHoli` de `holi_ciel.wgsl`).
// ShaderType : calcule la disposition mémoire GPU (alignements WGSL) de la struct ;
// l'ordre et les types des champs doivent correspondre à la struct WGSL.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/extended_material_bindless.rs#L71-L77
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.ShaderType.html
#[derive(ShaderType, Debug, Clone)]
struct ParamsCiel {
    dir_soleil: Vec3,
    gain: f32,
}

/// Dôme de ciel procédural (dégradé, halo solaire, cumulus).
// Asset : le matériau est stocké dans Assets<CielHoli> et référencé par Handle.
// TypePath : nom de type stable, exigé par Asset.
// AsBindGroup : génère le bind group à partir des attributs de champ ;
// uniform(0) = @binding(0) dans le shader (groupe MATERIAL_BIND_GROUP).
// Book : https://bevy.org/learn/book/assets/custom-assets/#defining-an-asset-type
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_material.rs#L42-L51
// Doc : https://docs.rs/bevy/latest/bevy/asset/trait.Asset.html
// Doc : https://docs.rs/bevy/latest/bevy/reflect/trait.TypePath.html
// Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/trait.AsBindGroup.html
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CielHoli {
    #[uniform(0)]
    params: ParamsCiel,
}

// Material : décrit les shaders et l'état de rendu ; chaque méthode a une valeur par défaut.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader/shader_material.rs#L53-L63
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html
impl Material for CielHoli {
    // Fragment shader personnalisé ; le chemin est relatif au dossier `assets/`.
    // Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html#method.fragment_shader
    // Doc : https://docs.rs/bevy/latest/bevy/shader/enum.ShaderRef.html
    fn fragment_shader() -> ShaderRef {
        "shaders/holi_ciel.wgsl".into()
    }
    // false : le matériau n'est pas dessiné dans les cartes d'ombre (ne projette pas d'ombre).
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html#method.enable_shadows
    fn enable_shadows() -> bool {
        false
    }
    // Ajuste le pipeline de rendu généré pour ce matériau.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/shader_advanced/custom_vertex_attribute.rs#L74-L86
    // Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html#method.specialize
    // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/struct.RenderPipelineDescriptor.html
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // on regarde le dôme de l'intérieur : on élimine les faces avant au lieu des faces arrière
        // Doc : https://docs.rs/bevy/latest/bevy/render/render_resource/enum.Face.html
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

/// Système Startup : dôme de ciel et soleil.
// Commands : modifications différées du monde ; ResMut<Assets<T>> : écriture dans la collection
// d'assets de type T ; add : stocke l'asset et renvoie son Handle.
// spawn_scene(bsn! {...}) : crée une entité ; #Nom ajoute un composant Name.
// Sphere : primitive convertie en Mesh par `add`. NotShadowCaster : ne projette pas d'ombre.
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Book : https://bevy.org/learn/book/assets/bevy-s-asset-framework/#the-basics-of-loading-assets
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L13-L37
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/shadow_caster_receiver.rs#L49-L54
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/struct.Assets.html#method.add
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
// Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
// Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/name/struct.Name.html
// Doc : https://docs.rs/bevy/latest/bevy/math/primitives/struct.Sphere.html
// Doc : https://docs.rs/bevy/latest/bevy/light/struct.NotShadowCaster.html
fn creer_ciel(mut commands: Commands, mut maillages: ResMut<Assets<Mesh>>, mut ciels: ResMut<Assets<CielHoli>>) {
    let ciel = ciels.add(CielHoli { params: ParamsCiel { dir_soleil: dir_soleil(), gain: 1.0 } });
    commands.spawn_scene(bsn! { #Ciel piece(maillages.add(Sphere::new(700.0)), ciel, Bascule::Ciel) NotShadowCaster });

    // DirectionalLight : lumière à rayons parallèles (soleil), orientée par la rotation du Transform.
    // template_value(x) : remplace entièrement le composant par x (ici une chaîne d'appels de méthodes).
    // looking_at(cible, haut) : oriente -Z local vers la cible.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/lighting.rs#L190-L200
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/3d/3d_scene.rs#L33-L36
    // Doc : https://docs.rs/bevy/latest/bevy/light/struct.DirectionalLight.html
    // Doc : https://docs.rs/bevy/latest/bevy/scene/fn.template_value.html
    // Doc : https://docs.rs/bevy/latest/bevy/transform/components/struct.Transform.html#method.looking_at
    commands.spawn_scene(bsn! {
        Soleil DirectionalLight { illuminance: 16000.0, shadow_maps_enabled: true, color: Color::srgb(1.0, 0.88, 0.7) }
        template_value(Transform::from_translation(dir_soleil() * 30.0).looking_at(Vec3::ZERO, Vec3::Y))
    });
}

/// Système Update (seulement quand Bascules change) : active ou coupe les ombres du soleil.
// Res<T> : lecture d'une ressource. Single : exactement une entité correspond ;
// With<Soleil> : filtre sur la présence du marqueur.
// Book : https://bevy.org/learn/book/storing-data/queries/#working-with-singleton-entities
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.Res.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Single.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/query/struct.With.html
fn appliquer_ombres(bascules: Res<Bascules>, mut soleil: Single<&mut DirectionalLight, With<Soleil>>) {
    soleil.shadow_maps_enabled = bascules.actif(Bascule::Ombres);
}
