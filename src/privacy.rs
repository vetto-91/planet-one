//! Privacy delle segnalazioni F12: prima di scrivere log, stato e prestazioni nella cartella della
//! segnalazione si tolgono i dati personali. La cartella resta sul PC; è il giocatore a decidere se
//! mandarla.
//!
//! Cosa si toglie:
//! - le cartelle dell'utente (`HOME`, `USERPROFILE`, `LOCALAPPDATA`, `APPDATA`, `TEMP`) e, in generale,
//!   il nome nei percorsi `C:\Users\<nome>\`, `/home/<nome>/`, `/Users/<nome>/`;
//! - il nome del personaggio, che il giocatore sceglie e spesso è il suo (`<giocatore>`);
//! - il testo della chat con la IA e le trascrizioni del microfono (`omit_chat`).
//!
//! Il gioco non scrive mai nel log il nome del computer né indirizzi di rete.

use serde_json::Value;

/// Al posto del nome dell'utente nei percorsi.
const USER: &str = "<utente>";
/// Al posto del nome del personaggio.
const PLAYER: &str = "<giocatore>";

/// Le sostituzioni da fare, dalla più lunga alla più corta.
pub struct Redactor {
    paths: Vec<(String, String)>,
    player: Option<String>,
}

impl Redactor {
    /// Dalle variabili d'ambiente di questa macchina e dal nome del personaggio.
    pub fn current(player: Option<&str>) -> Self {
        let mut pairs = Vec::new();
        for (var, label) in [
            ("LOCALAPPDATA", "%LOCALAPPDATA%"),
            ("APPDATA", "%APPDATA%"),
            ("TEMP", "%TEMP%"),
            ("TMP", "%TEMP%"),
            ("USERPROFILE", "%USERPROFILE%"),
            ("XDG_DATA_HOME", "$XDG_DATA_HOME"),
            ("XDG_CONFIG_HOME", "$XDG_CONFIG_HOME"),
            ("HOME", "~"),
        ] {
            if let Some(v) = std::env::var_os(var) {
                pairs.push((v.to_string_lossy().into_owned(), label.to_string()));
            }
        }
        Self::new(pairs, player)
    }

    pub fn new(pairs: Vec<(String, String)>, player: Option<&str>) -> Self {
        let mut paths: Vec<(String, String)> = pairs
            .into_iter()
            // Una cartella troppo corta (es. "/") toglierebbe pezzi di testo qualunque.
            .filter(|(p, _)| p.trim_end_matches(['/', '\\']).len() >= 4)
            .map(|(p, l)| (p.trim_end_matches(['/', '\\']).to_string(), l))
            .collect();
        paths.sort_by_key(|(p, _)| std::cmp::Reverse(p.len()));
        paths.dedup_by(|a, b| a.0 == b.0);
        let player = player
            .map(str::trim)
            .filter(|n| n.chars().count() >= 2)
            .map(str::to_string);
        Self { paths, player }
    }

    /// Il testo senza dati personali.
    pub fn redact(&self, text: &str) -> String {
        let mut out = text.to_string();
        for (path, label) in &self.paths {
            out = replace_path(&out, path, label);
        }
        out = mask_user_dirs(&out);
        if let Some(name) = &self.player {
            out = replace_word(&out, name, PLAYER);
        }
        out
    }

    /// Lo stato JSON senza dati personali, stringa per stringa (le barre di Windows nel JSON scritto
    /// sono raddoppiate: si lavora sui valori, non sul testo).
    pub fn redact_json(&self, value: &mut Value) {
        match value {
            Value::String(s) => *s = self.redact(s),
            Value::Array(list) => list.iter_mut().for_each(|v| self.redact_json(v)),
            Value::Object(map) => map.values_mut().for_each(|v| self.redact_json(v)),
            _ => {}
        }
    }
}

/// La chat con la IA nello stato: resta quanti turni e quanto era lunga la risposta, non il testo.
pub fn omit_chat(state: &mut Value) {
    if let Some(answer) = state.pointer_mut("/chat/risposta")
        && let Some(text) = answer.as_str()
    {
        *answer = Value::String(format!("<omessa, {} caratteri>", text.chars().count()));
    }
}

/// Sostituisce una cartella, scritta con `\` o con `/`, senza badare a maiuscole e minuscole
/// (Windows non le distingue).
fn replace_path(text: &str, path: &str, label: &str) -> String {
    let mut out = text.to_string();
    for variant in [
        path.to_string(),
        path.replace('\\', "/"),
        path.replace('/', "\\"),
    ] {
        out = replace_ci(&out, &variant, label);
    }
    out
}

fn replace_ci(text: &str, needle: &str, with: &str) -> String {
    if needle.is_empty() {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    let low_needle = needle.to_lowercase();
    // Le minuscole possono cambiare la lunghezza in byte: in quel caso si cerca alla lettera.
    if lower.len() != text.len() || low_needle.len() != needle.len() {
        return text.replace(needle, with);
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (i, _) in lower.match_indices(&low_needle) {
        if i < last {
            continue;
        }
        out.push_str(&text[last..i]);
        out.push_str(with);
        last = i + needle.len();
    }
    out.push_str(&text[last..]);
    out
}

/// `C:\Users\mario\…`, `/home/mario/…`, `/Users/mario/…` → il nome diventa `<utente>`.
fn mask_user_dirs(text: &str) -> String {
    let mut out = text.to_string();
    for (prefix, sep) in [("\\users\\", '\\'), ("/users/", '/'), ("/home/", '/')] {
        out = mask_after(&out, prefix, sep);
    }
    out
}

fn mask_after(text: &str, prefix: &str, sep: char) -> String {
    let lower = text.to_lowercase();
    if lower.len() != text.len() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (i, _) in lower.match_indices(prefix) {
        if i < last {
            continue;
        }
        let start = i + prefix.len();
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c == sep || c == '"' || c.is_whitespace())
            .unwrap_or(rest.len());
        let name = &rest[..end];
        out.push_str(&text[last..start]);
        if name.is_empty() || name == USER || name.starts_with('%') || name.starts_with('$') {
            out.push_str(name);
        } else {
            out.push_str(USER);
        }
        last = start + end;
    }
    out.push_str(&text[last..]);
    out
}

/// Sostituisce una parola intera (non dentro un'altra parola).
fn replace_word(text: &str, word: &str, with: &str) -> String {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (i, m) in text.match_indices(word) {
        if i < last {
            continue;
        }
        let before = text[..i].chars().next_back();
        let after = text[i + m.len()..].chars().next();
        if before.is_some_and(is_word) || after.is_some_and(is_word) {
            continue;
        }
        out.push_str(&text[last..i]);
        out.push_str(with);
        last = i + m.len();
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn windows() -> Redactor {
        Redactor::new(
            vec![
                (
                    r"C:\Users\mario\AppData\Local".into(),
                    "%LOCALAPPDATA%".into(),
                ),
                (r"C:\Users\mario".into(), "%USERPROFILE%".into()),
            ],
            Some("StigmaSocy"),
        )
    }

    #[test]
    fn user_folders_never_reach_a_report() {
        let r = windows();
        let line = r"Screenshot saved to C:\Users\mario\AppData\Local\planet-one\segnalazioni\x\schermo.png";
        let out = r.redact(line);
        assert!(!out.contains("mario"), "{out}");
        assert!(out.contains(r"%LOCALAPPDATA%\planet-one"), "{out}");
        // Con le barre dritte e le maiuscole diverse.
        let out = r.redact("c:/users/MARIO/Desktop/gioco");
        assert!(!out.to_lowercase().contains("mario"), "{out}");
        // Un'altra cartella utente, non nelle variabili d'ambiente.
        let out = r.redact(r"D:\Users\luigi\giochi e /home/anna/.local/share/planet-one");
        assert!(!out.contains("luigi") && !out.contains("anna"), "{out}");
        assert!(out.contains("/home/<utente>/.local"), "{out}");
    }

    #[test]
    fn the_character_name_is_hidden_as_a_whole_word() {
        let r = windows();
        let out = r.redact("Allora è deciso, StigmaSocy. Ma StigmaSocyX resta.");
        assert_eq!(out, "Allora è deciso, <giocatore>. Ma StigmaSocyX resta.");
    }

    #[test]
    fn json_strings_are_cleaned_and_the_ai_chat_is_omitted() {
        let r = windows();
        let mut state = json!({
            "dialogo": {"riga": "Ciao StigmaSocy"},
            "percorso": r"C:\Users\mario\AppData\Local\planet-one",
            "chat": {"turni": 3, "risposta": "Ho pane e focacce"},
        });
        omit_chat(&mut state);
        r.redact_json(&mut state);
        let text = state.to_string();
        assert!(
            !text.contains("mario") && !text.contains("StigmaSocy"),
            "{text}"
        );
        assert!(!text.contains("focacce"), "{text}");
        assert_eq!(state["chat"]["risposta"], "<omessa, 17 caratteri>");
    }

    #[test]
    fn short_or_missing_values_change_nothing() {
        let r = Redactor::new(vec![("/".into(), "~".into())], Some("A"));
        assert_eq!(r.redact("A / B"), "A / B");
    }
}
