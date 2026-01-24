//! Name Generator Module
//!
//! Generates culturally-appropriate names for TTRPG characters and NPCs.

use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Name culture/origin
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameCulture {
    // Fantasy cultures
    Elvish,
    Dwarvish,
    Orcish,
    Halfling,
    Gnomish,
    Draconic,
    Infernal,
    Celestial,
    // Human-inspired cultures
    Nordic,
    Celtic,
    Greek,
    Roman,
    Arabic,
    Japanese,
    Chinese,
    African,
    Slavic,
    Germanic,
    // Generic
    Common,
    Fantasy,
}

/// Name gender
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameGender {
    Male,
    Female,
    Neutral,
}

/// Name type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum NameType {
    #[default]
    FirstName,
    LastName,
    FullName,
    Title,
    Epithet,
    PlaceName,
    TavernName,
    ShopName,
}

/// Generated name result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedName {
    pub name: String,
    pub culture: NameCulture,
    pub gender: NameGender,
    pub name_type: NameType,
    pub meaning: Option<String>,
}

/// Name generation options
#[derive(Debug, Clone, Default)]
pub struct NameOptions {
    pub culture: Option<NameCulture>,
    pub gender: Option<NameGender>,
    pub name_type: NameType,
    pub include_meaning: bool,
    pub syllable_count: Option<usize>,
}


// ============================================================================
// Name Components
// ============================================================================

struct NameComponents {
    prefixes: Vec<&'static str>,
    middles: Vec<&'static str>,
    suffixes_male: Vec<&'static str>,
    suffixes_female: Vec<&'static str>,
    suffixes_neutral: Vec<&'static str>,
}

fn get_components(culture: &NameCulture) -> NameComponents {
    match culture {
        NameCulture::Elvish => NameComponents {
            prefixes: vec![
                "Ael", "Aer", "Ama", "Ara", "Cal", "Cel", "Eil", "Fae", "Gal", "Ith",
                "Lar", "Lue", "Mae", "Nar", "Nil", "Pha", "Qui", "Ral", "Sae", "Thi",
                "Ula", "Val", "Zan", "Ael", "Syl", "Fen", "Mir", "Lor", "Tha", "Eli",
            ],
            middles: vec![
                "an", "ar", "el", "en", "il", "in", "ol", "or", "ae", "ai",
                "ia", "ie", "io", "iu", "ea", "ei", "eo", "au", "ui", "oa",
                "ri", "li", "ni", "si", "thi", "dri", "ven", "wen", "lyn", "dor",
            ],
            suffixes_male: vec![
                "ion", "ior", "ias", "ian", "ius", "ael", "orn", "dor", "mir", "vin",
                "wen", "ric", "las", "ran", "sar", "thas", "ron", "lin", "vel", "nar",
            ],
            suffixes_female: vec![
                "iel", "iel", "ia", "ira", "ara", "wen", "wyn", "eth", "ith", "ael",
                "riel", "diel", "liel", "niel", "thiel", "wen", "lyn", "ris", "nis", "lis",
            ],
            suffixes_neutral: vec![
                "iel", "ien", "ael", "wen", "lyn", "rin", "sin", "thin", "ven", "ean",
            ],
        },
        NameCulture::Dwarvish => NameComponents {
            prefixes: vec![
                "Bal", "Bar", "Bor", "Bru", "Dag", "Dar", "Dor", "Dun", "Dur", "Gar",
                "Gim", "Gor", "Grim", "Kaz", "Kil", "Kor", "Mor", "Nar", "Nor", "Rag",
                "Rog", "Rud", "Thol", "Thor", "Thro", "Tor", "Trag", "Ul", "Ur", "Zan",
            ],
            middles: vec![
                "ag", "ak", "al", "an", "ar", "az", "og", "ok", "ol", "on",
                "or", "oz", "ug", "uk", "ul", "un", "ur", "uz", "im", "in",
            ],
            suffixes_male: vec![
                "in", "im", "ak", "ok", "ik", "uk", "rim", "grim", "din", "dak",
                "gar", "gor", "dur", "dor", "bar", "bor", "rik", "rok", "thak", "zhak",
            ],
            suffixes_female: vec![
                "a", "ra", "da", "na", "la", "ris", "dis", "hild", "run", "wyn",
                "lin", "rin", "din", "min", "vin", "dris", "gris", "thra", "zhara", "kara",
            ],
            suffixes_neutral: vec![
                "in", "an", "on", "ar", "or", "ak", "ok", "rim", "dim", "lin",
            ],
        },
        NameCulture::Orcish => NameComponents {
            prefixes: vec![
                "Azg", "Bol", "Bru", "Dug", "Gar", "Gol", "Gom", "Gor", "Gra", "Gro",
                "Gul", "Kra", "Kro", "Kru", "Lur", "Mog", "Mor", "Mug", "Mur", "Naz",
                "Rak", "Rog", "Ruk", "Sha", "Shag", "Skar", "Sko", "Sna", "Ug", "Urk",
            ],
            middles: vec![
                "ag", "ak", "ar", "ash", "at", "az", "og", "ok", "or", "osh",
                "ot", "oz", "ug", "uk", "ur", "ush", "ut", "uz", "ra", "ro",
            ],
            suffixes_male: vec![
                "ak", "ash", "ath", "az", "gash", "gath", "gaz", "gor", "goth", "gul",
                "kash", "kath", "mash", "math", "nash", "nath", "rak", "rash", "zog", "zul",
            ],
            suffixes_female: vec![
                "a", "ra", "sha", "ka", "ga", "tha", "gra", "kra", "shra", "tra",
                "na", "la", "za", "ba", "da", "va", "ma", "sa", "ja", "ha",
            ],
            suffixes_neutral: vec![
                "ak", "ash", "az", "og", "uk", "ush", "ra", "za", "ga", "sha",
            ],
        },
        NameCulture::Halfling => NameComponents {
            prefixes: vec![
                "Bil", "Bun", "Cor", "Dar", "Dro", "Fal", "Fin", "Fro", "Hal", "Hob",
                "Jas", "Jol", "Lar", "Lav", "Lil", "Mer", "Mil", "Ned", "Pip", "Pod",
                "Ros", "Sam", "Tan", "Ted", "Til", "Tom", "Wil", "Yas", "Zin", "Zol",
            ],
            middles: vec![
                "a", "e", "i", "o", "u", "an", "en", "in", "on", "un",
                "ar", "er", "ir", "or", "ur", "al", "el", "il", "ol", "ul",
            ],
            suffixes_male: vec![
                "bo", "co", "do", "fo", "go", "ho", "ko", "lo", "mo", "no",
                "po", "ro", "so", "to", "wo", "ric", "lin", "win", "bin", "din",
            ],
            suffixes_female: vec![
                "a", "bella", "ella", "ina", "ita", "ola", "ula", "wyn", "lyn", "ris",
                "lis", "nis", "dis", "bis", "tis", "la", "na", "ra", "sa", "ta",
            ],
            suffixes_neutral: vec![
                "by", "cy", "dy", "fy", "gy", "ky", "ly", "my", "ny", "ry",
            ],
        },
        NameCulture::Nordic => NameComponents {
            prefixes: vec![
                "Ag", "Alf", "Ar", "Bj", "Bor", "Bra", "Dag", "Eir", "Frey", "Grim",
                "Gu", "Hal", "Har", "Hel", "Ing", "Iv", "Jor", "Kar", "Leif", "Mag",
                "Odd", "Ol", "Rag", "Sig", "Sven", "Thor", "Tor", "Ulf", "Val", "Vig",
            ],
            middles: vec![
                "a", "e", "i", "o", "u", "ar", "er", "ir", "or", "ur",
                "al", "el", "il", "ol", "ul", "an", "en", "in", "on", "un",
            ],
            suffixes_male: vec![
                "ar", "bjorn", "din", "dor", "fast", "geir", "grim", "heim", "leif", "mar",
                "mund", "nar", "nir", "olf", "orn", "rik", "stein", "ulf", "valdr", "var",
            ],
            suffixes_female: vec![
                "a", "dis", "frid", "gerd", "gund", "hild", "run", "borg", "laug", "rid",
                "vig", "vor", "wyn", "ya", "ja", "na", "ra", "sa", "va", "wa",
            ],
            suffixes_neutral: vec![
                "i", "e", "a", "ar", "ir", "ur", "in", "an", "on", "en",
            ],
        },
        NameCulture::Celtic => NameComponents {
            prefixes: vec![
                "Aeth", "Aod", "Bran", "Bren", "Cad", "Conn", "Corm", "Cu", "Diar", "Don",
                "Eog", "Fer", "Fin", "Gael", "Gorm", "Kev", "Lugh", "Mael", "Niall", "Ois",
                "Pad", "Rian", "Sean", "Tal", "Tier", "Tor", "Tuath", "Uch", "Uil", "Wen",
            ],
            middles: vec![
                "a", "ae", "ai", "ea", "ei", "ia", "ie", "io", "ua", "ue",
                "an", "en", "in", "on", "ar", "er", "ir", "or", "al", "el",
            ],
            suffixes_male: vec![
                "an", "in", "on", "ius", "mac", "mael", "og", "ric", "val", "wyn",
                "ard", "orn", "ulf", "gar", "mar", "dan", "don", "fin", "gin", "lin",
            ],
            suffixes_female: vec![
                "a", "een", "enn", "id", "in", "na", "ne", "wen", "wyn", "gwen",
                "eth", "ith", "oth", "uth", "aith", "eith", "iath", "nia", "sia", "tia",
            ],
            suffixes_neutral: vec![
                "an", "en", "in", "on", "un", "wyn", "wen", "eth", "ith", "ath",
            ],
        },
        NameCulture::Greek => NameComponents {
            prefixes: vec![
                "Ach", "Aer", "Agam", "Ajax", "Alex", "Andr", "Apo", "Ares", "Artem", "Athen",
                "Dem", "Dion", "Elek", "Eur", "Hect", "Hel", "Her", "Hom", "Jas", "Leon",
                "Lys", "Nic", "Od", "Orph", "Per", "Phil", "Plat", "Soph", "Thes", "Xen",
            ],
            middles: vec![
                "a", "e", "i", "o", "u", "ae", "ai", "au", "ei", "eu",
                "oi", "ou", "an", "en", "on", "as", "es", "is", "os", "us",
            ],
            suffixes_male: vec![
                "ander", "as", "cles", "doros", "es", "eus", "goras", "ias", "icles", "ios",
                "ius", "on", "or", "os", "seus", "sthenes", "theus", "us", "xios", "zes",
            ],
            suffixes_female: vec![
                "a", "aia", "ane", "anthe", "eia", "ela", "ene", "ia", "ina", "ione",
                "issa", "ope", "ora", "ossa", "phe", "sia", "sta", "stra", "tia", "yne",
            ],
            suffixes_neutral: vec![
                "is", "ys", "os", "as", "es", "us", "on", "an", "en", "in",
            ],
        },
        NameCulture::Arabic => NameComponents {
            prefixes: vec![
                "Ab", "Ah", "Al", "Am", "As", "Az", "Ba", "Da", "Fa", "Ha",
                "Ib", "Ja", "Ka", "Kha", "Ma", "Mu", "Na", "Qa", "Ra", "Sa",
                "Sha", "Sul", "Ta", "Wa", "Ya", "Za", "Zay", "Zul", "Nas", "Nur",
            ],
            middles: vec![
                "a", "i", "u", "aa", "ee", "ii", "oo", "uu", "ai", "au",
                "ad", "af", "al", "am", "an", "ar", "as", "at", "az", "ir",
            ],
            suffixes_male: vec![
                "ad", "af", "ah", "al", "am", "an", "ar", "as", "at", "az",
                "bir", "dir", "fir", "hir", "im", "ir", "is", "it", "man", "rid",
            ],
            suffixes_female: vec![
                "a", "ah", "ala", "ana", "ara", "aya", "eela", "eema", "eena", "eera",
                "ifa", "ika", "ila", "ima", "ina", "ira", "isa", "ita", "iya", "iza",
            ],
            suffixes_neutral: vec![
                "i", "a", "an", "ir", "ar", "am", "at", "as", "ad", "ah",
            ],
        },
        NameCulture::Japanese => NameComponents {
            prefixes: vec![
                "Aki", "Asa", "Haru", "Hiro", "Ichi", "Ishi", "Kage", "Kaze", "Ken", "Kuro",
                "Masa", "Mizu", "Naka", "Nobu", "Rai", "Ryu", "Saku", "Shin", "Taka", "Teru",
                "Tomo", "Tsuki", "Yama", "Yoshi", "Yuki", "Aki", "Hana", "Kiku", "Sora", "Umi",
            ],
            middles: vec![
                "a", "e", "i", "o", "u", "ka", "ke", "ki", "ko", "ku",
                "ma", "me", "mi", "mo", "mu", "na", "ne", "ni", "no", "nu",
            ],
            suffixes_male: vec![
                "hiko", "hiro", "ichi", "ji", "kazu", "ki", "maru", "masa", "moto", "nobu",
                "nori", "o", "ro", "shi", "suke", "ta", "taka", "to", "ya", "zo",
            ],
            suffixes_female: vec![
                "e", "i", "ka", "ki", "ko", "me", "mi", "na", "ne", "no",
                "ra", "re", "ri", "sa", "se", "shi", "yo", "yu", "yuki", "ka",
            ],
            suffixes_neutral: vec![
                "i", "u", "ki", "mi", "ri", "shi", "chi", "hi", "ni", "ji",
            ],
        },
        NameCulture::Draconic => NameComponents {
            prefixes: vec![
                "Ach", "Arj", "Bal", "Bel", "Cax", "Dra", "Fyr", "Gal", "Irx", "Jex",
                "Kaz", "Kri", "Lor", "Maz", "Nax", "Orx", "Pax", "Raz", "Sax", "Thra",
                "Tyx", "Urx", "Vax", "Wrax", "Xar", "Yax", "Zar", "Zex", "Zor", "Zyx",
            ],
            middles: vec![
                "a", "ax", "ex", "ix", "ox", "ux", "ar", "er", "ir", "or",
                "ur", "ash", "esh", "ish", "osh", "ush", "ath", "eth", "ith", "oth",
            ],
            suffixes_male: vec![
                "ax", "arax", "ex", "ix", "or", "ox", "rax", "rex", "rix", "rox",
                "tharax", "thex", "thix", "thox", "thrax", "ux", "vax", "xar", "zar", "zex",
            ],
            suffixes_female: vec![
                "ara", "arix", "era", "erix", "ira", "irix", "ora", "orix", "ura", "urix",
                "yra", "yrix", "xia", "xira", "zara", "zira", "thira", "thra", "shira", "shra",
            ],
            suffixes_neutral: vec![
                "ax", "ex", "ix", "ox", "ux", "ar", "er", "ir", "or", "ur",
            ],
        },
        NameCulture::Infernal => NameComponents {
            prefixes: vec![
                "Asm", "Baal", "Bel", "Dis", "Gla", "Lev", "Mam", "Mol", "Nab", "Orcus",
                "Raz", "Tan", "Zag", "Zer", "Baz", "Crom", "Draz", "Graz", "Kraz", "Vraz",
                "Xaz", "Yraz", "Abr", "Mal", "Vel", "Zul", "Gor", "Mor", "Sor", "Tor",
            ],
            middles: vec![
                "a", "e", "i", "o", "u", "az", "ez", "iz", "oz", "uz",
                "ar", "er", "ir", "or", "ur", "as", "es", "is", "os", "us",
            ],
            suffixes_male: vec![
                "amon", "as", "athon", "eus", "gor", "ion", "ius", "oth", "thas", "us",
                "xus", "zar", "zul", "baal", "roth", "thul", "goth", "mael", "rael", "zael",
            ],
            suffixes_female: vec![
                "a", "ia", "ith", "ix", "yth", "yx", "ara", "era", "ira", "ora",
                "ura", "essa", "issa", "ossa", "ussa", "aith", "eith", "oith", "rath", "rith",
            ],
            suffixes_neutral: vec![
                "us", "os", "as", "is", "uth", "oth", "ath", "eth", "ith", "ix",
            ],
        },
        NameCulture::Celestial => NameComponents {
            prefixes: vec![
                "Aur", "Cel", "Div", "Elar", "Gal", "Hal", "Lum", "Mir", "Nal", "Ori",
                "Pax", "Rad", "Ser", "Sol", "Ura", "Zel", "Aer", "Bri", "Cas", "Dae",
                "Ely", "Fay", "Glo", "Hel", "Iri", "Jov", "Kyr", "Lyr", "Myr", "Nyx",
            ],
            middles: vec![
                "a", "ae", "ai", "au", "ea", "ei", "ia", "ie", "io", "iu",
                "oa", "oe", "oi", "ua", "ue", "ui", "el", "al", "il", "ol",
            ],
            suffixes_male: vec![
                "ael", "ariel", "el", "iel", "ion", "ius", "or", "riel", "thiel", "uel",
                "xiel", "ziel", "anael", "uriel", "ophiel", "adiel", "emiel", "oniel", "udiel", "yriel",
            ],
            suffixes_female: vec![
                "a", "ella", "ia", "iel", "ina", "ira", "issa", "ita", "ola", "ora",
                "yra", "aria", "elia", "ilia", "olia", "ulia", "ania", "enia", "inia", "onia",
            ],
            suffixes_neutral: vec![
                "iel", "el", "al", "il", "ol", "ael", "uel", "eal", "ial", "oel",
            ],
        },
        _ => NameComponents {
            prefixes: vec![
                "Ar", "Bel", "Cal", "Dar", "El", "Fal", "Gar", "Hal", "Il", "Jar",
                "Kal", "Lar", "Mar", "Nar", "Or", "Par", "Qar", "Rar", "Sar", "Tar",
                "Ur", "Val", "War", "Xar", "Yar", "Zar", "Aer", "Bor", "Cor", "Dor",
            ],
            middles: vec![
                "a", "e", "i", "o", "u", "an", "en", "in", "on", "un",
                "ar", "er", "ir", "or", "ur", "al", "el", "il", "ol", "ul",
            ],
            suffixes_male: vec![
                "an", "ar", "as", "en", "er", "es", "in", "ir", "is", "on",
                "or", "os", "un", "ur", "us", "ric", "vin", "win", "lin", "din",
            ],
            suffixes_female: vec![
                "a", "ana", "ara", "ela", "ena", "era", "ina", "ira", "isa", "ona",
                "ora", "osa", "una", "ura", "usa", "lyn", "wyn", "eth", "ith", "ath",
            ],
            suffixes_neutral: vec![
                "an", "en", "in", "on", "un", "ar", "er", "ir", "or", "ur",
            ],
        },
    }
}

// ============================================================================
// Place Name Components
// ============================================================================

fn get_place_prefixes() -> Vec<&'static str> {
    vec![
        "Black", "White", "Red", "Green", "Blue", "Silver", "Gold", "Iron", "Storm", "Thunder",
        "Dragon", "Wolf", "Bear", "Eagle", "Raven", "Lion", "Serpent", "Shadow", "Sun", "Moon",
        "Star", "High", "Low", "North", "South", "East", "West", "Old", "New", "Lost",
        "Dark", "Bright", "Crystal", "Frost", "Fire", "Stone", "Wood", "River", "Lake", "Sea",
    ]
}

fn get_place_suffixes() -> Vec<&'static str> {
    vec![
        "haven", "hold", "keep", "castle", "tower", "gate", "bridge", "ford", "dale", "vale",
        "wood", "forest", "field", "meadow", "hill", "mount", "peak", "cliff", "crag", "stone",
        "water", "falls", "spring", "well", "marsh", "fen", "moor", "heath", "shire", "land",
        "town", "burg", "borough", "ville", "port", "bay", "cove", "isle", "reach", "hollow",
    ]
}

fn get_tavern_adjectives() -> Vec<&'static str> {
    vec![
        "Golden", "Silver", "Rusty", "Gilded", "Drunken", "Merry", "Weary", "Prancing", "Dancing", "Sleeping",
        "Howling", "Laughing", "Crying", "Roaring", "Whispering", "Silent", "Noisy", "Dusty", "Shiny", "Broken",
        "Lucky", "Unlucky", "Happy", "Grumpy", "Jolly", "Wicked", "Noble", "Humble", "Proud", "Shy",
    ]
}

fn get_tavern_nouns() -> Vec<&'static str> {
    vec![
        "Dragon", "Griffin", "Unicorn", "Phoenix", "Pegasus", "Basilisk", "Hydra", "Chimera", "Manticore", "Sphinx",
        "Wolf", "Bear", "Lion", "Tiger", "Boar", "Stag", "Raven", "Eagle", "Owl", "Serpent",
        "Knight", "King", "Queen", "Prince", "Princess", "Wizard", "Witch", "Bard", "Monk", "Pilgrim",
        "Goblet", "Tankard", "Flagon", "Barrel", "Keg", "Sword", "Shield", "Crown", "Scepter", "Throne",
    ]
}

fn get_shop_types() -> Vec<&'static str> {
    vec![
        "Emporium", "Shoppe", "Trading Post", "Mercantile", "Provisions", "Supplies", "Goods", "Wares",
        "Armory", "Smithy", "Forge", "Workshop", "Atelier", "Boutique", "Market", "Bazaar",
    ]
}

// ============================================================================
// Name Generator
// ============================================================================

/// Generates culturally-appropriate names
pub struct NameGenerator {
    rng: rand::rngs::StdRng,
    name_meanings: HashMap<String, String>,
}

impl NameGenerator {
    pub fn new() -> Self {
        Self {
            rng: rand::rngs::StdRng::from_entropy(),
            name_meanings: Self::load_meanings(),
        }
    }

    /// Create with a specific seed for reproducible results
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            name_meanings: Self::load_meanings(),
        }
    }

    fn load_meanings() -> HashMap<String, String> {
        let mut meanings = HashMap::new();
        // Add some example meanings
        meanings.insert("ar".to_string(), "noble".to_string());
        meanings.insert("bel".to_string(), "beautiful".to_string());
        meanings.insert("cal".to_string(), "bright".to_string());
        meanings.insert("dar".to_string(), "dark".to_string());
        meanings.insert("el".to_string(), "star".to_string());
        meanings.insert("fal".to_string(), "falcon".to_string());
        meanings.insert("gar".to_string(), "spear".to_string());
        meanings.insert("thor".to_string(), "thunder".to_string());
        meanings.insert("val".to_string(), "strong".to_string());
        meanings.insert("wyn".to_string(), "friend".to_string());
        meanings
    }

    /// Generate a name with the given options
    pub fn generate(&mut self, options: &NameOptions) -> GeneratedName {
        let culture = options.culture.clone().unwrap_or(NameCulture::Fantasy);
        let gender = options.gender.clone().unwrap_or(NameGender::Neutral);

        match options.name_type {
            NameType::FirstName => self.generate_first_name(&culture, &gender, options),
            NameType::LastName => self.generate_last_name(&culture, options),
            NameType::FullName => self.generate_full_name(&culture, &gender, options),
            NameType::Title => self.generate_title(&gender),
            NameType::Epithet => self.generate_epithet(),
            NameType::PlaceName => self.generate_place_name(),
            NameType::TavernName => self.generate_tavern_name(),
            NameType::ShopName => self.generate_shop_name(),
        }
    }

    fn generate_first_name(
        &mut self,
        culture: &NameCulture,
        gender: &NameGender,
        options: &NameOptions,
    ) -> GeneratedName {
        let components = get_components(culture);

        let prefix = components.prefixes.choose(&mut self.rng).unwrap_or(&"A");

        let middle = if options.syllable_count.unwrap_or(2) > 1 {
            *components.middles.choose(&mut self.rng).unwrap_or(&"")
        } else {
            ""
        };

        let suffix = match gender {
            NameGender::Male => components.suffixes_male.choose(&mut self.rng).unwrap_or(&""),
            NameGender::Female => components.suffixes_female.choose(&mut self.rng).unwrap_or(&""),
            NameGender::Neutral => components.suffixes_neutral.choose(&mut self.rng).unwrap_or(&""),
        };

        let name = format!("{}{}{}", prefix, middle, suffix);
        let meaning = if options.include_meaning {
            self.name_meanings.get(&prefix.to_lowercase()).cloned()
        } else {
            None
        };

        GeneratedName {
            name,
            culture: culture.clone(),
            gender: gender.clone(),
            name_type: NameType::FirstName,
            meaning,
        }
    }

    fn generate_last_name(&mut self, culture: &NameCulture, options: &NameOptions) -> GeneratedName {
        let components = get_components(culture);

        let prefix = components.prefixes.choose(&mut self.rng).unwrap_or(&"A");
        let middle = *components.middles.choose(&mut self.rng).unwrap_or(&"");
        let suffix = components.suffixes_neutral.choose(&mut self.rng).unwrap_or(&"");

        let name = format!("{}{}{}", prefix, middle, suffix);
        let meaning = if options.include_meaning {
            self.name_meanings.get(&prefix.to_lowercase()).cloned()
        } else {
            None
        };

        GeneratedName {
            name,
            culture: culture.clone(),
            gender: NameGender::Neutral,
            name_type: NameType::LastName,
            meaning,
        }
    }

    fn generate_full_name(
        &mut self,
        culture: &NameCulture,
        gender: &NameGender,
        options: &NameOptions,
    ) -> GeneratedName {
        let first = self.generate_first_name(culture, gender, options);
        let last = self.generate_last_name(culture, options);

        let full_name = format!("{} {}", first.name, last.name);

        GeneratedName {
            name: full_name,
            culture: culture.clone(),
            gender: gender.clone(),
            name_type: NameType::FullName,
            meaning: first.meaning,
        }
    }

    fn generate_title(&mut self, gender: &NameGender) -> GeneratedName {
        let titles_male = vec![
            "Lord", "Sir", "Baron", "Count", "Duke", "Prince", "King", "Master", "Father", "Brother",
        ];
        let titles_female = vec![
            "Lady", "Dame", "Baroness", "Countess", "Duchess", "Princess", "Queen", "Mistress", "Mother", "Sister",
        ];
        let titles_neutral = vec![
            "Noble", "Sage", "Elder", "Keeper", "Guardian", "Warden", "Champion", "Hero", "Chosen", "Oracle",
        ];

        let title = match gender {
            NameGender::Male => *titles_male.choose(&mut self.rng).unwrap_or(&"Lord"),
            NameGender::Female => *titles_female.choose(&mut self.rng).unwrap_or(&"Lady"),
            NameGender::Neutral => *titles_neutral.choose(&mut self.rng).unwrap_or(&"Noble"),
        };

        GeneratedName {
            name: title.to_string(),
            culture: NameCulture::Common,
            gender: gender.clone(),
            name_type: NameType::Title,
            meaning: None,
        }
    }

    fn generate_epithet(&mut self) -> GeneratedName {
        let epithets = vec![
            "the Bold", "the Brave", "the Wise", "the Strong", "the Swift", "the Cunning",
            "the Just", "the Merciless", "the Gentle", "the Fierce", "the Silent", "the Loud",
            "the Fair", "the Dark", "the Bright", "the Shadow", "the Storm", "the Flame",
            "the Frost", "the Stone", "the Iron", "the Golden", "the Silver", "the Black",
            "the White", "the Red", "the Green", "the Blue", "the Ancient", "the Young",
            "Dragonslayer", "Giantbane", "Oathbreaker", "Kingmaker", "Shadowwalker", "Stormcaller",
            "Flamebringer", "Frostweaver", "Stoneheart", "Ironwill", "Goldeneye", "Silvertongue",
        ];

        let epithet = *epithets.choose(&mut self.rng).unwrap_or(&"the Bold");

        GeneratedName {
            name: epithet.to_string(),
            culture: NameCulture::Common,
            gender: NameGender::Neutral,
            name_type: NameType::Epithet,
            meaning: None,
        }
    }

    fn generate_place_name(&mut self) -> GeneratedName {
        let prefixes = get_place_prefixes();
        let suffixes = get_place_suffixes();

        let prefix = *prefixes.choose(&mut self.rng).unwrap_or(&"New");
        let suffix = *suffixes.choose(&mut self.rng).unwrap_or(&"town");

        let name = format!("{}{}", prefix, suffix);

        GeneratedName {
            name,
            culture: NameCulture::Common,
            gender: NameGender::Neutral,
            name_type: NameType::PlaceName,
            meaning: None,
        }
    }

    fn generate_tavern_name(&mut self) -> GeneratedName {
        let adjectives = get_tavern_adjectives();
        let nouns = get_tavern_nouns();

        let adjective = *adjectives.choose(&mut self.rng).unwrap_or(&"Golden");
        let noun = *nouns.choose(&mut self.rng).unwrap_or(&"Dragon");

        let name = format!("The {} {}", adjective, noun);

        GeneratedName {
            name,
            culture: NameCulture::Common,
            gender: NameGender::Neutral,
            name_type: NameType::TavernName,
            meaning: None,
        }
    }

    fn generate_shop_name(&mut self) -> GeneratedName {
        // Generate a proprietor name
        let first_options = NameOptions {
            culture: Some(NameCulture::Common),
            gender: Some(NameGender::Neutral),
            name_type: NameType::FirstName,
            include_meaning: false,
            syllable_count: Some(2),
        };
        let proprietor = self.generate(&first_options);

        let shop_types = get_shop_types();
        let shop_type = *shop_types.choose(&mut self.rng).unwrap_or(&"Shoppe");

        let name = format!("{}'s {}", proprietor.name, shop_type);

        GeneratedName {
            name,
            culture: NameCulture::Common,
            gender: NameGender::Neutral,
            name_type: NameType::ShopName,
            meaning: None,
        }
    }

    /// Generate multiple names
    pub fn generate_batch(&mut self, options: &NameOptions, count: usize) -> Vec<GeneratedName> {
        (0..count).map(|_| self.generate(options)).collect()
    }

    /// Generate a full NPC name with title and epithet
    pub fn generate_npc_name(
        &mut self,
        culture: &NameCulture,
        gender: &NameGender,
        include_title: bool,
        include_epithet: bool,
    ) -> String {
        let mut parts = Vec::new();

        if include_title {
            let title = self.generate_title(gender);
            parts.push(title.name);
        }

        let options = NameOptions {
            culture: Some(culture.clone()),
            gender: Some(gender.clone()),
            name_type: NameType::FullName,
            include_meaning: false,
            syllable_count: Some(2),
        };
        let full_name = self.generate(&options);
        parts.push(full_name.name);

        if include_epithet {
            let epithet = self.generate_epithet();
            parts.push(epithet.name);
        }

        parts.join(" ")
    }
}

impl Default for NameGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_first_name() {
        let mut gen = NameGenerator::with_seed(42);

        let options = NameOptions {
            culture: Some(NameCulture::Elvish),
            gender: Some(NameGender::Female),
            name_type: NameType::FirstName,
            include_meaning: false,
            syllable_count: Some(3),
        };

        let name = gen.generate(&options);
        assert!(!name.name.is_empty());
        assert_eq!(name.culture, NameCulture::Elvish);
        assert_eq!(name.gender, NameGender::Female);
    }

    #[test]
    fn test_generate_full_name() {
        let mut gen = NameGenerator::with_seed(42);

        let options = NameOptions {
            culture: Some(NameCulture::Dwarvish),
            gender: Some(NameGender::Male),
            name_type: NameType::FullName,
            include_meaning: true,
            syllable_count: Some(2),
        };

        let name = gen.generate(&options);
        assert!(name.name.contains(' ')); // Should have first and last name
    }

    #[test]
    fn test_generate_tavern_name() {
        let mut gen = NameGenerator::with_seed(42);

        let options = NameOptions {
            name_type: NameType::TavernName,
            ..Default::default()
        };

        let name = gen.generate(&options);
        assert!(name.name.starts_with("The "));
    }

    #[test]
    fn test_generate_batch() {
        let mut gen = NameGenerator::with_seed(42);

        let options = NameOptions {
            culture: Some(NameCulture::Nordic),
            gender: Some(NameGender::Male),
            name_type: NameType::FirstName,
            include_meaning: false,
            syllable_count: Some(2),
        };

        let names = gen.generate_batch(&options, 10);
        assert_eq!(names.len(), 10);
    }

    #[test]
    fn test_reproducible_with_seed() {
        let mut gen1 = NameGenerator::with_seed(12345);
        let mut gen2 = NameGenerator::with_seed(12345);

        let options = NameOptions {
            culture: Some(NameCulture::Orcish),
            gender: Some(NameGender::Male),
            name_type: NameType::FirstName,
            include_meaning: false,
            syllable_count: Some(2),
        };

        let name1 = gen1.generate(&options);
        let name2 = gen2.generate(&options);

        assert_eq!(name1.name, name2.name);
    }
}
