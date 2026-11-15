//! Voci dei personaggi: clip OGG in `assets/audio/voci/<personaggio>/<ID>.ogg`.
//!
//! La mappa battuta → clip sta in `assets/quests/<quest>/voci.txt` (generata da
//! `scripts/export_voci.py`). Una battuta può avere più clip riprodotte in
//! sequenza (es. direzione + distanza del ladro). Una sola voce alla volta:
//! una nuova battuta interrompe la precedente.

use crate::audio::AudioSettings;
use crate::entities::DialogState;
use bevy::audio::Volume;
use bevy::prelude::*;
use std::collections::VecDeque;

/// Guadagno delle voci rispetto allo slider. Le clip arrivano già quasi al massimo
/// (picchi fra −3 e −0,8 dBFS): con 1,4 a volume pieno superavano 0 dBFS e saturavano.
/// Sopra la musica le tiene il ducking (`audio::DUCK_LEVEL`), non il guadagno.
/// Vale anche per la voce dell'IA (`ai::tts`).
pub const VOICE_GAIN: f32 = 0.85;
/// Altezza della bocca sopra l'origine di un personaggio (m).
const SPEAKER_MOUTH: f32 = 0.7;

/// Origine della voce: la voce di un dialogo si ferma alla chiusura del
/// dialogo, quella di un fumetto (congedo, battuta del ladro) no.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoiceKind {
    Dialog,
    Bark,
}

/// Nome dei clip delle prime battute di un abitante: `abitanti/<nome>-<n>` (`scripts/gen_voci_abitanti.py`).
pub fn greeting_clips(name: &str, count: usize) -> Vec<Vec<String>> {
    let slug = name.to_lowercase().replace(' ', "-");
    (1..=count)
        .map(|n| vec![format!("abitanti/{slug}-{n}")])
        .collect()
}

/// Un saluto di commiato, maschile o femminile secondo la voce Piper del personaggio.
pub fn farewell_clip(voice: &str, salt: u32) -> String {
    let female = ["paola", "serena", "lisa"]
        .iter()
        .any(|n| voice.contains(n));
    format!(
        "congedo/{}-{}",
        if female { "f" } else { "m" },
        salt % FAREWELLS + 1
    )
}

/// FNV-1a a 64 bit: lo stesso hash di `scripts/gen_voci_prologo.py`.
pub fn fnv1a(text: &str) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01B3);
    }
    h
}

/// Clip della battuta `line` (testo del file, con `{nome}`) detta da `speaker`:
/// `prologo/<abitante>-<hash>` (ADR 0050). Vuota se la clip non è stata generata.
pub fn line_clip(speaker: &str, line: &str) -> Vec<String> {
    let name = format!(
        "prologo/{}-{:016x}",
        speaker.to_lowercase().replace(' ', "-"),
        fnv1a(line.trim())
    );
    if clip_exists(&name) {
        vec![name]
    } else {
        Vec::new()
    }
}

/// La clip c'è fra gli asset? (radice di Bevy, accanto all'eseguibile o nei sorgenti)
fn clip_exists(name: &str) -> bool {
    let rel = format!("assets/audio/voci/{name}.ogg");
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    if let Some(r) = std::env::var_os("BEVY_ASSET_ROOT") {
        roots.push(r.into());
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        roots.push(dir.to_path_buf());
    }
    roots.push(env!("CARGO_MANIFEST_DIR").into());
    roots.push(".".into());
    roots.iter().any(|r| r.join(&rel).is_file())
}

/// Quanti congedi diversi ci sono per sesso.
const FAREWELLS: u32 = 6;

/// Coda delle clip da riprodurre.
#[derive(Resource, Default)]
pub struct VoiceQueue {
    pending: VecDeque<String>,
    kind: Option<VoiceKind>,
    interrupt: bool,
    /// Chi parla: la voce esce dalla sua testa (audio posizionale); `None` = da nessuna parte.
    speaker: Option<Entity>,
}

impl VoiceQueue {
    /// Riproduce le clip in sequenza, interrompendo la voce in corso.
    pub fn play(&mut self, clips: Vec<String>, kind: VoiceKind) {
        self.play_from(clips, kind, None);
    }

    /// Come `play`, con la voce che esce da `speaker` (e lo segue se si muove).
    pub fn play_from(&mut self, clips: Vec<String>, kind: VoiceKind, speaker: Option<Entity>) {
        self.pending = clips.into();
        self.kind = Some(kind);
        self.interrupt = true;
        self.speaker = speaker;
    }

    /// C'è una voce in coda (o appena richiesta)?
    pub fn busy(&self) -> bool {
        !self.pending.is_empty() || self.interrupt
    }

    /// Ferma la voce se è del tipo indicato.
    pub fn stop(&mut self, kind: VoiceKind) {
        if self.kind == Some(kind) {
            self.pending.clear();
            self.kind = None;
            self.interrupt = true;
        }
    }
}

/// Clip in riproduzione.
#[derive(Component)]
pub struct VoiceClip;

/// Sistema: avvia la clip successiva quando la precedente è finita
/// (`PlaybackSettings::DESPAWN` elimina l'entità a fine clip).
pub fn drive_voice(
    mut commands: Commands,
    mut queue: ResMut<VoiceQueue>,
    settings: Res<AudioSettings>,
    asset_server: Res<AssetServer>,
    playing: Query<Entity, With<VoiceClip>>,
    speakers: Query<(), With<GlobalTransform>>,
) {
    if queue.interrupt {
        queue.interrupt = false;
        for e in &playing {
            commands.entity(e).despawn();
        }
        return; // la clip nuova parte al frame dopo, a entità vecchie sparite
    }
    if !playing.is_empty() {
        return;
    }
    if let Some(clip) = queue.pending.pop_front() {
        let player = AudioPlayer::new(asset_server.load(format!("audio/voci/{clip}.ogg")));
        let volume = settings.voice_volume * VOICE_GAIN;
        match queue.speaker.filter(|e| speakers.contains(*e)) {
            // Figlia di chi parla, all'altezza della bocca: la segue mentre cammina.
            Some(speaker) => {
                commands.spawn((
                    VoiceClip,
                    player,
                    crate::audio::spatial_once(volume),
                    Transform::from_xyz(0.0, SPEAKER_MOUTH, 0.0),
                    ChildOf(speaker),
                ));
            }
            None => {
                commands.spawn((
                    VoiceClip,
                    player,
                    PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
                ));
            }
        }
        info!("[VOICE] clip avviata: {clip}");
    } else {
        queue.kind = None;
    }
}

/// Sistema: a ogni riga mostrata dal dialogo parte la sua voce; alla chiusura
/// del dialogo si ferma la voce del dialogo.
pub fn dialog_voice(
    dialog: Res<DialogState>,
    mut queue: ResMut<VoiceQueue>,
    mut last: Local<(bool, usize)>,
) {
    if dialog.open {
        if (!last.0 || dialog.index != last.1)
            && !dialog.choosing
            && let Some(clips) = dialog.voices.get(dialog.index)
            && !clips.is_empty()
        {
            queue.play_from(clips.clone(), VoiceKind::Dialog, dialog.speaker_entity);
        }
    } else if last.0 {
        queue.stop(VoiceKind::Dialog);
    }
    *last = (dialog.open, dialog.index);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_clips_follow_the_persona_file_names() {
        assert_eq!(
            greeting_clips("Don Anselmo", 2)[1],
            vec!["abitanti/don-anselmo-2"]
        );
    }

    #[test]
    fn mission_lines_find_their_generated_clip() {
        // Stesso hash dello script Python (FNV-1a 64).
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
        assert!(line_clip("Nessuno", "battuta che non esiste").is_empty());
    }

    #[test]
    fn every_companion_line_has_its_voice() {
        for (name, key, text) in crate::companions::lines() {
            assert!(
                !line_clip(name, text).is_empty(),
                "{name} «{key}»: voce mancante (scripts/gen_voci_prologo.py)"
            );
        }
    }

    #[test]
    fn every_bark_line_has_its_voice() {
        for (name, key, text) in crate::awakening::bark_lines() {
            assert!(
                !line_clip(name, text).is_empty(),
                "{name} «{key}»: voce mancante (scripts/gen_voci_prologo.py)"
            );
        }
    }

    #[test]
    fn farewell_clip_matches_voice_and_wraps() {
        assert_eq!(farewell_clip("it_IT-paola-medium", 0), "congedo/f-1");
        assert_eq!(farewell_clip("it_IT-riccardo-x_low", 7), "congedo/m-2");
    }

    #[test]
    fn every_clip_exists_on_disk() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/voci");
        for sex in ["m", "f"] {
            for n in 1..=FAREWELLS {
                assert!(
                    root.join(format!("congedo/{sex}-{n}.ogg")).is_file(),
                    "{sex}-{n}"
                );
            }
        }
    }
}
