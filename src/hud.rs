//! HUD: contatore FPS in alto a destra, attivabile/disattivabile dal menu.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::sprite::Anchor;

/// Risorsa che decide se mostrare il contatore FPS.
#[derive(Resource, Default)]
pub struct ShowFps(pub bool);

/// Marker per l'entità testo FPS.
#[derive(Component)]
pub struct FpsText;

/// Costruisce l'HUD FPS: un'entità UI Text2d persistente, non soggetta
/// al reset della scena 3D.
///
/// L'anchor di default di `Text2d` è il centro del testo; lo impostiamo
/// in alto a destra così il testo resta ancorato al bordo destro della
/// viewport anche se la finestra viene ridimensionata (la posizione viene
/// ricalcolata ogni frame in [`update_fps`]).
pub fn setup_fps(mut commands: Commands, show: Res<ShowFps>) {
    let visibility = if show.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    commands.spawn((
        FpsText,
        Text2d::new("FPS: --"),
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        TextFont {
            font_size: bevy::text::FontSize::Px(18.0),
            ..default()
        },
        Anchor::TOP_RIGHT,
        Transform::from_xyz(620.0, 340.0, 0.0),
        visibility,
    ));
}

/// Aggiorna il testo FPS leggendo il diagnostic del plugin ufficiale.
/// Se `ShowFps` è spento, nasconde il testo invece di despawnarlo così
/// può essere riattivato istantaneamente dal menu.
pub fn update_fps(
    show: Res<ShowFps>,
    diagnostics: Res<DiagnosticsStore>,
    mut text: Query<(&mut Text2d, &mut Visibility, &mut Transform), With<FpsText>>,
    camera: Query<&Camera>,
) {
    let Ok((mut text_2d, mut visibility, mut transform)) = text.single_mut() else {
        return;
    };

    // Mostra o nascondi secondo la risorsa.
    if !show.0 {
        *visibility = Visibility::Hidden;
        return;
    }
    *visibility = Visibility::Visible;

    // Legge il diagnostic FPS; se non disponibile mostra "--".
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or_default();
    text_2d.0 = format!("FPS: {:.0}", fps);

    // Posiziona il testo in alto a destra della viewport corrente.
    // 620/340 ≈ (1280/2, 720/2) meno un margine di 20 px.
    if let Ok(cam) = camera.single() {
        let Some(rect) = cam.logical_viewport_rect() else {
            return;
        };
        let size = rect.max - rect.min;
        transform.translation.x = size.x * 0.5 - 20.0;
        transform.translation.y = size.y * 0.5 - 20.0;
    }
}
