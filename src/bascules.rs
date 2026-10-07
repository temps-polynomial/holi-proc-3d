//! Bascules : montrer/masquer des calques d'objets et activer/désactiver des effets, au clavier
//! (F1-F12, K ; F pour la profondeur de champ) ou en cliquant dans le panneau ; Tab masque le panneau.
//! Ce module gère l'état, la visibilité des calques et l'interface ; chaque autre module lit
//! la ressource `Bascules` pour ses propres effets (halo, ombres, vent...).

use bevy::prelude::*;

/// Plugin : regroupe ressources et systèmes d'un domaine ; ajouté à l'App dans main.rs.
// Book : https://bevy.org/learn/book/modular-architecture/plugins/
// Doc : https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html
pub struct PluginBascules;

impl Plugin for PluginBascules {
    fn build(&self, app: &mut App) {
        // init_resource : ressource créée avec `Default::default()`.
        // Startup : exécuté une fois au lancement ; Update : à chaque image.
        // after : s'exécute après la saisie ; run_if : seulement si Bascules a changé.
        // Book : https://bevy.org/learn/book/storing-data/resources/#initializing-resources
        // Book : https://bevy.org/learn/book/the-game-loop/schedules/#the-standard-bevy-schedules
        // Book : https://bevy.org/learn/book/the-game-loop/schedules/#arranging-systems-in-schedules
        // Book : https://bevy.org/learn/book/control-flow/run-conditions/#run-conditions
        // Book : https://bevy.org/learn/book/control-flow/change-detection/#resources
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.init_resource
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_systems
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.IntoScheduleConfigs.html#method.after
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.IntoScheduleConfigs.html#method.run_if
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/common_conditions/fn.resource_changed.html
        // in_set : range le système dans l'ensemble SaisieBascules.
        // Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.IntoScheduleConfigs.html#method.in_set
        app.init_resource::<Bascules>()
            .add_systems(Startup, creer_interface)
            .add_systems(
                Update,
                (
                    saisie_bascules.in_set(SaisieBascules),
                    appliquer_bascules.after(SaisieBascules).run_if(resource_changed::<Bascules>),
                ),
            );
    }
}

/// Ensemble de systèmes contenant la saisie des bascules : les autres modules s'ordonnent
/// après lui (`.after(SaisieBascules)`) sans dépendre de la fonction elle-même.
// SystemSet : étiquette partagée pour ordonner ou conditionner un groupe de systèmes.
// Book : https://bevy.org/learn/book/control-flow/run-conditions/#run-conditions
// Doc : https://docs.rs/bevy/latest/bevy/ecs/schedule/trait.SystemSet.html
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SaisieBascules;

/// Une bascule par calque d'objets (Herbe..Ciel), puis une par effet (Profondeur..Vent).
// Default : requis pour l'utiliser dans un composant créé par `bsn!` (voir Calque).
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Bascule {
    #[default]
    Herbe,
    Arbres,
    Poudre,
    Decor,
    Rochers,
    Estrade,
    Collines,
    Ciel,
    Profondeur,
    Halo,
    Etalonnage,
    Ombres,
    Vent,
}

impl Bascule {
    /// Toutes les bascules, dans l'ordre du panneau.
    pub const TOUTES: [Bascule; 13] = [
        Bascule::Herbe, Bascule::Arbres, Bascule::Poudre, Bascule::Decor, Bascule::Rochers,
        Bascule::Estrade, Bascule::Collines, Bascule::Ciel, Bascule::Profondeur, Bascule::Halo,
        Bascule::Etalonnage, Bascule::Ombres, Bascule::Vent,
    ];

    /// (touche, nom affiché de la touche, libellé du panneau).
    // KeyCode : touche physique du clavier.
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/input/keyboard_input.rs#L13-L26
    // Doc : https://docs.rs/bevy/latest/bevy/input/keyboard/enum.KeyCode.html
    fn infos(self) -> (KeyCode, &'static str, &'static str) {
        match self {
            Bascule::Herbe => (KeyCode::F1, "F1", "Herbe & fleurs"),
            Bascule::Arbres => (KeyCode::F2, "F2", "Arbres"),
            Bascule::Poudre => (KeyCode::F3, "F3", "Poudre"),
            Bascule::Decor => (KeyCode::F4, "F4", "Cloture, fanions & ballons"),
            Bascule::Rochers => (KeyCode::F5, "F5", "Rochers & buissons"),
            Bascule::Estrade => (KeyCode::F6, "F6", "Estrade & canons"),
            Bascule::Collines => (KeyCode::F7, "F7", "Collines"),
            Bascule::Ciel => (KeyCode::F8, "F8", "Ciel"),
            Bascule::Profondeur => (KeyCode::F9, "F9", "Profondeur de champ"),
            Bascule::Halo => (KeyCode::F10, "F10", "Halo lumineux"),
            Bascule::Etalonnage => (KeyCode::F11, "F11", "Vignette & etalonnage"),
            Bascule::Ombres => (KeyCode::F12, "F12", "Ombres"),
            Bascule::Vent => (KeyCode::KeyK, "K", "Vent dans l'herbe"),
        }
    }
}

/// État des bascules (donnée globale unique -> ressource).
// Book : https://bevy.org/learn/book/storing-data/resources/
// Doc : https://docs.rs/bevy/latest/bevy/ecs/resource/trait.Resource.html
#[derive(Resource)]
pub struct Bascules {
    actif: [bool; 13],
    panneau: bool,
}

impl Default for Bascules {
    fn default() -> Self {
        Self { actif: [true; 13], panneau: true }
    }
}

impl Bascules {
    pub fn actif(&self, b: Bascule) -> bool {
        self.actif[b as usize]
    }
    fn inverser(&mut self, b: Bascule) {
        self.actif[b as usize] = !self.actif[b as usize];
    }
}

/// Bascule qui montre/masque l'entité.
// Component : donnée attachée à une entité.
// Default + Clone : requis pour utiliser le composant dans `bsn!` (gabarit FromTemplate automatique).
// Deref / DerefMut : le composant se comporte comme la Bascule qu'il contient (lecture / écriture) :
// `*calque` donne la Bascule (`**calque` depuis une référence &Calque, comme dans une requête), et
// les méthodes de Bascule s'appellent directement sur le composant. Remplace l'accès `calque.0`.
// Book : https://bevy.org/learn/book/storing-data/entities-components/#defining-components
// Book : https://bevy.org/learn/book/storing-data/designing-components/#guidance-for-structuring-components
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/math/bounding_2d.rs#L196-L197
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/math/bounding_2d.rs#L182-L184
// Doc : https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/template/trait.FromTemplate.html
// Doc : https://docs.rs/bevy/latest/bevy/prelude/derive.Deref.html
// Doc : https://docs.rs/bevy/latest/bevy/prelude/derive.DerefMut.html
// Rust : https://doc.rust-lang.org/book/ch15-02-deref.html#implementing-the-deref-trait
#[derive(Component, Default, Clone, Deref, DerefMut)]
pub struct Calque(pub Bascule);

/// Ligne du panneau (et son texte) associée à une bascule.
#[derive(Component, Default, Clone, Deref, DerefMut)]
struct BoutonBascule(Bascule);

/// Marqueur du panneau, pour le retrouver dans une requête.
#[derive(Component, Default, Clone)]
struct PanneauBascules;

/// Un maillage et son matériau, montrés ou masqués par la bascule `calque`.
// Fonction de scène : renvoie un morceau de scène BSN réutilisable dans d'autres `bsn!`.
// bsn! { A B C } décrit une entité portant les composants A, B et C.
// Mesh3d : maillage à afficher ; MeshMaterial3d::<M> : son matériau (type M explicite dans bsn!).
// Handle<T> : référence vers un asset stocké dans Assets<T>.
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/scene/bsn.rs#L38-L60
// Doc : https://docs.rs/bevy/latest/bevy/scene/macro.bsn.html
// Doc : https://docs.rs/bevy/latest/bevy/scene/trait.Scene.html
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh3d.html
// Doc : https://docs.rs/bevy/latest/bevy/pbr/struct.MeshMaterial3d.html
// Doc : https://docs.rs/bevy/latest/bevy/asset/enum.Handle.html
// Doc : https://docs.rs/bevy/latest/bevy/mesh/struct.Mesh.html
// Doc : https://docs.rs/bevy/latest/bevy/pbr/trait.Material.html
pub fn piece<M: Material>(maillage: Handle<Mesh>, materiau: Handle<M>, calque: Bascule) -> impl Scene {
    bsn! { Mesh3d(maillage) MeshMaterial3d::<M>(materiau) Calque(calque) }
}

/// Système Startup : titre en haut à gauche et panneau des bascules en bas à droite.
// Commands : modifications différées du monde ; spawn_scene : crée une entité décrite par bsn!.
// Text : texte d'interface. TextFont::font_size : taille (px). Node : boîte de mise en page
// (flexbox) ; Absolute = placée par rapport à la fenêtre. BackgroundColor : fond de la boîte.
// Book : https://bevy.org/learn/book/intro/the-next-three-letters/#commands
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/scene/bsn.rs#L51-L57
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/camera/camera_orbit.rs#L82-L97
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Commands.html
// Doc : https://docs.rs/bevy/latest/bevy/scene/trait.CommandsSceneExt.html#tymethod.spawn_scene
// Doc : https://docs.rs/bevy/latest/bevy/ui/widget/struct.Text.html
// Doc : https://docs.rs/bevy/latest/bevy/text/struct.TextFont.html
// Doc : https://docs.rs/bevy/latest/bevy/text/enum.FontSize.html
// Doc : https://docs.rs/bevy/latest/bevy/ui/struct.Node.html
// Doc : https://docs.rs/bevy/latest/bevy/ui/enum.PositionType.html
// Doc : https://docs.rs/bevy/latest/bevy/ui/fn.px.html
// Doc : https://docs.rs/bevy/latest/bevy/ui/struct.BackgroundColor.html
// Doc : https://docs.rs/bevy/latest/bevy/color/enum.Color.html#method.srgba
fn creer_interface(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Text("HOLI (Rust Bevy 0.19) - poudre statique\nglisser : orbite, maj+glisser : deplacer, molette : zoom")
        TextFont { font_size: px(16) }
        Node { position_type: PositionType::Absolute, top: px(12), left: px(14) } BackgroundColor(Color::srgba(0.1, 0.1, 0.25, 0.3))
    });
    // Panneau en colonne (FlexDirection::Column) : un titre puis une ligne-bouton par bascule.
    // Button : rend le nœud cliquable (ajoute Interaction). TextColor : couleur du texte.
    // Children [ ... ] : entités enfants ; la liste accepte un Vec de scènes, construit par itération.
    // Book : https://bevy.org/learn/book/storing-data/relations/#adding-children-declaratively
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/layout/flex_layout.rs#L26-L34
    // Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/widgets/button.rs#L86-L99
    // Doc : https://docs.rs/bevy/latest/bevy/ui/enum.FlexDirection.html
    // Doc : https://docs.rs/bevy/latest/bevy/ui/struct.UiRect.html
    // Doc : https://docs.rs/bevy/latest/bevy/ui/widget/struct.Button.html
    // Doc : https://docs.rs/bevy/latest/bevy/text/struct.TextColor.html
    // Doc : https://docs.rs/bevy/latest/bevy/ecs/hierarchy/struct.Children.html
    // Doc : https://docs.rs/bevy/latest/bevy/scene/trait.SceneList.html
    commands.spawn_scene(bsn! {
        PanneauBascules BackgroundColor(Color::srgba(0.05, 0.08, 0.18, 0.45))
        Node {
            position_type: PositionType::Absolute, bottom: px(12), right: px(12),
            flex_direction: FlexDirection::Column, row_gap: px(2), padding: UiRect::all(px(6)),
        }
        Children [
            Text("bascules (Tab masque)") TextFont { font_size: px(13) } TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
            {Bascule::TOUTES.iter().map(|&b| bsn! {
                // Node avant Button : garde FocusPolicy::Pass (ordre d'écriture = ordre de résolution).
                Node { padding: UiRect::axes(px(8), px(3)) } Button BoutonBascule(b) BackgroundColor(Color::srgba(0.2, 0.6, 0.3, 0.55))
                Children [Text("") TextFont { font_size: px(14) } BoutonBascule(b)]
            }).collect::<Vec<_>>()}
        ]
    });
}

/// Système Update : touches et clics inversent les bascules ; Tab montre/masque le panneau.
// Res/ResMut : accès en lecture/écriture à une ressource.
// ButtonInput<KeyCode> : état des touches (just_pressed = enfoncée à cette image).
// Changed<Interaction> : seulement les boutons dont l'état (survol, appui) vient de changer.
// Single : exactement une entité correspond (sinon le système est sauté).
// With<T> : filtre sur la présence de T sans le lire. Display::None retire le nœud de la mise en page.
// Book : https://bevy.org/learn/book/storing-data/resources/#accessing-resources
// Book : https://bevy.org/learn/book/control-flow/change-detection/#filtering
// Book : https://bevy.org/learn/book/storing-data/queries/#working-with-singleton-entities
// Book : https://bevy.org/learn/book/storing-data/queries/#anatomy-of-a-query
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/widgets/button.rs#L24-L46
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/layout/display_and_visibility.rs#L50-L56
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.Res.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/change_detection/struct.ResMut.html
// Doc : https://docs.rs/bevy/latest/bevy/input/struct.ButtonInput.html#method.just_pressed
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Query.html
// Doc : https://docs.rs/bevy/latest/bevy/ui/enum.Interaction.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/query/struct.Changed.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/system/struct.Single.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/query/struct.With.html
// Doc : https://docs.rs/bevy/latest/bevy/ui/enum.Display.html
fn saisie_bascules(
    touches: Res<ButtonInput<KeyCode>>,
    mut bascules: ResMut<Bascules>,
    boutons: Query<(&Interaction, &BoutonBascule), Changed<Interaction>>,
    mut panneau: Single<&mut Node, With<PanneauBascules>>,
) {
    // Écrire dans `bascules` (ResMut) le marque comme modifié : les systèmes en run_if s'exécuteront.
    for b in Bascule::TOUTES {
        if touches.just_pressed(b.infos().0) {
            bascules.inverser(b);
        }
    }
    if touches.just_pressed(KeyCode::KeyF) {
        bascules.inverser(Bascule::Profondeur);
    }
    for (interaction, bouton) in &boutons {
        if *interaction == Interaction::Pressed {
            // **bouton : la 1re * suit la référence &BoutonBascule, la 2e passe par Deref vers la Bascule.
            bascules.inverser(**bouton);
        }
    }
    if touches.just_pressed(KeyCode::Tab) {
        bascules.panneau = !bascules.panneau;
        panneau.display = if bascules.panneau { Display::Flex } else { Display::None };
    }
}

/// Système Update (seulement quand Bascules change) : visibilité des calques, libellés et
/// couleurs des lignes du panneau.
// Visibility::Hidden masque l'entité et ses enfants ; Inherited suit le parent.
// Without<Text> : rend `lignes` disjointe de `libelles` (les deux portent BoutonBascule).
// Book : https://bevy.org/learn/book/storing-data/queries/#mutable-and-immutable-query-data
// Ex : https://github.com/bevyengine/bevy/blob/release-0.19.1/examples/ui/layout/display_and_visibility.rs#L60-L72
// Doc : https://docs.rs/bevy/latest/bevy/camera/visibility/enum.Visibility.html
// Doc : https://docs.rs/bevy/latest/bevy/ecs/query/struct.Without.html
fn appliquer_bascules(
    bascules: Res<Bascules>,
    mut calques: Query<(&Calque, &mut Visibility)>,
    mut libelles: Query<(&mut Text, &BoutonBascule)>,
    mut lignes: Query<(&mut BackgroundColor, &BoutonBascule), Without<Text>>,
) {
    for (calque, mut vis) in &mut calques {
        *vis = if bascules.actif(**calque) { Visibility::Inherited } else { Visibility::Hidden };
    }
    for (mut texte, bouton) in &mut libelles {
        // infos() est une méthode de Bascule, appelée directement sur le composant grâce à Deref.
        let (_, nom_touche, libelle) = bouton.infos();
        let etat = if bascules.actif(**bouton) { "oui" } else { "non" };
        texte.0 = format!("[{nom_touche}]  {etat}   {libelle}");
    }
    for (mut fond, bouton) in &mut lignes {
        fond.0 = if bascules.actif(**bouton) { Color::srgba(0.2, 0.6, 0.3, 0.55) } else { Color::srgba(0.35, 0.35, 0.4, 0.45) };
    }
}
