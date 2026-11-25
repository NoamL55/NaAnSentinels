// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Règles de détection, ouvertes et éditables (rules/signatures.toml).
//! Trois leviers : signatures (motifs), imports suspects (fonctions dangereuses), chaînes suspectes.

use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Signature {
    pub nom: String,
    #[serde(rename = "type")]
    pub type_: String, // "chaine" | "hex" | "sha256"
    pub motif: String,
    pub gravite: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportSuspect {
    pub nom: String,
    pub gravite: String,
    pub raison: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChaineSuspecte {
    pub motif: String,
    pub gravite: String,
    pub raison: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Regles {
    #[serde(default, rename = "signature")]
    pub signatures: Vec<Signature>,
    #[serde(default, rename = "import_suspect")]
    pub imports_suspects: Vec<ImportSuspect>,
    #[serde(default, rename = "chaine_suspecte")]
    pub chaines_suspectes: Vec<ChaineSuspecte>,
}

impl Regles {
    /// Charge les règles depuis un fichier TOML. En cas d'erreur, renvoie le message (affiché dans l'app).
    pub fn charger(chemin: &Path) -> Result<Regles, String> {
        let txt = fs::read_to_string(chemin)
            .map_err(|e| format!("Lecture des règles ({}) impossible : {e}", chemin.display()))?;
        toml::from_str(&txt).map_err(|e| format!("Règles invalides : {e}"))
    }

    pub fn total(&self) -> usize {
        self.signatures.len() + self.imports_suspects.len() + self.chaines_suspectes.len()
    }
}
