//! IA locale per espandere i dialoghi (Ollama, solo sul PC dell'utente).
//!
//! I dialoghi e le quest scritti hanno sempre la precedenza: la IA interviene solo dopo,
//! con l'opzione "Parliamo d'altro…" (stato `AppState::Chat`), e non legge né modifica
//! lo stato del gioco (quest, inventario, monete). Tutto gira in locale: vedi `ollama.rs`.
//! Una thread worker parla con Ollama; il gioco non si blocca mai. Spenta dal menu o con
//! il servizio assente, il gioco procede esattamente come senza IA.

pub mod ollama;
pub mod persona;
pub mod tts;
pub mod ui;

use crate::AppState;
use crate::audio::AudioSettings;
use crate::daynight::GameClock;
use crate::quests::denaro_rubato::QuestState;
use bevy::prelude::*;
use ollama::{Probe, Reply, Turn};
use std::sync::{Mutex, mpsc};

/// Ogni quanti secondi si ricontrolla il servizio.
const PROBE_EVERY: f32 = 15.0;
/// Quante battute passate si mandano al modello.
const MAX_TURNS: usize = 10;

/// Stato del servizio IA.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum AiStatus {
    /// Spenta dal menu o non ancora controllata.
    #[default]
    Off,
    Ready,
    /// Servizio raggiungibile ma modello non scaricato.
    NoModel,
    Unreachable,
    /// `ai_host` non è locale: rifiutato.
    BadHost,
}

impl AiStatus {
    /// Testo per il menu.
    pub fn label(&self) -> &'static str {
        match self {
            AiStatus::Off => "spenta",
            AiStatus::Ready => "pronta (locale)",
            AiStatus::NoModel => "modello non installato: ollama pull <modello>",
            AiStatus::Unreachable => "Ollama non trovato su questo PC",
            AiStatus::BadHost => "host non locale: rifiutato",
        }
    }
}

enum Request {
    Probe {
        host: String,
        model: String,
    },
    Chat {
        id: u64,
        host: String,
        model: String,
        system: String,
        turns: Vec<Turn>,
    },
    Unload {
        host: String,
        model: String,
    },
}

enum Event {
    Probe(Probe),
    Models(Vec<String>),
    Reply(u64, Result<Reply, String>),
}

/// Stato globale della IA e canali verso la thread worker.
#[derive(Resource)]
pub struct AiState {
    pub status: AiStatus,
    /// Modelli installati nell'Ollama locale (aggiornati a ogni controllo).
    pub models: Vec<String>,
    tx: mpsc::Sender<Request>,
    rx: Mutex<mpsc::Receiver<Event>>,
    probe_timer: f32,
    probing: bool,
    was_enabled: bool,
}

impl Default for AiState {
    fn default() -> Self {
        let (tx, worker_rx) = mpsc::channel::<Request>();
        let (worker_tx, rx) = mpsc::channel::<Event>();
        let mock = std::env::var("PLANET_ONE_AI_MOCK").is_ok_and(|v| v != "0");
        if mock {
            info!("[AI] modalità MOCK: nessuna connessione, risposte fisse");
        }
        std::thread::spawn(move || worker(worker_rx, worker_tx, mock));
        Self {
            status: AiStatus::Off,
            models: Vec::new(),
            tx,
            rx: Mutex::new(rx),
            probe_timer: PROBE_EVERY,
            probing: false,
            was_enabled: false,
        }
    }
}

fn worker(rx: mpsc::Receiver<Request>, tx: mpsc::Sender<Event>, mock: bool) {
    for req in rx {
        match req {
            Request::Probe { host, model } => {
                let r = if mock {
                    Probe::Ready
                } else {
                    ollama::probe(&host, &model)
                };
                let _ = tx.send(Event::Probe(r));
                let models = if mock {
                    vec!["mock".to_string()]
                } else {
                    ollama::list_models(&host)
                };
                let _ = tx.send(Event::Models(models));
            }
            Request::Chat {
                id,
                host,
                model,
                system,
                turns,
            } => {
                let r = if mock {
                    std::thread::sleep(std::time::Duration::from_millis(700));
                    Ok(mock_reply(turns.len()))
                } else {
                    ollama::chat(&host, &model, &system, &turns)
                };
                let _ = tx.send(Event::Reply(id, r));
            }
            Request::Unload { host, model } => {
                if !mock {
                    ollama::unload(&host, &model);
                }
            }
        }
    }
}

fn mock_reply(n: usize) -> Reply {
    let text = match n % 3 {
        0 => {
            "Ah, un viaggiatore! Siediti vicino alla stufa, c'è sempre posto per una buona storia."
        }
        1 => "Le colline sono belle, ma di notte conviene stare vicino a una luce.",
        _ => "Questa è una risposta di prova: la IA vera non è collegata.",
    };
    Reply {
        text: text.to_string(),
        options: vec![
            "Raccontami delle colline.".into(),
            "Cosa vendi?".into(),
            "Devo andare.".into(),
        ],
    }
}

/// Una battuta della conversazione (per la UI).
#[derive(Clone)]
pub struct ChatTurn {
    pub player: bool,
    pub text: String,
}

/// Azione richiesta dalla UI della chat.
#[derive(Clone)]
pub enum ChatAction {
    Send(String),
    Close,
}

/// Conversazione IA in corso (stato `AppState::Chat`).
#[derive(Resource, Default)]
pub struct AiChat {
    pub npc_name: String,
    pub history: Vec<ChatTurn>,
    pub options: Vec<String>,
    pub input: String,
    pub thinking: bool,
    pub error: Option<String>,
    pub action: Option<ChatAction>,
    pub focus_input: bool,
    /// Voce e velocità del personaggio (dalla persona).
    voice: String,
    speed: f32,
    system: String,
    req: u64,
}

impl AiChat {
    /// Prepara una nuova conversazione con l'NPC `name`.
    pub fn begin(&mut self, name: &str) {
        *self = AiChat {
            npc_name: name.to_string(),
            focus_input: true,
            ..Default::default()
        };
    }
}

fn day_phase(hour: f32) -> &'static str {
    match hour {
        h if (5.0..12.0).contains(&h) => "mattina",
        h if (12.0..18.0).contains(&h) => "pomeriggio",
        h if (18.0..22.0).contains(&h) => "sera",
        _ => "notte",
    }
}

/// Contesto in sola lettura per il prompt (ora e riassunto della missione, senza spoiler).
fn context_line(quest: &QuestState, clock: &GameClock) -> String {
    let mut s = format!("è {} (ore {})", day_phase(clock.hour), clock.label());
    if quest.final_done {
        s.push_str(". Il viaggiatore ti ha riportato le monete rubate: gli sei riconoscente");
    } else if quest.accepted {
        s.push_str(
            ". Hai chiesto al viaggiatore di recuperare le monete che un ladro ti ha rubato",
        );
    }
    s.push('.');
    s
}

impl AiState {
    /// Anticipa il prossimo controllo (dopo aver cambiato modello).
    pub fn refresh(&mut self) {
        self.probe_timer = 0.0;
    }
}

/// Sistema: controlla il servizio a intervalli e scarica il modello quando si spegne la IA.
pub fn ai_probe(time: Res<Time>, settings: Res<AudioSettings>, mut ai: ResMut<AiState>) {
    if !settings.ai_enabled {
        if ai.was_enabled {
            ai.was_enabled = false;
            let _ = ai.tx.send(Request::Unload {
                host: settings.ai_host.clone(),
                model: settings.ai_model.clone(),
            });
        }
        ai.status = AiStatus::Off;
        ai.probe_timer = 0.0;
        return;
    }
    ai.was_enabled = true;
    ai.probe_timer -= time.delta_secs();
    if ai.probe_timer <= 0.0 && !ai.probing {
        ai.probe_timer = PROBE_EVERY;
        ai.probing = true;
        let _ = ai.tx.send(Request::Probe {
            host: settings.ai_host.clone(),
            model: settings.ai_model.clone(),
        });
    }
}

/// Sistema: raccoglie le risposte della thread worker.
pub fn ai_pump(
    settings: Res<AudioSettings>,
    mut ai: ResMut<AiState>,
    mut chat: ResMut<AiChat>,
    mut tts: ResMut<tts::TtsState>,
) {
    let events: Vec<Event> = {
        let Ok(rx) = ai.rx.lock() else {
            return;
        };
        rx.try_iter().collect()
    };
    for ev in events {
        match ev {
            Event::Probe(p) => {
                ai.probing = false;
                ai.status = if settings.ai_enabled {
                    match p {
                        Probe::Ready => AiStatus::Ready,
                        Probe::NoModel => AiStatus::NoModel,
                        Probe::Unreachable => AiStatus::Unreachable,
                        Probe::BadHost => AiStatus::BadHost,
                    }
                } else {
                    AiStatus::Off
                };
            }
            Event::Models(list) => ai.models = list,
            Event::Reply(id, result) => {
                if id != chat.req {
                    continue;
                }
                chat.thinking = false;
                match result {
                    Ok(r) => {
                        // La risposta si mostra subito e si legge ad alta voce (Piper locale).
                        let voice = if settings.tts_voice.is_empty() {
                            chat.voice.clone()
                        } else {
                            settings.tts_voice.clone()
                        };
                        tts.speak(&r.text, &voice, chat.speed);
                        chat.history.push(ChatTurn {
                            player: false,
                            text: r.text,
                        });
                        chat.options = r.options;
                        chat.error = None;
                    }
                    Err(e) => {
                        warn!("[AI] errore: {e}");
                        chat.error =
                            Some("Il Viandante resta in silenzio… (la IA non risponde)".into());
                        if chat.options.is_empty() {
                            chat.options = vec!["Arrivederci.".into()];
                        }
                    }
                }
            }
        }
    }
}

/// Invia al modello la conversazione attuale (la battuta del giocatore è già in `history`).
fn request_reply(chat: &mut AiChat, ai: &AiState, settings: &AudioSettings) {
    let mut turns: Vec<Turn> = chat
        .history
        .iter()
        .rev()
        .take(MAX_TURNS)
        .rev()
        .map(|t| (t.player, t.text.clone()))
        .collect();
    if turns.is_empty() || !turns[0].0 {
        // Il modello vuole iniziare con una battuta dell'utente.
        turns.insert(
            0,
            (
                true,
                "(Il giocatore si avvicina e ti rivolge la parola. Salutalo brevemente e chiedigli di cosa vuole parlare.)"
                    .to_string(),
            ),
        );
    }
    chat.req += 1;
    chat.thinking = true;
    chat.options.clear();
    let _ = ai.tx.send(Request::Chat {
        id: chat.req,
        host: settings.ai_host.clone(),
        model: settings.ai_model.clone(),
        system: chat.system.clone(),
        turns,
    });
}

/// OnEnter(Chat): costruisce il prompt e chiede il saluto iniziale.
pub fn ai_enter_chat(
    mut chat: ResMut<AiChat>,
    ai: Res<AiState>,
    settings: Res<AudioSettings>,
    quest: Res<QuestState>,
    clock: Res<GameClock>,
) {
    let p = persona::load(&chat.npc_name);
    chat.voice = p.voice.clone();
    chat.speed = p.speed;
    chat.system = persona::system_prompt(&p, &context_line(&quest, &clock));
    request_reply(&mut chat, &ai, &settings);
    info!("[AI] chat con {} avviata", chat.npc_name);
}

/// Sistema (stato Chat): esegue le azioni scelte nella UI.
pub fn ai_handle_action(
    mut chat: ResMut<AiChat>,
    ai: Res<AiState>,
    settings: Res<AudioSettings>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(action) = chat.action.take() else {
        return;
    };
    match action {
        ChatAction::Close => next.set(AppState::Playing),
        ChatAction::Send(text) => {
            let text = text.trim().to_string();
            if text.is_empty() || chat.thinking {
                return;
            }
            chat.history.push(ChatTurn { player: true, text });
            chat.input.clear();
            request_reply(&mut chat, &ai, &settings);
        }
    }
}

/// OnExit(Chat): chiude la conversazione.
pub fn ai_exit_chat(mut chat: ResMut<AiChat>, mut tts: ResMut<tts::TtsState>) {
    tts.stop();
    chat.action = None;
    chat.thinking = false;
    chat.req += 1; // scarta eventuali risposte in arrivo
    info!("[AI] chat chiusa");
}

pub fn plugin(app: &mut App) {
    app.init_resource::<AiState>()
        .init_resource::<AiChat>()
        .init_resource::<tts::TtsState>()
        .add_systems(Update, (ai_probe, ai_pump, tts::tts_scan, tts::tts_pump))
        .add_systems(OnEnter(AppState::Chat), ai_enter_chat)
        .add_systems(OnExit(AppState::Chat), ai_exit_chat)
        .add_systems(Update, ai_handle_action.run_if(in_state(AppState::Chat)));
}
