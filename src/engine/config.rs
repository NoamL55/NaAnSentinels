// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Configuration éditable (config.toml). Si un champ manque, on prend une valeur par défaut : jamais de plantage.

use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Analyse {
    pub entropie_suspecte: f64,
    pub taille_max_mo: usize,
    pub instructions_desassemblees: usize,
}
impl Default for Analyse {
    fn default() -> Self {
        Analyse { entropie_suspecte: 7.2, taille_max_mo: 300, instructions_desassemblees: 60 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Interface {
    pub theme: String,
}
impl Default for Interface {
    fn default() -> Self {
        Interface { theme: "sombre".into() }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Mail {
    pub serveur_imap: String,
    pub port: u16,
    pub adresse: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub analyse: Analyse,
    pub interface: Interface,
    pub mail: Mail,
}

impl Config {
    /// Charge la config ; si le fichier est absent ou illisible, renvoie les valeurs par défaut.
    pub fn charger(chemin: &Path) -> Config {
        fs::read_to_string(chemin)
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }
}
