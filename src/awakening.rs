//! Il risveglio sulla spiaggia: la creazione del personaggio dentro la storia (MP-01, ADR 0047).
//!
//! Una partita nuova comincia senza schermata di creazione. Sulla spiaggia, tra i rottami:
//! 1. una **pozza d'acqua ferma** fra le rocce: guardandoti (E) scegli l'età;
//! 2. quattro **oggetti tra i rottami** (una spada, un arco, un rotolo di nodi, una borsa): il primo che
//!    raccogli (E) dice la tua indole, cioè la classe, che puoi accettare o sostituire scegliendo tu;
//! 3. **Gaspare** ti chiede il nome: lo scrivi tu, e la cartella dei salvataggi lo prende.
//!
//! Le tappe della missione `mp01` aspettano le tre fasi (`quando: conta risveglio <n>`).

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::AppState;
use crate::character::{AWAKE, Age, CUSTOM_POINTS, Character, Class, STAT_MAX, Stat, Stats};
use crate::inventory::{Inventory, ItemKind, Relic};

/// La pozza dove ci si specchia, e i quattro oggetti tra i rottami (classe, punto).
pub const POOL: Vec2 = Vec2::new(34.0, 84.5);
pub const RELICS: [(Class, Vec2); 4] = [
    // Il Guerriero lontano dalle botti, che lo coprivano (segnalazione 2026-11-18_22-49-24).
    (Class::Warrior, Vec2::new(25.8, 87.2)),
    (Class::Explorer, Vec2::new(28.0, 89.0)),
    (Class::Scholar, Vec2::new(32.0, 89.0)),
    (Class::Merchant, Vec2::new(33.5, 87.5)),
];
const REACH: f32 = 1.8;
/// Distanza (m) dal centro del mirino entro cui un oggetto dei rottami è «guardato».
const RELIC_AIM: f32 = 0.7;
/// Il centro dei rottami, e la distanza oltre la quale gli oggetti non scelti spariscono.
const RELICS_CENTER: Vec2 = Vec2::new(30.5, 87.5);
const RELICS_HIDE: f32 = 14.0;

/// Detto il nome, chi l'ha chiesto risponde subito (ADR 0059): lo legge `entities::interact`.
static GREET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// C'è un saluto in attesa dopo il nome?
pub fn greeting_pending() -> bool {
    GREET.load(std::sync::atomic::Ordering::Relaxed)
}

/// Il saluto è stato detto.
pub fn greeting_done() {
    GREET.store(false, std::sync::atomic::Ordering::Relaxed);
}
/// Punti statistica in più alla fine della creazione, da spendere nel pannello C.
pub const BONUS_POINTS: u32 = 2;

/// Quello che il corpo ricorda: l'oggetto tra i rottami e la classe che rivela.
pub fn relic_of(class: Class) -> (&'static str, &'static str) {
    match class {
        Class::Warrior => (
            "una sbarra di ferro, che la tua mano stringe nel punto esatto di bilanciamento",
            "Ti alzi già in guardia: la memoria del bronzo e del ferro.",
        ),
        Class::Explorer => (
            "un arco spezzato a metà, e lo sguardo che corre all'orizzonte",
            "Non guardi i tuoi piedi ma il sentiero sulla scogliera: la memoria del vento e delle stelle.",
        ),
        Class::Scholar => (
            "un rotolo di cordicelle annodate: sette nodi che sembrano dire qualcosa",
            "Noti subito ciò che agli altri sfugge: la memoria dei segni e dei nodi.",
        ),
        Class::Merchant => (
            "una borsa di cuoio con qualche moneta coniata in un porto lontano",
            "Prima ancora di alzarti conti ciò che si è salvato: la memoria degli scambi.",
        ),
        Class::Custom => (
            "",
            "Il mare ha sciolto ogni stampo: sei «dai molti modi», come Odisseo.",
        ),
    }
}

/// Ciò che si porta via raccogliendo l'oggetto della propria indole.
pub fn kit_of(class: Class) -> Vec<(ItemKind, u32)> {
    match class {
        Class::Warrior => vec![(ItemKind::ShortSword, 1)],
        Class::Explorer => vec![(ItemKind::Bow, 1), (ItemKind::Arrow, 12)],
        Class::Scholar => vec![
            (ItemKind::Relic(Relic::Quipu), 1),
            (ItemKind::HealthPotion, 2),
        ],
        Class::Merchant => vec![(ItemKind::Satchel, 1), (ItemKind::Coin, 15)],
        Class::Custom => Vec::new(),
    }
}

/// Punti ancora da distribuire nella scelta libera, partendo da 8 in tutto.
pub fn points_left(stats: Stats) -> i32 {
    let base = i32::from(Class::Custom.stats().get(Stat::Strength));
    let spent: i32 = Stat::ALL
        .iter()
        .map(|s| i32::from(stats.get(*s)) - base)
        .sum();
    i32::from(CUSTOM_POINTS) - spent
}

/// Pannello aperto durante il risveglio.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Panel {
    #[default]
    Mirror,
    /// L'indole rivelata dall'oggetto raccolto.
    Class(Class),
    /// La scelta libera delle statistiche.
    Custom,
    Name,
}

/// Stato dei pannelli del risveglio.
#[derive(Resource, Debug)]
pub struct Awakening {
    pub panel: Panel,
    pub custom: Stats,
    pub name: String,
    /// Classe scelta dal pilota automatico o dalla shell (le prove non cliccano i pannelli).
    pub request: Option<Request>,
}

impl Default for Awakening {
    fn default() -> Self {
        Self {
            panel: Panel::Mirror,
            custom: Class::Custom.stats(),
            name: String::new(),
            request: None,
        }
    }
}

/// Una risposta ai pannelli data dalla shell (`risveglio eta <…>`, `risveglio classe <…>`, `risveglio nome <…>`).
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Age(Age),
    Class(Class),
    Name(String),
}

/// Un oggetto tra i rottami.
#[derive(Component)]
pub struct WreckRelic(pub Class);

/// La pozza d'acqua.
#[derive(Component)]
pub struct MirrorPool;

fn awake_needed(character: &Character, missions: &crate::quests::missions::Missions) -> bool {
    character.awakening < AWAKE
        && missions.enabled
        && crate::region::current() == crate::region::Region::Isola
}

/// Sistema: la pozza e i quattro oggetti sulla spiaggia, finché il risveglio non è finito.
#[allow(clippy::too_many_arguments)]
pub fn stage_beach(
    mut commands: Commands,
    character: Res<Character>,
    missions: Res<crate::quests::missions::Missions>,
    field: Option<Res<crate::terrain::TerrainField>>,
    pbr: Res<crate::pbr::PbrMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    pools: Query<Entity, With<MirrorPool>>,
    relics: Query<(Entity, &WreckRelic)>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
) {
    let Some(field) = field else {
        return;
    };
    // Gli oggetti dei rottami compaiono e spariscono solo lontano dagli occhi (ADR 0059).
    let far = players.single().map_or(true, |p| {
        p.translation().xz().distance(RELICS_CENTER) > RELICS_HIDE
    });
    if !awake_needed(&character, &missions) {
        for e in pools.iter().chain(relics.iter().map(|(e, _)| e)) {
            commands.entity(e).despawn();
        }
        return;
    }
    let ground = |p: Vec2| Vec3::new(p.x, field.height_at(p.x, p.y), p.y);
    if character.awakening == 0 && pools.is_empty() {
        let rock = pbr.get(crate::pbr::Mat::Stone);
        let wood = pbr.get(crate::pbr::Mat::Planks);
        // L'anello stende la texture una volta su tutto il giro (6 m) e su tutta l'altezza: la
        // pietra si ripete invece ogni 2,5 m, come sugli edifici.
        let ring_rock = materials
            .get(&rock)
            .cloned()
            .map(|mut m| {
                m.uv_transform = bevy::math::Affine2::from_scale(Vec2::new(2.4, 0.48));
                materials.add(m)
            })
            .unwrap_or_else(|| rock.clone());
        // Acqua scura e ferma, quasi uno specchio: non se ne vede il fondo (ci si specchia).
        let water = materials.add(StandardMaterial {
            base_color: Color::srgba(0.02, 0.06, 0.08, 0.94),
            reflectance: 0.9,
            perceptual_roughness: 0.02,
            ..crate::buildings::clear_water_material()
        });
        let g = ground(POOL);
        let upright = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        commands
            .spawn((
                MirrorPool,
                crate::scene::SceneNode,
                Transform::from_translation(g),
                Visibility::default(),
                avian3d::prelude::RigidBody::Static,
            ))
            .with_children(|c| {
                // Un pozzo vero (segnalazione 2026-11-18_22-36-48: «non sembra un pozzo, non è
                // fondo, sembra una pozzanghera», e le pietre del bordo si sovrapponevano agli
                // angoli interni): parapetto tondo di pietra alto 80 cm, copertina, l'acqua scura
                // giù dentro, i due montanti con la traversa, la corda e il secchio.
                const RIM_H: f32 = 0.8;
                c.spawn((
                    Mesh3d(meshes.add(crate::pbr::with_tangents(Extrusion::new(
                        Annulus::new(0.62, 0.95),
                        RIM_H + 0.4,
                    )))),
                    MeshMaterial3d(ring_rock.clone()),
                    // Scende 40 cm sotto il terreno: sulla spiaggia in pendenza non resta sospeso.
                    Transform::from_xyz(0.0, RIM_H / 2.0 - 0.2, 0.0).with_rotation(upright),
                ));
                c.spawn((
                    Mesh3d(meshes.add(crate::pbr::with_tangents(Extrusion::new(
                        Annulus::new(0.58, 1.0),
                        0.08,
                    )))),
                    // Liscia: sulle facce piatte dell'anello la texture girava a vortice.
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.5, 0.48, 0.44),
                        perceptual_roughness: 0.85,
                        ..default()
                    })),
                    Transform::from_xyz(0.0, RIM_H + 0.04, 0.0).with_rotation(upright),
                ));
                c.spawn((
                    Mesh3d(meshes.add(Circle::new(0.62))),
                    MeshMaterial3d(water),
                    Transform::from_xyz(0.0, 0.2, 0.0).with_rotation(upright),
                    bevy::light::NotShadowCaster,
                ));
                // Montanti, traversa, corda e secchio.
                let post = meshes.add(crate::pbr::with_tangents(Cuboid::new(0.12, 1.7, 0.12)));
                for x in [-1.05f32, 1.05] {
                    c.spawn((
                        Mesh3d(post.clone()),
                        MeshMaterial3d(wood.clone()),
                        Transform::from_xyz(x, 0.85, 0.0),
                    ));
                }
                c.spawn((
                    Mesh3d(meshes.add(crate::pbr::with_tangents(Cuboid::new(2.3, 0.1, 0.1)))),
                    MeshMaterial3d(wood.clone()),
                    Transform::from_xyz(0.0, 1.62, 0.0),
                ));
                c.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.012, 0.6))),
                    MeshMaterial3d(wood.clone()),
                    Transform::from_xyz(0.0, 1.27, 0.0),
                ));
                c.spawn((
                    Mesh3d(meshes.add(crate::pbr::with_tangents(Cylinder::new(0.13, 0.22)))),
                    MeshMaterial3d(wood.clone()),
                    Transform::from_xyz(0.0, 0.88, 0.0),
                ));
                // Solido: il parapetto non si attraversa.
                c.spawn((
                    avian3d::prelude::Collider::cylinder(0.95, RIM_H + 0.08),
                    Transform::from_xyz(0.0, (RIM_H + 0.08) / 2.0, 0.0),
                    crate::camera::BlocksCamera,
                ));
            });
    }
    if character.awakening <= 1 && relics.is_empty() {
        let iron = pbr.get(crate::pbr::Mat::Iron);
        let wood = pbr.get(crate::pbr::Mat::Planks);
        let cloth = pbr.get(crate::pbr::Mat::Cloth);
        for (class, p) in RELICS {
            let (mesh, mat, lift): (Mesh, Handle<StandardMaterial>, f32) = match class {
                Class::Warrior => (Cuboid::new(0.07, 0.07, 1.1).into(), iron.clone(), 0.05),
                Class::Explorer => (Torus::new(0.42, 0.47).into(), wood.clone(), 0.04),
                Class::Scholar => (Cylinder::new(0.09, 0.45).into(), cloth.clone(), 0.09),
                _ => (Sphere::new(0.16).into(), wood.clone(), 0.12),
            };
            commands.spawn((
                WreckRelic(class),
                crate::scene::SceneNode,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(mat),
                Transform::from_translation(ground(p) + Vec3::Y * lift)
                    .with_rotation(Quat::from_rotation_y(p.x * 0.7) * Quat::from_rotation_z(1.4)),
            ));
        }
    } else if character.awakening > 1 && far {
        for (e, _) in &relics {
            commands.entity(e).despawn();
        }
    }
}

/// Sistema: vicino alla pozza o a un oggetto, E apre il pannello; dopo il dialogo con Gaspare si
/// chiede il nome.
#[allow(clippy::too_many_arguments)]
pub fn beach_prompts(
    keys: Res<ButtonInput<KeyCode>>,
    character: Res<Character>,
    missions: Res<crate::quests::missions::Missions>,
    dialog: Res<crate::entities::DialogState>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    relics: Query<(&GlobalTransform, &WreckRelic)>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut awakening: ResMut<Awakening>,
    mut nearest: ResMut<crate::entities::NearestInteract>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !awake_needed(&character, &missions) || dialog.open {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation().xz();
    let mut offer = |text: &str, panel: Panel| {
        nearest.prompt = Some(text.to_string());
        if keys.just_pressed(KeyCode::KeyE) {
            awakening.panel = panel;
            next.set(AppState::Awakening);
        }
    };
    match character.awakening {
        0 if p.distance(POOL) <= REACH + 0.6 => {
            offer("[E] Guarda il tuo riflesso", Panel::Mirror);
        }
        1 => {
            // Vale l'oggetto nel mirino; se non se ne guarda nessuno, solo se ce n'è uno a portata.
            // Prima vinceva il primo trovato, anche alle spalle (segnalazione 2026-11-18_22-46-44).
            let near: Vec<_> = relics
                .iter()
                .filter(|(t, _)| t.translation().xz().distance(p) <= REACH)
                .collect();
            let aimed = cameras.single().ok().and_then(|cam| {
                let (o, f) = (cam.translation(), *cam.forward());
                near.iter()
                    .filter_map(|(t, r)| {
                        let to = t.translation() - o;
                        let along = to.dot(f);
                        let off = (to - f * along).length();
                        (along > 0.0 && off < RELIC_AIM).then_some((off, r.0))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .map(|(_, c)| c)
            });
            let pick = aimed.or_else(|| (near.len() == 1).then(|| near[0].1.0));
            if let Some(class) = pick {
                offer("[E] Raccogli", Panel::Class(class));
            }
        }
        // Gaspare ha chiesto il nome: il pannello si apre da solo.
        2 if missions.step_of("mp01") == Some(3) => {
            awakening.panel = Panel::Name;
            next.set(AppState::Awakening);
        }
        _ => {}
    }
}

/// Applica una risposta (dal pannello o dalla shell). Rende `true` se il pannello va chiuso.
#[allow(clippy::too_many_arguments)]
fn answer(
    request: Request,
    character: &mut Character,
    awakening: &mut Awakening,
    inventory: &mut Inventory,
    progress: &mut crate::skills::Progress,
    session: &mut crate::save::Session,
) -> bool {
    match request {
        Request::Age(age) => {
            character.age = age;
            character.awakening = character.awakening.max(1);
            session.notify(format!("Nell'acqua ferma: {}", age.name().to_lowercase()));
            info!("[RISVEGLIO] età: {}", age.key());
            true
        }
        Request::Class(class) => {
            let base = if class == Class::Custom {
                awakening.custom
            } else {
                class.stats()
            };
            character.class = class;
            character.stats = character.age.apply(base);
            character.awakening = character.awakening.max(2);
            for (kind, n) in kit_of(class) {
                inventory.add(kind, n);
            }
            progress.stat_points += BONUS_POINTS;
            session.notify(format!(
                "{}: hai {BONUS_POINTS} punti statistica da spendere (C)",
                class.name()
            ));
            info!("[RISVEGLIO] indole: {}", class.name());
            true
        }
        Request::Name(name) => {
            let name = name.replace(['|', '\n', '='], " ").trim().to_string();
            if crate::save::folder_name(&name).is_none() {
                return false;
            }
            character.name = name.clone();
            character.awakening = AWAKE;
            crate::save::rename_session(session, &name);
            session.notify(format!("Non sei più Nessuno: sei {name}"));
            GREET.store(true, std::sync::atomic::Ordering::Relaxed);
            info!("[RISVEGLIO] nome: {name}");
            true
        }
    }
}

/// Sistema (stato `Awakening`): i tre pannelli del risveglio, nello stile dei dialoghi.
#[allow(clippy::too_many_arguments)]
pub fn awakening_window(
    mut contexts: EguiContexts,
    mut character: ResMut<Character>,
    mut awakening: ResMut<Awakening>,
    mut inventory: ResMut<Inventory>,
    mut progress: ResMut<crate::skills::Progress>,
    mut session: ResMut<crate::save::Session>,
    mut next: ResMut<NextState<AppState>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let mut chosen = awakening.request.take();
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let grey = egui::Color32::from_rgb(170, 178, 196);
    let frame = egui::Frame::default()
        .fill(crate::ui_style::PANEL_FILL)
        .stroke(egui::Stroke::new(2.0, crate::ui_style::GOLD))
        .inner_margin(egui::Margin::symmetric(26, 18))
        .corner_radius(egui::CornerRadius::same(10));
    let panel = awakening.panel;
    let mut back_to_class = None;
    egui::Area::new(egui::Id::new("awakening_panel"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.set_width(640.0_f32.min(ctx.viewport_rect().width() - 80.0));
                let heading = |ui: &mut egui::Ui, t: &str| {
                    ui.label(
                        egui::RichText::new(t)
                            .strong()
                            .size(24.0)
                            .color(crate::ui_style::GOLD),
                    );
                };
                match panel {
                    Panel::Mirror => {
                        heading(ui, "Lo Specchio della Marea");
                        ui.label(
                            "Il mare cancella le storie, ma non le primavere che hai sulle spalle. \
                             Che cosa vedi nell'acqua ferma?",
                        );
                        ui.add_space(8.0);
                        for age in Age::ALL {
                            if ui
                                .button(egui::RichText::new(age.name()).size(19.0).strong())
                                .clicked()
                            {
                                chosen = Some(Request::Age(age));
                            }
                            ui.label(egui::RichText::new(age.mirror()).size(14.0).color(grey));
                            ui.add_space(4.0);
                        }
                    }
                    Panel::Class(class) => {
                        let (thing, memory) = relic_of(class);
                        heading(ui, class.name());
                        ui.label(format!("Tra i rottami raccogli {thing}."));
                        ui.label(egui::RichText::new(memory).italics());
                        ui.add_space(6.0);
                        stats_rows(ui, character.age.apply(class.stats()), grey);
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Sì: è la mia indole").clicked() {
                                chosen = Some(Request::Class(class));
                            }
                            if ui.button("No: scelgo io i miei modi").clicked() {
                                awakening.panel = Panel::Custom;
                            }
                            // Raccolto l'oggetto sbagliato si torna tra i rottami senza scegliere.
                            if ui.button("Lascialo lì").clicked() {
                                back_to_class = Some(());
                            }
                        });
                    }
                    Panel::Custom => {
                        heading(ui, "Dai molti modi");
                        ui.label(relic_of(Class::Custom).1);
                        let left = points_left(awakening.custom);
                        ui.label(format!("Punti da distribuire: {left}"));
                        for stat in Stat::ALL {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [130.0, 24.0],
                                    egui::Label::new(egui::RichText::new(stat.name()).strong()),
                                );
                                let value = awakening.custom.get(stat);
                                let base = Class::Custom.stats().get(stat);
                                if ui
                                    .add_enabled(value > base, egui::Button::new(" − "))
                                    .clicked()
                                {
                                    awakening.custom.set(stat, value - 1);
                                }
                                ui.add_sized([34.0, 24.0], egui::Label::new(value.to_string()));
                                if ui
                                    .add_enabled(left > 0 && value < STAT_MAX, egui::Button::new(" + "))
                                    .clicked()
                                {
                                    awakening.custom.set(stat, value + 1);
                                }
                                ui.label(egui::RichText::new(stat.effect()).size(14.0).color(grey));
                            });
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(left == 0, egui::Button::new("Fatto"))
                                .clicked()
                            {
                                chosen = Some(Request::Class(Class::Custom));
                            }
                            if ui.button("Indietro").clicked() {
                                back_to_class = Some(());
                            }
                        });
                    }
                    Panel::Name => {
                        heading(ui, "Il tuo nome");
                        ui.label(
                            "«Finché non hai un nome, per il mare sei ancora Nessuno. Dimmi tu quale \
                             nome senti tuo adesso, e io ti chiamerò così davanti a tutto il paese.»",
                        );
                        ui.add_space(8.0);
                        let edit = ui.add(
                            egui::TextEdit::singleline(&mut awakening.name)
                                .desired_width(320.0)
                                .hint_text("il tuo nome"),
                        );
                        edit.request_focus();
                        let ok = crate::save::folder_name(&awakening.name).is_some();
                        if !ok && !awakening.name.trim().is_empty() {
                            ui.label(
                                egui::RichText::new("Questo nome non si può usare.")
                                    .color(egui::Color32::from_rgb(255, 170, 130)),
                            );
                        }
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui.add_enabled(ok, egui::Button::new("Dillo a Gaspare")).clicked()
                            || (ok && enter)
                        {
                            chosen = Some(Request::Name(awakening.name.clone()));
                        }
                    }
                }
            });
        });
    // Esc chiude l'indole e la scelta libera come «Lascialo lì» e «Indietro».
    if matches!(panel, Panel::Class(_) | Panel::Custom) && keys.just_pressed(KeyCode::Escape) {
        back_to_class = Some(());
    }
    if back_to_class.is_some() {
        awakening.panel = Panel::Mirror;
        next.set(AppState::Playing);
    }
    if let Some(request) = chosen
        && answer(
            request,
            &mut character,
            &mut awakening,
            &mut inventory,
            &mut progress,
            &mut session,
        )
    {
        next.set(AppState::Playing);
    }
}

fn stats_rows(ui: &mut egui::Ui, stats: Stats, grey: egui::Color32) {
    for stat in Stat::ALL {
        ui.horizontal(|ui| {
            ui.add_sized(
                [130.0, 20.0],
                egui::Label::new(egui::RichText::new(stat.name()).strong()),
            );
            ui.add_sized([30.0, 20.0], egui::Label::new(stats.get(stat).to_string()));
            ui.label(egui::RichText::new(stat.effect()).size(13.0).color(grey));
        });
    }
}

/// Sistema: le risposte date dalla shell valgono anche senza pannello aperto (prove automatiche).
pub fn shell_answers(
    state: Res<State<AppState>>,
    mut character: ResMut<Character>,
    mut awakening: ResMut<Awakening>,
    mut inventory: ResMut<Inventory>,
    mut progress: ResMut<crate::skills::Progress>,
    mut session: ResMut<crate::save::Session>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *state.get() == AppState::Awakening {
        return;
    }
    if let Some(request) = awakening.request.take() {
        answer(
            request,
            &mut character,
            &mut awakening,
            &mut inventory,
            &mut progress,
            &mut session,
        );
        next.set(AppState::Playing);
    }
}

/// Dove aspetta Ettore, sceso dal faro: sulla spiaggia accanto al molo, sulla sabbia libera e girato
/// verso il paese; prima stava tra la barca e il molo e sembrava incastrato (segnalazione
/// 2026-11-18_22-59-58).
pub const ETTORE_ON_PIER: (Vec2, Vec2) = (Vec2::new(13.2, 98.8), Vec2::new(-0.5, -0.85));
/// A che distanza ti saluta.
const ETTORE_HAIL: f32 = 8.0;

/// Ettore scende dal faro mentre si cerca Gaspare e si dice il nome (tappe 3 e 4 di MP-01)?
pub fn ettore_called(step: Option<usize>) -> bool {
    matches!(step, Some(2 | 3))
}

/// Sistema: Ettore scende dal faro al molo quando arrivi da Gaspare (ADR 0047), ti saluta quando
/// sei vicino e, saputo il tuo nome, torna su al faro.
#[allow(clippy::type_complexity)]
fn ettore_comes_down(
    missions: Res<crate::quests::missions::Missions>,
    mut villagers: Query<(Entity, &mut crate::villagers::Villager, &GlobalTransform)>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    mut barks: ResMut<crate::quests::Barks>,
    mut state: Local<(bool, bool)>,
) {
    let island = crate::region::current() == crate::region::Region::Isola;
    let want = island && ettore_called(missions.step_of("mp01"));
    let Some((e, mut v, t)) = villagers.iter_mut().find(|(_, v, _)| v.name() == "Ettore") else {
        return;
    };
    let near = players
        .single()
        .is_ok_and(|p| p.translation().distance(t.translation()) <= ETTORE_HAIL);
    let mut say = |key: &str| {
        if let Some(line) = crate::quests::bark_line("Ettore", key) {
            barks.say(e, "Ettore", line.to_string());
        }
    };
    if want && !state.0 {
        v.call_to(Some(ETTORE_ON_PIER));
        info!("[RISVEGLIO] Ettore scende dal faro");
    } else if !want && state.0 {
        v.call_to(None);
        if near {
            say("ritorno");
        }
        info!("[RISVEGLIO] Ettore torna al faro");
    }
    state.0 = want;
    // Il saluto, una volta sola, quando è arrivato e tu gli sei vicino.
    if want && !state.1 && near && v.at_call() {
        state.1 = true;
        say("arrivo");
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        ettore_comes_down.run_if(in_state(AppState::Playing)),
    );
    app.init_resource::<Awakening>()
        .add_systems(
            Update,
            (stage_beach, beach_prompts.after(crate::entities::interact))
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(Update, shell_answers);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_marks_the_stats_without_changing_their_sum_much() {
        let base = Class::Warrior.stats();
        assert_eq!(Age::Adult.apply(base), base);
        let young = Age::Young.apply(base);
        assert_eq!(young.get(Stat::Agility), base.get(Stat::Agility) + 1);
        let elder = Age::Elder.apply(base);
        assert_eq!(elder.get(Stat::Wisdom), base.get(Stat::Wisdom) + 1);
        for age in Age::ALL {
            assert_eq!(Age::parse(age.key()), Some(age));
        }
    }

    #[test]
    fn custom_points_are_counted() {
        let mut stats = Class::Custom.stats();
        assert_eq!(points_left(stats), i32::from(CUSTOM_POINTS));
        stats.set(Stat::Strength, 12);
        assert_eq!(points_left(stats), i32::from(CUSTOM_POINTS) - 4);
    }

    #[test]
    fn each_relic_gives_its_class_kit() {
        for (class, p) in RELICS {
            assert!(!kit_of(class).is_empty(), "{class:?}");
            assert!(!relic_of(class).0.is_empty());
            assert!(p.distance(POOL) > 1.5);
        }
        assert!(kit_of(Class::Custom).is_empty());
    }

    #[test]
    fn the_answers_walk_through_the_awakening() {
        let mut c = Character::castaway();
        let mut a = Awakening::default();
        let mut inv = Inventory::default();
        let mut p = crate::skills::Progress::default();
        let mut s = crate::save::Session::default();
        assert!(answer(
            Request::Age(Age::Elder),
            &mut c,
            &mut a,
            &mut inv,
            &mut p,
            &mut s
        ));
        assert_eq!((c.age, c.awakening), (Age::Elder, 1));
        assert!(answer(
            Request::Class(Class::Scholar),
            &mut c,
            &mut a,
            &mut inv,
            &mut p,
            &mut s
        ));
        assert_eq!(c.stats, Age::Elder.apply(Class::Scholar.stats()));
        assert_eq!(inv.qty(ItemKind::Relic(Relic::Quipu)), 1);
        assert_eq!(p.stat_points, BONUS_POINTS);
        assert!(!answer(
            Request::Name("  ".into()),
            &mut c,
            &mut a,
            &mut inv,
            &mut p,
            &mut s
        ));
        assert_eq!(c.awakening, 2);
    }
}
