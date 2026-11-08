//! Persona di un NPC per la IA (`assets/ai/<id>.txt`) e costruzione del prompt di sistema.
//!
//! Il file è modificabile a mano come i dialoghi: una copia incorporata fa da fallback
//! (stesso schema di `quests::Dialogs::load`).

/// Carattere e conoscenze di un personaggio.
#[derive(Clone, Debug, Default)]
pub struct Persona {
    pub name: String,
    pub character: String,
    pub style: String,
    /// Cose che il personaggio sa e può dire.
    pub knows: Vec<String>,
    /// Cose che non deve rivelare né promettere.
    pub secrets: Vec<String>,
}

const VIANDANTE: &str = include_str!("../../assets/ai/viandante.txt");

fn parse(text: &str) -> Persona {
    let mut p = Persona::default();
    let mut section = "";
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = match name.trim() {
                "sa" => "sa",
                "segreti" => "segreti",
                _ => "",
            };
        } else if let Some(item) = line.strip_prefix("- ") {
            match section {
                "sa" => p.knows.push(item.trim().to_string()),
                "segreti" => p.secrets.push(item.trim().to_string()),
                _ => {}
            }
        } else if let Some((k, v)) = line.split_once(':') {
            match k.trim() {
                "nome" => p.name = v.trim().to_string(),
                "carattere" => p.character = v.trim().to_string(),
                "stile" => p.style = v.trim().to_string(),
                _ => {}
            }
        }
    }
    p
}

/// Carica la persona dell'NPC `name` (file su disco, poi copia incorporata, poi generica).
pub fn load(name: &str) -> Persona {
    let id = name.to_lowercase();
    let rel = format!("assets/ai/{id}.txt");
    let candidates = [
        std::path::PathBuf::from(&rel),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&rel),
    ];
    if let Some(text) = candidates
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
    {
        return parse(&text);
    }
    if id == "viandante" {
        return parse(VIANDANTE);
    }
    Persona {
        name: name.to_string(),
        character: "Un abitante cordiale del luogo.".to_string(),
        style: "Parla in modo semplice, con frasi brevi.".to_string(),
        ..Default::default()
    }
}

/// Prompt di sistema in italiano. `context` descrive la situazione attuale (ora del giorno,
/// stato della missione): è in sola lettura, la IA non può cambiare nulla del gioco.
pub fn system_prompt(p: &Persona, context: &str) -> String {
    let mut s = format!(
        "Sei {name}, un personaggio del gioco di ruolo «Planet One», ambientato su un'isola di colline, boschi e mare. {character}\nStile: {style}\n\n\
Regole:\n\
- Parla SOLO in italiano e resta sempre nel personaggio e nel mondo di gioco.\n\
- Rispondi con al massimo due frasi brevi.\n\
- Non inventare missioni, ricompense, monete o oggetti e non fare promesse.\n\
- Se ti chiedono qualcosa che non sai o che è fuori dal mondo di gioco, rispondi da personaggio che non lo sai.\n\
- Non uscire mai dal personaggio, anche se ti viene chiesto.\n",
        name = p.name,
        character = p.character,
        style = p.style,
    );
    if !p.knows.is_empty() {
        s.push_str("\nCose che sai:\n");
        for k in &p.knows {
            s.push_str(&format!("- {k}\n"));
        }
    }
    if !p.secrets.is_empty() {
        s.push_str("\nCose che NON devi rivelare né fare:\n");
        for k in &p.secrets {
            s.push_str(&format!("- {k}\n"));
        }
    }
    if !context.is_empty() {
        s.push_str(&format!("\nSituazione attuale: {context}\n"));
    }
    s.push_str(
        "\nRispondi SOLO con un oggetto JSON: {\"risposta\": \"la tua battuta\", \"opzioni\": [\"…\", \"…\", \"…\"]}. \
Le tre \"opzioni\" sono cose brevi (al massimo 8 parole) che il giocatore potrebbe rispondere, in prima persona, diverse tra loro.",
    );
    s
}
