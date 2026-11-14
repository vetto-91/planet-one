//! Inventario e oggetti raccoglibili (Fase 4.1).
//!
//! Modello minimale in memoria (nessun salvataggio su file):
//! - [`ItemKind`]: tipi di oggetto (Pozione, Moneta).
//! - [`ItemStack`]: quantità posseduta per un tipo.
//! - [`Inventory`]: resource con gli slot.
//! - [`Pickup`]: entità fisica raccoglibile nella scena (sp disappear on pickup).

use crate::scene::SceneNode;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Tipi di oggetto raccoglibili.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ItemKind {
    HealthPotion,
    Coin,
    ShortSword,
    LongSword,
    Axe,
    Shield,
    Bow,
    Arrow,
    Apple,
    Fish,
    /// Zaini: aumentano il peso che si può portare.
    Satchel,
    CanvasPack,
    LeatherPack,
    ExpeditionPack,
    /// Porta più peso per qualche minuto.
    MulePotion,
    /// Oggetto incantato: più peso finché è indossato.
    CarryBelt,
    /// Passepartout: apre ogni porta (solo dalla shell).
    MasterKey,
    /// Grimaldello: apre le serrature dei bauli, a volte si spezza (ADR 0039).
    Lockpick,
    /// Materiali da costruzione (ADR 0042).
    Wood,
    Stone,
    /// Armature (ADR 0049): meno danno subito, più peso.
    LeatherArmor,
    MailArmor,
    PlateArmor,
    /// Oggetti di trama (ADR 0044).
    Relic(Relic),
}

/// Oggetti di trama del Prologo: si conservano, non si vendono, non vanno nella barra (ADR 0044).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Relic {
    /// La tavoletta incisa trovata al campo del ladro.
    Tablet,
    /// La prima delle dodici monete, affidata dal Viandante a chi risparmia il ladro.
    MotherCoin,
    /// Cordicelle annodate trovate tra i rottami della spiaggia.
    Quipu,
    /// Il disco di bronzo della torre antica.
    TyreDisc,
    /// Il quipu grande della grotta.
    MasterQuipu,
    /// Le piastrine dei tre banditi.
    BanditTags,
    /// Il campanaccio d'oro e rame del santuario.
    Cowbell,
    /// La bussola di Lauro, in cima alla torre.
    LauroCompass,
    /// La chiave di rame trovata vicino al pozzo.
    CopperKey,
    /// Un bulbo di Luce Viva di ricambio.
    LightBulb,
    /// La pergamena con le correnti dello Stretto.
    CurrentsMap,
    /// La carta delle secche di Gaspare.
    ShoalsMap,
    /// Pane da viaggio di Marta: sazia e dà forza.
    Focaccia,
    /// Amuleto di Ida: si porta qualche chilo in più.
    FeatherAmulet,
    /// Indizi (ADR 0045): letti, rivelano il segreto di chi li ha scritti.
    NightLog,
    SeedBox,
    GrandfatherLetter,
    HuntingNotes,
}

impl Relic {
    /// Tutti gli oggetti di trama (serve ai test).
    #[cfg(test)]
    pub const ALL: [Relic; 18] = [
        Relic::Tablet,
        Relic::MotherCoin,
        Relic::Quipu,
        Relic::TyreDisc,
        Relic::MasterQuipu,
        Relic::BanditTags,
        Relic::Cowbell,
        Relic::LauroCompass,
        Relic::CopperKey,
        Relic::LightBulb,
        Relic::CurrentsMap,
        Relic::ShoalsMap,
        Relic::Focaccia,
        Relic::FeatherAmulet,
        Relic::NightLog,
        Relic::SeedBox,
        Relic::GrandfatherLetter,
        Relic::HuntingNotes,
    ];

    fn name(self) -> &'static str {
        match self {
            Relic::Tablet => "Tavoletta del ladro",
            Relic::MotherCoin => "Moneta Madre",
            Relic::Quipu => "Quipu dei sette nodi",
            Relic::TyreDisc => "Disco di Tiro",
            Relic::MasterQuipu => "Quipu Maestro",
            Relic::BanditTags => "Piastrine dei banditi",
            Relic::Cowbell => "Campanaccio del Sole",
            Relic::LauroCompass => "Bussola di Lauro",
            Relic::CopperKey => "Chiave di rame",
            Relic::LightBulb => "Bulbo di Luce Viva",
            Relic::CurrentsMap => "Mappa delle Correnti",
            Relic::ShoalsMap => "Mappa delle Secche",
            Relic::Focaccia => "Focaccia del Navigante",
            Relic::FeatherAmulet => "Amuleto Piuma",
            Relic::NightLog => "Registro delle Notti",
            Relic::SeedBox => "Scatola dei semi",
            Relic::GrandfatherLetter => "Lettera del nonno di Osvaldo",
            Relic::HuntingNotes => "Taccuino di caccia",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Relic::Tablet => "tavoletta",
            Relic::MotherCoin => "moneta-madre",
            Relic::Quipu => "quipu",
            Relic::TyreDisc => "disco",
            Relic::MasterQuipu => "quipu-maestro",
            Relic::BanditTags => "piastrine",
            Relic::Cowbell => "campanaccio",
            Relic::LauroCompass => "bussola",
            Relic::CopperKey => "chiave-rame",
            Relic::LightBulb => "bulbo",
            Relic::CurrentsMap => "mappa-correnti",
            Relic::ShoalsMap => "mappa-secche",
            Relic::Focaccia => "focaccia",
            Relic::FeatherAmulet => "amuleto",
            Relic::NightLog => "registro",
            Relic::SeedBox => "scatola-semi",
            Relic::GrandfatherLetter => "lettera",
            Relic::HuntingNotes => "taccuino",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Relic::Tablet => "Segni incisi da Renzo: un nome, una rotta, una promessa.",
            Relic::MotherCoin => "La prima delle dodici. Il conio non somiglia a nessun altro.",
            Relic::Quipu => "Sette nodi su cordicelle colorate: un conto, o un messaggio.",
            Relic::TyreDisc => "Bronzo verde, con navi e stelle incise a spirale.",
            Relic::MasterQuipu => "Il quipu grande della grotta: cento nodi, una mappa.",
            Relic::BanditTags => "Tre piastrine con tre nomi. Chi le portava non dorme più.",
            Relic::Cowbell => "Oro e rame: quando suona le bestie si calmano.",
            Relic::LauroCompass => "Una bussola d'ottone con dentro una perlina rossa.",
            Relic::CopperKey => "Antica, verde di salsedine. Non si sa ancora cosa apra.",
            Relic::LightBulb => "Vetro e fili sottili: la Luce Viva, di ricambio.",
            Relic::CurrentsMap => "Le correnti dello Stretto, disegnate a inchiostro.",
            Relic::ShoalsMap => "Dove l'acqua è bassa e dove morde: la carta di Gaspare.",
            Relic::Focaccia => "Cura 30 punti vita e ridà fiato.",
            Relic::FeatherAmulet => "Leggero come una piuma: porti 5 kg in più finché lo indossi.",
            Relic::NightLog => "Le notti di Ettore al faro: lampi, risposte, un nome di donna.",
            Relic::SeedBox => "Semi dorati e foglie incise, raccolti dalle pale del mulino.",
            Relic::GrandfatherLetter => {
                "Una lettera ingiallita: parla di un'anfora e di una carta."
            }
            Relic::HuntingNotes => "Appunti di Silvia sulle tracce dei banditi fino alla grotta.",
        }
    }

    fn color(self) -> Color {
        match self {
            Relic::MotherCoin | Relic::Cowbell => Color::srgb(0.95, 0.75, 0.25),
            Relic::Focaccia => Color::srgb(0.85, 0.65, 0.35),
            Relic::FeatherAmulet => Color::srgb(0.85, 0.9, 0.95),
            _ => Color::srgb(0.7, 0.55, 0.35),
        }
    }
}

/// Categoria dell'inventario.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Category {
    Weapons,
    Defense,
    Ammo,
    Food,
    Gear,
    Valuables,
    /// Oggetti di trama.
    Keepsakes,
}

impl Category {
    pub const ALL: [Category; 7] = [
        Category::Weapons,
        Category::Defense,
        Category::Ammo,
        Category::Food,
        Category::Gear,
        Category::Valuables,
        Category::Keepsakes,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::Weapons => "Armi",
            Category::Defense => "Difesa",
            Category::Ammo => "Munizioni",
            Category::Food => "Cibo e cure",
            Category::Gear => "Equipaggiamento",
            Category::Valuables => "Valori",
            Category::Keepsakes => "Ricordi",
        }
    }
}

impl ItemKind {
    /// Tutti i tipi di oggetto.
    pub const ALL: [ItemKind; 41] = [
        ItemKind::HealthPotion,
        ItemKind::Coin,
        ItemKind::ShortSword,
        ItemKind::LongSword,
        ItemKind::Axe,
        ItemKind::Shield,
        ItemKind::Bow,
        ItemKind::Arrow,
        ItemKind::Apple,
        ItemKind::Fish,
        ItemKind::Satchel,
        ItemKind::CanvasPack,
        ItemKind::LeatherPack,
        ItemKind::ExpeditionPack,
        ItemKind::MulePotion,
        ItemKind::CarryBelt,
        ItemKind::MasterKey,
        ItemKind::Lockpick,
        ItemKind::Wood,
        ItemKind::Stone,
        ItemKind::LeatherArmor,
        ItemKind::MailArmor,
        ItemKind::PlateArmor,
        ItemKind::Relic(Relic::Tablet),
        ItemKind::Relic(Relic::MotherCoin),
        ItemKind::Relic(Relic::Quipu),
        ItemKind::Relic(Relic::TyreDisc),
        ItemKind::Relic(Relic::MasterQuipu),
        ItemKind::Relic(Relic::BanditTags),
        ItemKind::Relic(Relic::Cowbell),
        ItemKind::Relic(Relic::LauroCompass),
        ItemKind::Relic(Relic::CopperKey),
        ItemKind::Relic(Relic::LightBulb),
        ItemKind::Relic(Relic::CurrentsMap),
        ItemKind::Relic(Relic::ShoalsMap),
        ItemKind::Relic(Relic::Focaccia),
        ItemKind::Relic(Relic::FeatherAmulet),
        ItemKind::Relic(Relic::NightLog),
        ItemKind::Relic(Relic::SeedBox),
        ItemKind::Relic(Relic::GrandfatherLetter),
        ItemKind::Relic(Relic::HuntingNotes),
    ];

    /// Peso reale di un'unità (kg). Monete e munizioni non pesano.
    pub fn weight_kg(self) -> Option<f32> {
        match self {
            ItemKind::HealthPotion => Some(0.4),
            ItemKind::ShortSword => Some(1.0),
            ItemKind::LongSword => Some(1.5),
            ItemKind::Axe => Some(1.8),
            ItemKind::Shield => Some(3.5),
            ItemKind::Bow => Some(0.9),
            ItemKind::Apple => Some(0.15),
            ItemKind::Fish => Some(0.8),
            ItemKind::Satchel => Some(0.4),
            ItemKind::CanvasPack => Some(0.8),
            ItemKind::LeatherPack => Some(1.5),
            ItemKind::ExpeditionPack => Some(2.5),
            ItemKind::MulePotion => Some(0.4),
            ItemKind::CarryBelt => Some(0.3),
            ItemKind::MasterKey => Some(0.05),
            ItemKind::Lockpick => Some(0.02),
            ItemKind::Wood => Some(2.5),
            ItemKind::Stone => Some(4.0),
            ItemKind::LeatherArmor => Some(5.0),
            ItemKind::MailArmor => Some(11.0),
            ItemKind::PlateArmor => Some(18.0),
            ItemKind::Relic(Relic::Focaccia) => Some(0.3),
            ItemKind::Relic(_) => Some(0.05),
            ItemKind::Coin | ItemKind::Arrow => None,
        }
    }

    /// Categoria nell'inventario.
    pub fn category(self) -> Category {
        match self {
            ItemKind::ShortSword | ItemKind::LongSword | ItemKind::Axe | ItemKind::Bow => {
                Category::Weapons
            }
            ItemKind::Shield
            | ItemKind::LeatherArmor
            | ItemKind::MailArmor
            | ItemKind::PlateArmor => Category::Defense,
            ItemKind::Arrow => Category::Ammo,
            ItemKind::HealthPotion | ItemKind::Apple | ItemKind::Fish | ItemKind::MulePotion => {
                Category::Food
            }
            ItemKind::Satchel
            | ItemKind::CanvasPack
            | ItemKind::LeatherPack
            | ItemKind::ExpeditionPack
            | ItemKind::CarryBelt
            | ItemKind::Lockpick
            | ItemKind::Wood
            | ItemKind::Stone => Category::Gear,
            ItemKind::Coin | ItemKind::MasterKey => Category::Valuables,
            ItemKind::Relic(Relic::Focaccia) => Category::Food,
            ItemKind::Relic(_) => Category::Keepsakes,
        }
    }

    /// File dell'icona (`assets/textures/icone/<nome>.png`).
    pub fn icon(self) -> &'static str {
        match self {
            ItemKind::HealthPotion => "pozione",
            ItemKind::Coin => "moneta",
            ItemKind::ShortSword => "spada-corta",
            ItemKind::LongSword => "spada-lunga",
            ItemKind::Axe => "ascia",
            ItemKind::Shield => "scudo",
            ItemKind::Bow => "arco",
            ItemKind::Arrow => "frecce",
            ItemKind::Apple => "mela",
            ItemKind::Fish => "pesce",
            ItemKind::Satchel => "bisaccia",
            ItemKind::CanvasPack => "zaino-tela",
            ItemKind::LeatherPack => "zaino-cuoio",
            ItemKind::ExpeditionPack => "zaino-spedizione",
            ItemKind::MulePotion => "pozione-mulo",
            ItemKind::CarryBelt => "cintura",
            ItemKind::MasterKey => "passepartout",
            ItemKind::Lockpick => "grimaldello",
            ItemKind::Wood => "legname",
            ItemKind::Stone => "pietra",
            ItemKind::LeatherArmor => "armatura-cuoio",
            ItemKind::MailArmor => "armatura-maglia",
            ItemKind::PlateArmor => "armatura-piastre",
            ItemKind::Relic(r) => r.icon(),
        }
    }

    /// Le monete hanno il loro contatore: nella barra non vanno mai.
    pub fn can_go_in_hotbar(self) -> bool {
        !matches!(
            self,
            ItemKind::Coin | ItemKind::MasterKey | ItemKind::Wood | ItemKind::Stone
        ) && !matches!(self, ItemKind::Relic(r) if r != Relic::Focaccia && r != Relic::FeatherAmulet)
    }

    /// Nome mostrato nell'inventario.
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::HealthPotion => "Pozione di cura",
            ItemKind::Coin => "Moneta",
            ItemKind::ShortSword => "Spada corta",
            ItemKind::LongSword => "Spada lunga",
            ItemKind::Axe => "Ascia",
            ItemKind::Shield => "Scudo",
            ItemKind::Bow => "Arco",
            ItemKind::Arrow => "Frecce",
            ItemKind::Apple => "Mela",
            ItemKind::Fish => "Pesce",
            ItemKind::Satchel => "Bisaccia",
            ItemKind::CanvasPack => "Zaino di tela",
            ItemKind::LeatherPack => "Zaino di cuoio",
            ItemKind::ExpeditionPack => "Zaino da spedizione",
            ItemKind::MulePotion => "Pozione del mulo",
            ItemKind::CarryBelt => "Cintura del portatore",
            ItemKind::MasterKey => "Passepartout",
            ItemKind::Lockpick => "Grimaldello",
            ItemKind::Wood => "Legname",
            ItemKind::Stone => "Pietra",
            ItemKind::LeatherArmor => "Armatura di cuoio",
            ItemKind::MailArmor => "Cotta di maglia",
            ItemKind::PlateArmor => "Armatura a piastre",
            ItemKind::Relic(r) => r.name(),
        }
    }

    /// Descrizione breve (negozi e tooltip).
    pub fn description(self) -> &'static str {
        match self {
            ItemKind::HealthPotion => "Cura 25 punti vita.",
            ItemKind::Coin => "",
            ItemKind::ShortSword => "Danno 9, veloce.",
            ItemKind::LongSword => "Danno 14, raggio lungo.",
            ItemKind::Axe => "Danno 18, lenta, colpisce più nemici.",
            ItemKind::Shield => "Riduce il danno subito del 60%.",
            ItemKind::Bow => "Colpisce da lontano: serve una freccia a colpo.",
            ItemKind::Arrow => "Munizioni per l'arco (pacco da 10).",
            ItemKind::Apple => "Cura 10 punti vita.",
            ItemKind::Fish => "Cura 20 punti vita.",
            ItemKind::Satchel => "Porta 5 kg in più.",
            ItemKind::CanvasPack => "Porta 10 kg in più.",
            ItemKind::LeatherPack => "Porta 20 kg in più.",
            ItemKind::ExpeditionPack => "Porta 30 kg in più.",
            ItemKind::MulePotion => "Per 5 minuti porti 20 kg in più.",
            ItemKind::CarryBelt => "Incantata: porti 10 kg in più finché la indossi.",
            ItemKind::MasterKey => "Apre qualsiasi porta del paese.",
            ItemKind::Lockpick => "Per forzare la serratura di un baule. Può spezzarsi.",
            ItemKind::Wood => "Un tronco squadrato, per costruire.",
            ItemKind::Stone => "Un blocco di pietra, per costruire.",
            ItemKind::LeatherArmor => "Subisci il 15% di danno in meno. Leggera.",
            ItemKind::MailArmor => "Subisci il 30% di danno in meno.",
            ItemKind::PlateArmor => "Subisci il 45% di danno in meno. Pesante: corri meno.",
            ItemKind::Relic(r) => r.description(),
        }
    }

    /// Cerca un oggetto per nome (per i salvataggi), senza badare a maiuscole e accenti.
    pub fn from_name(text: &str) -> Option<ItemKind> {
        let wanted = crate::inventory_ui::normalize(text.trim());
        let names = ItemKind::ALL.map(|k| (k, crate::inventory_ui::normalize(k.name())));
        if let Some((k, _)) = names.iter().find(|(_, n)| *n == wanted) {
            return Some(*k);
        }
        // Singolare o plurale («freccia» per «Frecce»): stesso inizio di almeno 5 lettere,
        // lunghezza quasi uguale, e un solo oggetto che corrisponde.
        let common =
            |a: &str, b: &str| a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
        let close: Vec<ItemKind> = names
            .iter()
            .filter(|(_, n)| {
                common(n, &wanted) >= 5 && n.chars().count().abs_diff(wanted.chars().count()) <= 2
            })
            .map(|(k, _)| *k)
            .collect();
        (close.len() == 1).then(|| close[0])
    }

    /// Chili in più se è uno zaino.
    pub fn backpack_kg(self) -> Option<f32> {
        match self {
            ItemKind::Satchel => Some(5.0),
            ItemKind::CanvasPack => Some(10.0),
            ItemKind::LeatherPack => Some(20.0),
            ItemKind::ExpeditionPack => Some(30.0),
            _ => None,
        }
    }

    /// Chili in più se è un oggetto incantato da indossare.
    pub fn charm_kg(self) -> Option<f32> {
        match self {
            ItemKind::CarryBelt => Some(10.0),
            ItemKind::Relic(Relic::FeatherAmulet) => Some(5.0),
            _ => None,
        }
    }

    /// Effetto a tempo se bevuto: chili in più e secondi.
    pub fn carry_potion(self) -> Option<(f32, f32)> {
        (self == ItemKind::MulePotion).then_some((20.0, 300.0))
    }

    /// Frazione di danno che passa attraverso l'armatura (`None` se non è un'armatura).
    pub fn armor_factor(self) -> Option<f32> {
        match self {
            ItemKind::LeatherArmor => Some(0.85),
            ItemKind::MailArmor => Some(0.7),
            ItemKind::PlateArmor => Some(0.55),
            _ => None,
        }
    }

    /// Arma che si impugna (spada, ascia, arco).
    pub fn is_weapon(self) -> bool {
        matches!(
            self,
            ItemKind::ShortSword | ItemKind::LongSword | ItemKind::Axe | ItemKind::Bow
        )
    }

    /// Punti vita curati se si mangia/beve.
    pub fn heal(self) -> Option<f32> {
        match self {
            ItemKind::HealthPotion => Some(POTION_HEAL),
            ItemKind::Apple => Some(10.0),
            ItemKind::Fish => Some(20.0),
            ItemKind::Relic(Relic::Focaccia) => Some(30.0),
            _ => None,
        }
    }

    /// Colore dell'icona placeholder (usato per il disegno egui).
    pub fn color(self) -> Color {
        match self {
            ItemKind::HealthPotion => Color::srgb(0.9, 0.2, 0.2),
            ItemKind::Coin => Color::srgb(0.95, 0.8, 0.2),
            ItemKind::ShortSword => Color::srgb(0.7, 0.72, 0.78),
            ItemKind::LongSword => Color::srgb(0.85, 0.87, 0.95),
            ItemKind::Axe => Color::srgb(0.55, 0.38, 0.22),
            ItemKind::Shield => Color::srgb(0.25, 0.4, 0.75),
            ItemKind::Bow => Color::srgb(0.6, 0.42, 0.2),
            ItemKind::Arrow => Color::srgb(0.85, 0.78, 0.55),
            ItemKind::Apple => Color::srgb(0.5, 0.8, 0.3),
            ItemKind::Fish => Color::srgb(0.45, 0.6, 0.75),
            ItemKind::Satchel => Color::srgb(0.6, 0.48, 0.3),
            ItemKind::CanvasPack => Color::srgb(0.55, 0.55, 0.42),
            ItemKind::LeatherPack => Color::srgb(0.5, 0.3, 0.16),
            ItemKind::ExpeditionPack => Color::srgb(0.3, 0.4, 0.3),
            ItemKind::MulePotion => Color::srgb(0.3, 0.55, 0.9),
            ItemKind::CarryBelt => Color::srgb(0.7, 0.4, 0.8),
            ItemKind::MasterKey => Color::srgb(0.9, 0.75, 0.3),
            ItemKind::Lockpick => Color::srgb(0.75, 0.55, 0.3),
            ItemKind::Wood => Color::srgb(0.55, 0.38, 0.2),
            ItemKind::Stone => Color::srgb(0.6, 0.6, 0.62),
            ItemKind::LeatherArmor => Color::srgb(0.55, 0.35, 0.2),
            ItemKind::MailArmor => Color::srgb(0.6, 0.62, 0.66),
            ItemKind::PlateArmor => Color::srgb(0.75, 0.77, 0.8),
            ItemKind::Relic(r) => r.color(),
        }
    }
}

static GRANTS: std::sync::Mutex<Vec<(ItemKind, u32)>> = std::sync::Mutex::new(Vec::new());

/// Dà un oggetto al giocatore al prossimo frame, da un sistema che non ha l'inventario.
pub fn grant(kind: ItemKind, qty: u32) {
    if let Ok(mut g) = GRANTS.lock() {
        g.push((kind, qty));
    }
}

/// Sistema: consegna gli oggetti di [`grant`].
pub fn apply_grants(mut inventory: ResMut<Inventory>) {
    let pending = GRANTS
        .lock()
        .map(|mut g| std::mem::take(&mut *g))
        .unwrap_or_default();
    for (kind, qty) in pending {
        inventory.add(kind, qty);
        info!("[INVENTORY] ricevuto {} x{qty}", kind.name());
    }
}

/// Uno slot dell'inventario: tipo + quantità.
#[derive(Clone, Debug)]
pub struct ItemStack {
    pub kind: ItemKind,
    pub qty: u32,
}

/// Monete con cui si inizia (per i primi acquisti).
pub const START_COINS: u32 = 30;

/// Inventario del player (resource, NON soggetta a reset scena).
#[derive(Resource)]
pub struct Inventory {
    pub slots: Vec<ItemStack>,
    /// Oggetto assegnato a ogni tasto della barra (1..9 e 0).
    pub hotbar: [Option<ItemKind>; 10],
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: vec![ItemStack {
                kind: ItemKind::Coin,
                qty: START_COINS,
            }],
            hotbar: [None; 10],
        }
    }
}

/// Equipaggiamento: arma impugnata e scudo.
#[derive(Resource, Default)]
pub struct Equipment {
    pub weapon: Option<ItemKind>,
    pub shield: bool,
    pub backpack: Option<ItemKind>,
    /// Oggetto incantato indossato.
    pub charm: Option<ItemKind>,
    /// Armatura indossata (ADR 0049).
    pub armor: Option<ItemKind>,
}

impl Equipment {
    /// L'oggetto è impugnato o indossato?
    pub fn is_worn(&self, kind: ItemKind) -> bool {
        self.weapon == Some(kind)
            || (kind == ItemKind::Shield && self.shield)
            || self.backpack == Some(kind)
            || self.charm == Some(kind)
            || self.armor == Some(kind)
    }

    /// Frazione di danno che arriva al giocatore: armatura indossata e talento Armatura.
    pub fn armor_factor(&self) -> f32 {
        self.armor
            .and_then(ItemKind::armor_factor)
            .map_or(1.0, |f| {
                f * (1.0 - 0.05 * crate::skills::rank(crate::skills::Skill::Armor))
            })
    }

    /// Toglie ciò che non si possiede più.
    pub fn drop_missing(&mut self, inventory: &Inventory) {
        if self.weapon.is_some_and(|k| inventory.qty(k) == 0) {
            self.weapon = None;
        }
        if self.shield && inventory.qty(ItemKind::Shield) == 0 {
            self.shield = false;
        }
        if self.backpack.is_some_and(|k| inventory.qty(k) == 0) {
            self.backpack = None;
        }
        if self.charm.is_some_and(|k| inventory.qty(k) == 0) {
            self.charm = None;
        }
        if self.armor.is_some_and(|k| inventory.qty(k) == 0) {
            self.armor = None;
        }
    }
}

/// Statistiche di un'arma (a mani nude se `None`).
#[derive(Clone, Copy, Debug)]
pub struct WeaponStats {
    pub damage: f32,
    pub cooldown: f32,
    pub range: f32,
    /// Colpisce tutti i nemici nel cono (ascia).
    pub cleave: bool,
    pub ranged: bool,
}

pub fn weapon_stats(w: Option<ItemKind>) -> WeaponStats {
    let (damage, cooldown, range, cleave, ranged) = match w {
        Some(ItemKind::ShortSword) => (9.0, 0.45, 2.6, false, false),
        Some(ItemKind::LongSword) => (14.0, 0.75, 3.2, false, false),
        Some(ItemKind::Axe) => (18.0, 0.95, 2.6, true, false),
        Some(ItemKind::Bow) => (10.0, 0.9, 60.0, false, true),
        _ => (4.0, 0.6, 2.5, false, false),
    };
    WeaponStats {
        damage,
        cooldown,
        range,
        cleave,
        ranged,
    }
}

/// Quanto scudo riduce il danno subito (moltiplicatore).
pub const SHIELD_FACTOR: f32 = 0.4;

impl Inventory {
    /// Aggiunge (o incrementa) una quantità di un tipo. Un oggetto nuovo prende il primo tasto libero.
    pub fn add(&mut self, kind: ItemKind, qty: u32) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.kind == kind) {
            slot.qty += qty;
            return;
        }
        self.slots.push(ItemStack { kind, qty });
        if kind.can_go_in_hotbar()
            && self.hotbar_position(kind).is_none()
            && let Some(free) = self.hotbar.iter().position(Option::is_none)
        {
            self.hotbar[free] = Some(kind);
        }
    }

    /// Tasto (0..10) a cui è assegnato l'oggetto.
    pub fn hotbar_position(&self, kind: ItemKind) -> Option<usize> {
        self.hotbar.iter().position(|k| *k == Some(kind))
    }

    /// Oggetto nello slot `i` della barra, se posseduto.
    pub fn hotbar_slot(&self, i: usize) -> Option<&ItemStack> {
        let kind = (*self.hotbar.get(i)?)?;
        self.slots.iter().find(|s| s.kind == kind && s.qty > 0)
    }

    /// Assegna un oggetto a un tasto (o lo toglie con `None`). Se il tasto è occupato i due si scambiano.
    pub fn assign_hotbar(&mut self, kind: ItemKind, slot: Option<usize>) {
        if !kind.can_go_in_hotbar() {
            return;
        }
        let previous = self.hotbar_position(kind);
        match slot {
            None => {
                if let Some(p) = previous {
                    self.hotbar[p] = None;
                }
            }
            Some(i) if i < self.hotbar.len() => {
                let occupant = self.hotbar[i];
                self.hotbar[i] = Some(kind);
                if let Some(p) = previous
                    && p != i
                {
                    self.hotbar[p] = occupant;
                }
            }
            Some(_) => {}
        }
    }

    /// Peso totale trasportato (kg): monete e munizioni non contano.
    pub fn total_weight(&self) -> f32 {
        self.slots
            .iter()
            .filter_map(|s| s.kind.weight_kg().map(|w| w * s.qty as f32))
            .sum::<f32>()
            .abs() // la somma vuota è -0.0
    }

    /// Quantità posseduta di un tipo (0 se assente).
    pub fn qty(&self, kind: ItemKind) -> u32 {
        self.slots
            .iter()
            .find(|s| s.kind == kind)
            .map_or(0, |s| s.qty)
    }

    /// Consuma `qty` unità di un tipo; lo slot sparisce a quantità 0.
    pub fn consume(&mut self, kind: ItemKind, qty: u32) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.kind == kind) {
            slot.qty = slot.qty.saturating_sub(qty);
        }
        self.slots.retain(|s| s.qty > 0);
        // Finito un oggetto, il suo tasto si libera.
        for i in 0..self.hotbar.len() {
            if let Some(k) = self.hotbar[i]
                && self.qty(k) == 0
            {
                self.hotbar[i] = None;
            }
        }
    }
}

/// Punti vita ripristinati da una pozione.
pub const POTION_HEAL: f32 = 25.0;

/// Usa un oggetto: cura (si consuma; non se la vita è piena), impugna o ripone l'arma, imbraccia lo scudo.
/// Condivisa da barra e inventario.
pub fn use_item(
    kind: ItemKind,
    inventory: &mut Inventory,
    equip: &mut Equipment,
    health: Option<Mut<crate::player::PlayerHealth>>,
    effects: &mut crate::character::ActiveEffects,
) {
    if let Some((carry_kg, seconds)) = kind.carry_potion() {
        effects.add(crate::character::Effect {
            source: kind.name().to_string(),
            carry_kg,
            remaining: Some(seconds),
        });
        inventory.consume(kind, 1);
        crate::skills::train(crate::skills::Skill::Herbalism, 8.0);
        info!("[INVENTORY] {} bevuta", kind.name());
    } else if kind.backpack_kg().is_some() {
        equip.backpack = if equip.backpack == Some(kind) {
            None
        } else {
            Some(kind)
        };
        info!("[INVENTORY] zaino: {:?}", equip.backpack);
    } else if kind.charm_kg().is_some() {
        equip.charm = if equip.charm == Some(kind) {
            None
        } else {
            Some(kind)
        };
        info!("[INVENTORY] oggetto indossato: {:?}", equip.charm);
    } else if let Some(heal) = kind.heal() {
        let Some(mut hp) = health else {
            return;
        };
        if hp.current >= hp.max {
            info!("[INVENTORY] {} non usato: vita già piena", kind.name());
            return;
        }
        let heal = if kind == ItemKind::HealthPotion {
            crate::skills::train(crate::skills::Skill::Herbalism, 8.0);
            heal * (1.0 + 0.1 * crate::skills::rank(crate::skills::Skill::Herbalism))
        } else {
            heal
        };
        hp.current = (hp.current + heal).min(hp.max);
        inventory.consume(kind, 1);
        info!(
            "[INVENTORY] {} usato: vita {:.0}/{:.0}",
            kind.name(),
            hp.current,
            hp.max
        );
    } else if kind.armor_factor().is_some() {
        // Indossa / toglie l'armatura.
        equip.armor = if equip.armor == Some(kind) {
            None
        } else {
            Some(kind)
        };
        info!("[INVENTORY] armatura: {:?}", equip.armor);
    } else if kind.is_weapon() {
        // Impugna / rimette via l'arma.
        equip.weapon = if equip.weapon == Some(kind) {
            None
        } else {
            Some(kind)
        };
        info!("[INVENTORY] arma impugnata: {:?}", equip.weapon);
    } else if kind == ItemKind::Shield {
        equip.shield = !equip.shield;
        info!("[INVENTORY] scudo: {}", equip.shield);
    }
}

/// Sistema: i tasti 1..9 e 0 usano l'oggetto assegnato al riquadro corrispondente della barra.
/// Disattivato mentre un dialogo è aperto.
pub fn use_hotbar_items(
    keys: Res<ButtonInput<KeyCode>>,
    dialog: Res<crate::entities::DialogState>,
    mut inventory: ResMut<Inventory>,
    mut equip: ResMut<Equipment>,
    mut health: Query<&mut crate::player::PlayerHealth, With<crate::player::Player>>,
    mut effects: ResMut<crate::character::ActiveEffects>,
) {
    if dialog.open {
        return;
    }
    const SLOT_KEYS: [KeyCode; 10] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Digit0,
    ];
    for (i, key) in SLOT_KEYS.iter().enumerate() {
        if !keys.just_pressed(*key) {
            continue;
        }
        let Some(kind) = inventory.hotbar_slot(i).map(|s| s.kind) else {
            continue;
        };
        use_item(
            kind,
            &mut inventory,
            &mut equip,
            health.single_mut().ok(),
            &mut effects,
        );
    }
}

/// Marker su entità raccoglibili nella scena 3D.
#[derive(Component)]
pub struct Pickup {
    /// Gettato dal giocatore: si riprende con E, non da solo.
    pub dropped: bool,
    pub kind: ItemKind,
    pub qty: u32,
}

/// Spawn di un pickup (mesh + collider sensore non-bloccante? usiamo collider
/// statico piccolissimo e controllo distanza nel sistema di raccolta).
pub fn spawn_pickup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind: ItemKind,
    qty: u32,
    position: Vec3,
) -> Entity {
    let color = kind.color();
    let c = color.to_srgba();
    let size = match kind {
        ItemKind::Coin => 0.3,
        _ => 0.4,
    };
    commands
        .spawn((
            SceneNode,
            Pickup {
                kind,
                qty,
                dropped: false,
            },
            RigidBody::Static,
            Collider::cuboid(size, size, size),
            Mesh3d(meshes.add(Cuboid::new(size, size, size))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::new(c.red, c.green, c.blue, 0.0) * 0.3,
                ..default()
            })),
            Transform::from_translation(position),
        ))
        .id()
}

/// Oggetto a terra: originale (raccolta automatica) o gettato dal giocatore (si riprende con E).
#[allow(clippy::too_many_arguments)]
pub fn spawn_ground_item(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind: ItemKind,
    qty: u32,
    position: Vec3,
    dropped: bool,
) -> Entity {
    let entity = spawn_pickup(commands, meshes, materials, kind, qty, position);
    if dropped {
        commands.entity(entity).insert((
            Pickup {
                kind,
                qty,
                dropped: true,
            },
            crate::hut::PropKind::Pickup,
            crate::entities::Interactable {
                prompt: if qty > 1 {
                    format!("[E] Raccogli {} ×{qty}", kind.name())
                } else {
                    format!("[E] Raccogli {}", kind.name())
                },
                range: 2.6,
                aim: Some(0.5),
                use_from: None,
            },
        ));
    }
    entity
}

/// Sistema: «Getta a terra» dall'inventario, davanti ai piedi del giocatore.
#[allow(clippy::too_many_arguments)]
pub fn drop_requested_items(
    mut commands: Commands,
    mut ui: ResMut<crate::inventory_ui::InventoryUi>,
    mut inventory: ResMut<Inventory>,
    mut equip: ResMut<Equipment>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    visuals: Query<&GlobalTransform, With<crate::player::PlayerVisual>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(kind) = ui.drop_request.take() else {
        return;
    };
    let Ok(player) = players.single() else {
        return;
    };
    if inventory.qty(kind) == 0 {
        return;
    }
    let ahead = visuals
        .single()
        .map_or(*player.forward(), |v| *v.forward())
        .with_y(0.0)
        .normalize_or_zero();
    let at = player.translation() + ahead * 1.0 - Vec3::Y * 0.6;
    inventory.consume(kind, 1);
    equip.drop_missing(&inventory);
    spawn_ground_item(
        &mut commands,
        &mut meshes,
        &mut materials,
        kind,
        1,
        at,
        true,
    );
    info!("[INVENTORY] gettato a terra: {}", kind.name());
}

/// Sistema di raccolta: se il player è vicino a un Pickup (<= 2.0 m),
/// lo aggiunge all'inventario e despawna l'entità (raccolta automatica al
/// passaggio, stile action-RPG).
pub fn collect_pickups(
    mut commands: Commands,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    pickups: Query<(Entity, &GlobalTransform, &Pickup)>,
    mut inventory: ResMut<Inventory>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();

    for (entity, t, pickup) in &pickups {
        if !pickup.dropped && t.translation().distance(p) <= 2.0 {
            inventory.add(pickup.kind, pickup.qty);
            commands.entity(entity).despawn();
            info!("raccolto {:?} x{}", pickup.kind, pickup.qty);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn keepsakes_are_found_by_name_and_stay_out_of_the_hotbar() {
        for r in Relic::ALL {
            let k = ItemKind::Relic(r);
            assert_eq!(ItemKind::from_name(k.name()), Some(k), "{r:?}");
            assert!(ItemKind::ALL.contains(&k));
            assert!(k.weight_kg().is_some_and(|w| w < 0.5));
        }
        assert!(!ItemKind::Relic(Relic::Tablet).can_go_in_hotbar());
        assert!(ItemKind::Relic(Relic::Focaccia).can_go_in_hotbar());
        assert_eq!(
            ItemKind::Relic(Relic::Tablet).category(),
            Category::Keepsakes
        );
        assert_eq!(ItemKind::Relic(Relic::FeatherAmulet).charm_kg(), Some(5.0));
        assert_eq!(ItemKind::Relic(Relic::Focaccia).heal(), Some(30.0));
    }

    use super::*;

    #[test]
    fn names_accept_singular_and_plural() {
        assert_eq!(ItemKind::from_name("frecce"), Some(ItemKind::Arrow));
        assert_eq!(ItemKind::from_name("freccia"), Some(ItemKind::Arrow));
        assert_eq!(ItemKind::from_name("mela"), Some(ItemKind::Apple));
        assert_eq!(ItemKind::from_name("boh"), None);
    }

    #[test]
    fn coins_and_ammo_have_no_weight_the_rest_does() {
        for k in ItemKind::ALL {
            match k {
                ItemKind::Coin | ItemKind::Arrow => assert!(k.weight_kg().is_none()),
                _ => assert!(k.weight_kg().unwrap() > 0.0, "{k:?}"),
            }
        }
    }

    #[test]
    fn total_weight_ignores_coins_and_ammo() {
        let mut i = Inventory::default();
        i.add(ItemKind::Arrow, 20);
        i.add(ItemKind::Apple, 2);
        assert!((i.total_weight() - 0.3).abs() < 1e-5);
    }

    #[test]
    fn coins_never_enter_the_hotbar() {
        let mut i = Inventory::default();
        i.add(ItemKind::Coin, 50);
        assert!(i.hotbar_position(ItemKind::Coin).is_none());
        i.assign_hotbar(ItemKind::Coin, Some(2));
        assert!(i.hotbar_position(ItemKind::Coin).is_none());
    }

    #[test]
    fn new_items_take_free_keys_and_finished_ones_free_them() {
        let mut i = Inventory::default();
        i.add(ItemKind::Apple, 1);
        i.add(ItemKind::Bow, 1);
        assert_eq!(i.hotbar_position(ItemKind::Apple), Some(0));
        assert_eq!(i.hotbar_position(ItemKind::Bow), Some(1));
        i.consume(ItemKind::Apple, 1);
        assert_eq!(i.hotbar[0], None);
        i.add(ItemKind::Fish, 1);
        assert_eq!(i.hotbar_position(ItemKind::Fish), Some(0));
    }

    #[test]
    fn assigning_to_an_occupied_key_swaps() {
        let mut i = Inventory::default();
        i.add(ItemKind::Apple, 1);
        i.add(ItemKind::Bow, 1);
        i.assign_hotbar(ItemKind::Bow, Some(0));
        assert_eq!(i.hotbar_position(ItemKind::Bow), Some(0));
        assert_eq!(i.hotbar_position(ItemKind::Apple), Some(1));
        i.assign_hotbar(ItemKind::Apple, None);
        assert_eq!(i.hotbar_position(ItemKind::Apple), None);
        // Un'altra aggiunta non lo rimette in barra da solo.
        i.add(ItemKind::Apple, 1);
        assert_eq!(i.hotbar_position(ItemKind::Apple), None);
    }

    #[test]
    fn every_item_has_its_icon_file() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures/icone");
        for k in ItemKind::ALL {
            assert!(dir.join(format!("{}.png", k.icon())).exists(), "{k:?}");
        }
    }
}

// --- Icona sopra gli oggetti gettati ---------------------------------------------------------

/// Segnalino: l'oggetto a terra ha già la sua icona.
#[derive(Component)]
pub struct HasIcon;

/// Icona che guarda sempre verso la camera.
#[derive(Component)]
pub struct PickupIcon;

/// Sistema: sopra ogni oggetto gettato dal giocatore compare l'icona, come nella barra rapida.
#[allow(clippy::type_complexity)]
pub fn attach_pickup_icons(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    items: Query<(Entity, &Pickup), (Changed<Pickup>, Without<HasIcon>)>,
    mut quad: Local<Option<Handle<Mesh>>>,
) {
    for (entity, pickup) in &items {
        if !pickup.dropped {
            continue;
        }
        let mesh = quad
            .get_or_insert_with(|| meshes.add(Rectangle::new(0.55, 0.55)))
            .clone();
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(
                server.load(format!("textures/icone/{}.png", pickup.kind.icon())),
            ),
            unlit: true,
            alpha_mode: AlphaMode::Mask(0.35),
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        commands.entity(entity).insert(HasIcon).with_children(|c| {
            c.spawn((
                PickupIcon,
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, 0.55, 0.0),
                bevy::light::NotShadowCaster,
            ));
        });
    }
}

/// Sistema: le icone si girano verso la camera e ondeggiano appena.
pub fn face_pickup_icons(
    time: Res<Time>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut icons: Query<(&GlobalTransform, &ChildOf, &mut Transform), With<PickupIcon>>,
    parents: Query<&GlobalTransform, Without<PickupIcon>>,
) {
    let Ok(cam) = cameras.single() else {
        return;
    };
    for (global, parent, mut tf) in &mut icons {
        let Ok(parent_t) = parents.get(parent.parent()) else {
            continue;
        };
        let to = cam.translation() - global.translation();
        let yaw = to.x.atan2(to.z);
        tf.rotation = parent_t.rotation().inverse() * Quat::from_rotation_y(yaw);
        tf.translation.y = 0.55 + 0.04 * (time.elapsed_secs() * 2.0).sin();
    }
}
