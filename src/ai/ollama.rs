//! Client minimo di Ollama, **solo in locale**.
//!
//! Vincolo del progetto: modelli e servizi girano esclusivamente sul PC dell'utente.
//! Per questo: si accettano solo host di loopback e `ureq` è compilato senza TLS
//! (impossibile parlare con servizi esterni via HTTPS).

use serde_json::{Value, json};
use std::time::Duration;

/// Risposta strutturata della IA.
#[derive(Clone, Debug, Default)]
pub struct Reply {
    pub text: String,
    pub options: Vec<String>,
}

/// Esito del controllo del servizio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    Ready,
    /// Servizio raggiungibile ma modello non scaricato.
    NoModel,
    Unreachable,
    /// Host non locale: rifiutato.
    BadHost,
}

/// Normalizza l'host e lo accetta solo se è di loopback (`127.0.0.1`, `localhost`, `::1`).
pub fn local_base(host: &str) -> Option<String> {
    let h = host.trim().trim_end_matches('/');
    let rest = h.strip_prefix("http://")?; // niente https: nessun TLS
    if rest.contains('@') || rest.contains('/') {
        return None;
    }
    let name = if let Some(r) = rest.strip_prefix('[') {
        r.split(']').next()?.to_string()
    } else {
        rest.split(':').next()?.to_string()
    };
    matches!(name.as_str(), "127.0.0.1" | "localhost" | "::1").then(|| format!("http://{rest}"))
}

fn agent(secs: u64) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(2))
        .timeout(Duration::from_secs(secs))
        .build()
}

/// Il server risponde? (avvio dell'Ollama del gioco, `server.rs`)
pub fn is_up(base: &str) -> bool {
    agent(2).get(&format!("{base}/api/version")).call().is_ok()
}

/// Controlla che Ollama risponda e che il modello sia presente.
pub fn probe(host: &str, model: &str) -> Probe {
    let Some(base) = local_base(host) else {
        return Probe::BadHost;
    };
    let Ok(resp) = agent(3).get(&format!("{base}/api/tags")).call() else {
        return Probe::Unreachable;
    };
    let Ok(text) = resp.into_string() else {
        return Probe::Unreachable;
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return Probe::Unreachable;
    };
    let wanted = model.trim();
    let found = v["models"].as_array().is_some_and(|list| {
        list.iter().any(|m| {
            let n = m["name"].as_str().unwrap_or("");
            n == wanted || n.strip_suffix(":latest") == Some(wanted)
        })
    });
    if found { Probe::Ready } else { Probe::NoModel }
}

/// Nomi dei modelli installati nell'Ollama locale (vuoto se non risponde).
pub fn list_models(host: &str) -> Vec<String> {
    let Some(base) = local_base(host) else {
        return Vec::new();
    };
    agent(3)
        .get(&format!("{base}/api/tags"))
        .call()
        .ok()
        .and_then(|r| r.into_string().ok())
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|v| {
            v["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|m| m["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Una battuta del giocatore (`true`) o dell'NPC (`false`).
pub type Turn = (bool, String);

/// Chiede una risposta al modello (bloccante: va chiamata dalla thread worker).
pub fn chat(host: &str, model: &str, system: &str, turns: &[Turn]) -> Result<Reply, String> {
    let base = local_base(host).ok_or("host non locale")?;
    let mut messages = vec![json!({"role": "system", "content": system})];
    for (player, text) in turns {
        messages.push(json!({
            "role": if *player { "user" } else { "assistant" },
            "content": text,
        }));
    }
    let body = json!({
        "model": model,
        "stream": false,
        "think": false,
        "keep_alive": KEEP_ALIVE,
        "format": {
            "type": "object",
            "properties": {
                "risposta": {"type": "string"},
                "opzioni": {"type": "array", "items": {"type": "string"}, "minItems": 3, "maxItems": 3}
            },
            "required": ["risposta", "opzioni"]
        },
        "options": {"temperature": 0.7, "num_predict": 220, "num_ctx": CHAT_CTX},
        "messages": messages,
    });
    // Col gioco che occupa la scheda video la prima risposta (modello da caricare, persona lunga)
    // può superare i 40 s: prima la chat finiva in «la IA non risponde».
    let resp = agent(150)
        .post(&format!("{base}/api/chat"))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("richiesta fallita: {e}"))?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let content = v["message"]["content"].as_str().ok_or("risposta vuota")?;
    parse_reply(content)
}

/// Estrae e ripulisce la risposta JSON del modello.
pub fn parse_reply(content: &str) -> Result<Reply, String> {
    let json_text = match (content.find('{'), content.rfind('}')) {
        (Some(a), Some(b)) if b > a => &content[a..=b],
        _ => return Err("risposta non JSON".into()),
    };
    let v: Value = serde_json::from_str(json_text).map_err(|e| e.to_string())?;
    let text = clean(v["risposta"].as_str().ok_or("manca la risposta")?, 240);
    if text.is_empty() {
        return Err("risposta vuota".into());
    }
    let mut options: Vec<String> = v["opzioni"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|o| o.as_str())
                .map(|o| clean(o, 60))
                .filter(|o| !o.is_empty())
                .take(3)
                .collect()
        })
        .unwrap_or_default();
    if options.len() < 2 {
        options = vec![
            "Capisco.".into(),
            "Dimmi di più.".into(),
            "Devo andare.".into(),
        ];
    }
    Ok(Reply { text, options })
}

/// Toglie markdown e spazi e tronca a `max` caratteri (a fine frase se possibile).
fn clean(s: &str, max: usize) -> String {
    let s: String = s
        .chars()
        .filter(|c| !matches!(c, '*' | '#' | '`' | '_'))
        .collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
        return s;
    }
    let cut: String = s.chars().take(max).collect();
    match cut.rfind(['.', '!', '?']) {
        Some(i) if i > max / 2 => cut[..=i].to_string(),
        _ => format!("{}…", cut.trim_end()),
    }
}

/// Carica il modello in memoria prima della prima chat (la prima risposta altrimenti aspetta il
/// caricamento) e dice quanta parte sta sulla scheda video: (totale, in VRAM, secondi). I valori di
/// `/api/ps` valgono come rapporto, non come GB (vedi `placement_note`).
pub fn preload(host: &str, model: &str) -> Result<(f32, f32, f32), String> {
    let base = local_base(host).ok_or("host non locale")?;
    let start = std::time::Instant::now();
    agent(180)
        .post(&format!("{base}/api/generate"))
        .set("Content-Type", "application/json")
        // Stesso contesto della chat: con quello predefinito di Ollama (che può essere enorme) il
        // modello occupa molta più memoria, finisce in parte sul processore e la chat lo ricarica.
        .send_string(
            &json!({"model": model, "keep_alive": KEEP_ALIVE, "options": {"num_ctx": CHAT_CTX}})
                .to_string(),
        )
        .map_err(|e| format!("caricamento fallito: {e}"))?;
    let secs = start.elapsed().as_secs_f32();
    let (size, vram) = placement(host, model)?;
    Ok((size, vram, secs))
}

/// Quanto occupa il modello in memoria e quanta parte sta sulla scheda video (GB).
pub fn placement(host: &str, model: &str) -> Result<(f32, f32), String> {
    let base = local_base(host).ok_or("host non locale")?;
    let text = agent(5)
        .get(&format!("{base}/api/ps"))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let entry = v["models"]
        .as_array()
        .and_then(|l| {
            l.iter().find(|m| {
                let n = m["name"].as_str().unwrap_or("");
                n == model || n.strip_suffix(":latest") == Some(model)
            })
        })
        .ok_or("modello non in memoria")?;
    let gb = |k: &str| entry[k].as_f64().unwrap_or(0.0) as f32 / 1e9;
    Ok((gb("size"), gb("size_vram")))
}

/// Contesto della chat (token): basta per la persona e qualche scambio.
const CHAT_CTX: u32 = 3072;

/// Quanto resta in memoria il modello dopo l'ultima richiesta.
const KEEP_ALIVE: &str = "10m";

/// Scarica il modello dalla memoria (VRAM) subito.
pub fn unload(host: &str, model: &str) {
    let Some(base) = local_base(host) else {
        return;
    };
    let _ = agent(5)
        .post(&format!("{base}/api/generate"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"model": model, "keep_alive": 0}).to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loopback_hosts() {
        assert!(local_base("http://127.0.0.1:11434").is_some());
        assert!(local_base("http://localhost:11434/").is_some());
        assert!(local_base("http://[::1]:11434").is_some());
        assert!(local_base("http://example.com").is_none());
        assert!(local_base("https://127.0.0.1:11434").is_none());
        assert!(local_base("http://127.0.0.1@evil.com").is_none());
        assert!(local_base("http://192.168.1.5:11434").is_none());
        assert!(local_base("http://localhost.evil.com").is_none());
    }

    #[test]
    fn parses_and_cleans_replies() {
        let r = parse_reply(
            "{\"risposta\": \"**Ciao**, viaggiatore!\", \"opzioni\": [\"A\", \"B\", \"C\", \"D\"]}",
        )
        .unwrap();
        assert_eq!(r.text, "Ciao, viaggiatore!");
        assert_eq!(r.options.len(), 3);
        assert!(parse_reply("niente json").is_err());
    }
}
