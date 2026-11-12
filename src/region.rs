//! Regioni del mondo (ADR 0033): l'Isola di Fontevivo e il Continente del Nord.
//!
//! Ogni regione è un mondo a sé, con il suo terreno, i suoi dati (`assets/world/…`) e i suoi
//! piazzamenti; se ne carica una alla volta e il viaggio (barca di Gaspare) ricostruisce la scena.
//! La regione corrente è globale: la leggono anche funzioni che non sono sistemi (dati del mondo,
//! misura del terreno).

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Region {
    /// L'isola del Prologo, 300 × 300 m.
    #[default]
    Isola,
    /// Atto I: la piana del bosco di querce e betulle, il Grande Lago e Fontebruma.
    Nord,
}

#[cfg(not(test))]
static CURRENT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
#[cfg(test)]
thread_local! {
    // Nei test ogni thread ha la sua regione: i test girano in parallelo.
    static CURRENT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// Regione caricata in questo momento.
pub fn current() -> Region {
    #[cfg(not(test))]
    let v = CURRENT.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    let v = CURRENT.with(std::cell::Cell::get);
    Region::ALL[v as usize % Region::ALL.len()]
}

/// Cambia la regione corrente: va fatto solo mentre la scena si ricostruisce
/// (`scene::reset_scene`) o prima dell'avvio.
pub fn set(region: Region) {
    let v = Region::ALL.iter().position(|r| *r == region).unwrap_or(0) as u8;
    #[cfg(not(test))]
    CURRENT.store(v, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    CURRENT.with(|c| c.set(v));
}

impl Region {
    pub const ALL: [Region; 2] = [Region::Isola, Region::Nord];

    /// Nome breve, usato nei file e nella shell.
    pub fn key(self) -> &'static str {
        match self {
            Region::Isola => "isola",
            Region::Nord => "nord",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        let t = text.trim().to_lowercase();
        Region::ALL
            .into_iter()
            .find(|r| r.key() == t)
            .or(match t.as_str() {
                "fontevivo" | "sud" => Some(Region::Isola),
                "continente" | "fontebruma" => Some(Region::Nord),
                _ => None,
            })
    }

    /// Nome per il giocatore.
    pub fn title(self) -> &'static str {
        match self {
            Region::Isola => "Isola di Fontevivo",
            Region::Nord => "Continente del Nord — Piana delle Brume",
        }
    }

    /// Lato (m) del terreno quadrato, centrato nell'origine.
    pub fn size(self) -> f32 {
        match self {
            Region::Isola => 300.0,
            Region::Nord => 1200.0,
        }
    }

    /// Passo (m) della griglia del terreno: il continente è 16 volte più esteso e usa un
    /// passo più largo (801² vertici contro i 601² dell'isola); i sentieri sono larghi 3 m.
    pub fn step(self) -> f32 {
        match self {
            Region::Isola => 0.5,
            Region::Nord => 1.5,
        }
    }

    /// File di dati del mondo (`isola.txt`, `villaggio.txt`, `abitanti.txt`) di questa regione,
    /// relativo alla cartella del progetto.
    pub fn world_file(self, name: &str) -> String {
        match self {
            Region::Isola => format!("assets/world/{name}"),
            Region::Nord => format!("assets/world/nord/{name}"),
        }
    }

    /// File dei piazzamenti (edifici, lampioni, cartelli…).
    pub fn placements_file(self) -> &'static str {
        match self {
            Region::Isola => "debug/placements.txt",
            Region::Nord => "debug/placements-nord.txt",
        }
    }

    /// Dove si arriva sbarcando in questa regione (il molo), e da che parte si guarda (yaw).
    pub fn arrival(self) -> (Vec3, f32) {
        match self {
            // Molo del porticciolo di Fontevivo, a sud dell'isola.
            Region::Isola => (Vec3::new(8.5, 1.0, 96.0), 0.0),
            // Approdo delle Brume, sulla costa sud del continente: si guarda a nord, verso il bosco.
            Region::Nord => (Vec3::new(0.0, 4.0, 455.0), 0.0),
        }
    }
}

/// Copia incorporata dei file del mondo (i pacchetti distribuiti non hanno i file su disco
/// se qualcuno li toglie; in sviluppo vale sempre il file su disco, riletto all'avvio).
fn embedded(region: Region, name: &str) -> &'static str {
    match (region, name) {
        (Region::Isola, "isola.txt") => include_str!("../assets/world/isola.txt"),
        (Region::Isola, "villaggio.txt") => include_str!("../assets/world/villaggio.txt"),
        (Region::Isola, "abitanti.txt") => include_str!("../assets/world/abitanti.txt"),
        (Region::Isola, "placements") => include_str!("../debug/placements.txt"),
        (Region::Nord, "isola.txt") => include_str!("../assets/world/nord/isola.txt"),
        (Region::Nord, "villaggio.txt") => include_str!("../assets/world/nord/villaggio.txt"),
        (Region::Nord, "abitanti.txt") => include_str!("../assets/world/nord/abitanti.txt"),
        (Region::Nord, "placements") => include_str!("../debug/placements-nord.txt"),
        _ => "",
    }
}

/// Testo di un file del mondo della regione corrente: `isola.txt`, `villaggio.txt`,
/// `abitanti.txt` (disco, poi copia incorporata).
pub fn world_text(name: &str) -> String {
    let region = current();
    let rel = region.world_file(name);
    [
        std::path::PathBuf::from(&rel),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&rel),
    ]
    .iter()
    .find_map(|p| std::fs::read_to_string(p).ok())
    .unwrap_or_else(|| embedded(region, name).to_string())
}

/// Piazzamenti incorporati della regione corrente (quando il file su disco non c'è).
pub fn embedded_placements() -> &'static str {
    embedded(current(), "placements")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_have_keys_and_files() {
        assert_eq!(Region::parse("Nord"), Some(Region::Nord));
        assert_eq!(Region::parse("continente"), Some(Region::Nord));
        assert_eq!(Region::parse("isola"), Some(Region::Isola));
        assert_eq!(Region::parse("luna"), None);
        assert_eq!(
            Region::Isola.world_file("isola.txt"),
            "assets/world/isola.txt"
        );
        assert_eq!(
            Region::Nord.world_file("isola.txt"),
            "assets/world/nord/isola.txt"
        );
    }

    #[test]
    fn the_continent_terrain_stays_light() {
        let verts = |r: Region| ((r.size() / r.step()) as usize + 1).pow(2);
        // Sedici volte la superficie, meno del doppio dei vertici.
        assert!(verts(Region::Nord) < verts(Region::Isola) * 2);
        for r in Region::ALL {
            assert_eq!((r.size() / r.step()).fract(), 0.0);
        }
    }

    #[test]
    fn every_region_has_its_world_files() {
        for r in Region::ALL {
            set(r);
            for name in ["isola.txt", "villaggio.txt", "abitanti.txt"] {
                assert!(!world_text(name).is_empty(), "{r:?} {name}");
            }
            assert!(embedded_placements().contains("[P-"), "{r:?}");
        }
        set(Region::Isola);
    }

    #[test]
    fn current_region_can_be_switched() {
        assert_eq!(current(), Region::Isola);
        set(Region::Nord);
        assert_eq!(current(), Region::Nord);
        set(Region::Isola);
    }
}
