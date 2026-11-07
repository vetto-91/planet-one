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

/// Origine della voce: la voce di un dialogo si ferma alla chiusura del
/// dialogo, quella di un fumetto (congedo, battuta del ladro) no.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoiceKind {
    Dialog,
    Bark,
}

/// Coda delle clip da riprodurre.
#[derive(Resource, Default)]
pub struct VoiceQueue {
    pending: VecDeque<String>,
    kind: Option<VoiceKind>,
    interrupt: bool,
}

impl VoiceQueue {
    /// Riproduce le clip in sequenza, interrompendo la voce in corso.
    pub fn play(&mut self, clips: Vec<String>, kind: VoiceKind) {
        self.pending = clips.into();
        self.kind = Some(kind);
        self.interrupt = true;
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
        commands.spawn((
            VoiceClip,
            AudioPlayer::new(asset_server.load(format!("audio/voci/{clip}.ogg"))),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(settings.voice_volume)),
        ));
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
            queue.play(clips.clone(), VoiceKind::Dialog);
        }
    } else if last.0 {
        queue.stop(VoiceKind::Dialog);
    }
    *last = (dialog.open, dialog.index);
}
