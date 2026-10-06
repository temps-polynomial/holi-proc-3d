//! HOLI : une prairie de fête des couleurs avec Bevy 0.19.
//!
//! Prairie + herbe agitée par le vent + fleurs, arbres générés, rochers, buissons, clôture,
//! fanions, ballons, canons à poudre, ciel, la chaîne d'image de la caméra (HDR, ACES,
//! étalonnage, halo, vignette, flou de profondeur de champ) et un panneau de bascules.
//! Les nuages de poudre sont un volume statique calculé sur le CPU.
//!
//! Commandes : glisser = orbite, maj+glisser = déplacer, molette = zoom.
//! F1-F12 / K (ou clic sur les lignes du panneau) basculent calques et effets ; Tab masque le panneau.
//!
//! Organisation : un plugin par domaine, chaque fichier se lit de haut en bas
//! (plugin, données, création des entités, systèmes). Ordre de lecture conseillé :
//!   bascules.rs  état des bascules, calques, interface (ressources, composants, UI, entrées)
//!   camera.rs    caméra orbitale et post-traitement (Single, souris, Time)
//!   ciel.rs      dôme de ciel, soleil, lumière (premier matériau personnalisé)
//!   herbe.rs     herbe au vent (StandardMaterial étendu + vertex shader)
//!   poudre/      volume de poudre (textures 3D + raymarching)
//!   decor/       prairie, collines, arbres, rochers, estrade, clôture, fanions, ballons
//!   alea.rs, maillage.rs : outils sans logique Bevy (hasard, assemblage de maillages)

// Modules Rust du projet ; `poudre` et `decor` sont des dossiers (mod.rs + sous-modules).
// Book : https://bevy.org/learn/book/modular-architecture/project-organization/#modules
mod alea;
mod bascules;
mod camera;
mod ciel;
mod decor;
mod herbe;
mod maillage;
mod poudre;

// Prélude : importe les types les plus courants (App, Commands, Query, Transform...).
// Doc : https://docs.rs/bevy/latest/bevy/prelude/index.html
use bevy::prelude::*;

/// Couleurs de la fête, partagées par la poudre, les canons, les fanions et les ballons.
const PALETTE: [[f32; 3]; 8] = [
    [1.00, 0.16, 0.55], // rose vif
    [1.00, 0.50, 0.05], // orange
    [1.00, 0.88, 0.08], // jaune
    [0.45, 0.92, 0.12], // citron vert
    [0.05, 0.85, 0.95], // cyan
    [0.15, 0.40, 1.00], // bleu
    [0.55, 0.20, 1.00], // violet
    [0.95, 0.15, 0.95], // magenta
];

fn main() {
    // App : construit l'application (plugins, ressources, systèmes) puis lance la boucle.
    // Book : https://bevy.org/learn/book/intro/apps-worlds/#apps
    // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html
    App::new()
        // DefaultPlugins : fenêtre, rendu, entrées, assets, UI, scènes...
        // Book : https://bevy.org/learn/book/modular-architecture/plugins/#plugin-groups
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.add_plugins
        // Doc : https://docs.rs/bevy/latest/bevy/prelude/struct.DefaultPlugins.html
        .add_plugins(DefaultPlugins)
        // Plugins du projet, un par domaine (chacun enregistre ses ressources, matériaux et systèmes).
        // Book : https://bevy.org/learn/book/modular-architecture/plugins/
        // Book : https://bevy.org/learn/book/modular-architecture/project-organization/
        .add_plugins((
            bascules::PluginBascules,
            camera::PluginCamera,
            ciel::PluginCiel,
            herbe::PluginHerbe,
            poudre::PluginPoudre,
            decor::PluginDecor,
        ))
        // Doc : https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.run
        .run();
}
