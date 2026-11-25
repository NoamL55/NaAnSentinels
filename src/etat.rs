// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! État local de cette installation, rangé dans le dossier de données de l'utilisateur (jamais dans le dépôt) :
//! identifiant aléatoire, choix du signal, compte éventuel, compteurs d'analyses, vue préférée.
//! Le jeton de session y est stocké tel quel : il vaut pour ce compte NaAnSentinels seulement, pas un mot de passe.

use std::fs;
use std::path::PathBuf;

use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::engine::verdict::Niveau;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Compteurs {
    pub total: u64,
    pub sain: u64,
    pub a_regarder: u64,
    pub suspect: u64,
    pub dangereux: u64,
}

impl Compteurs {
    pub fn compter(&mut self, n: Niveau) {
        self.total += 1;
        match n {
            Niveau::Sain => self.sain += 1,
            Niveau::ARegarder => self.a_regarder += 1,
            Niveau::Suspect => self.suspect += 1,
            Niveau::Dangereux => self.dangereux += 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EtatLocal {
    /// Identifiant aléatoire de cette installation (32 caractères hexadécimaux). Ne dit rien de toi.
    pub install_id: String,
    /// Signal de présence : DÉSACTIVÉ par défaut. Rien ne part tant que tu ne l'actives pas.
    pub signal_actif: bool,
    pub serveur: String,
    pub pseudo: Option<String>,
    pub jeton: Option<String>,
    pub compteurs: Compteurs,
    pub vue_experte: bool,
}

impl Default for EtatLocal {
    fn default() -> Self {
        EtatLocal {
            install_id: nouvel_identifiant(),
            signal_actif: false,
            serveur: String::new(),
            pseudo: None,
            jeton: None,
            compteurs: Compteurs::default(),
            vue_experte: false,
        }
    }
}

pub fn nouvel_identifiant() -> String {
    let mut o = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut o);
    o.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Dossier de données de l'utilisateur : %APPDATA%\NaAnSentinels sous Windows, ~/.config/naansentinels ailleurs.
pub fn dossier_donnees() -> PathBuf {
    if let Ok(d) = std::env::var("NAANSENTINELS_DONNEES") {
        return PathBuf::from(d);
    }
    if let Ok(a) = std::env::var("APPDATA") {
        return PathBuf::from(a).join("NaAnSentinels");
    }
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(x).join("naansentinels");
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config").join("naansentinels")
}

impl EtatLocal {
    pub fn charger() -> EtatLocal {
        Self::charger_depuis(&dossier_donnees().join("etat.json"))
    }

    pub fn charger_depuis(chemin: &std::path::Path) -> EtatLocal {
        let mut e: EtatLocal = fs::read_to_string(chemin).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        if e.install_id.len() != 32 || !e.install_id.chars().all(|c| c.is_ascii_hexdigit()) {
            e.install_id = nouvel_identifiant();
        }
        e
    }

    pub fn sauver(&self) {
        self.sauver_dans(&dossier_donnees().join("etat.json"));
    }

    pub fn sauver_dans(&self, chemin: &std::path::Path) {
        if let Some(d) = chemin.parent() {
            let _ = fs::create_dir_all(d);
        }
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let _ = fs::write(chemin, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_desactive_par_defaut_et_identifiant_aleatoire() {
        let a = EtatLocal::default();
        let b = EtatLocal::default();
        assert!(!a.signal_actif, "le signal doit être désactivé par défaut");
        assert_eq!(a.install_id.len(), 32);
        assert_ne!(a.install_id, b.install_id);
        assert!(a.jeton.is_none() && a.pseudo.is_none());
    }

    #[test]
    fn aller_retour_disque_et_compteurs() {
        let chemin = std::env::temp_dir().join(format!("naansentinels-etat-{}", std::process::id())).join("etat.json");
        let mut e = EtatLocal::default();
        e.compteurs.compter(Niveau::Sain);
        e.compteurs.compter(Niveau::Dangereux);
        e.sauver_dans(&chemin);
        let relu = EtatLocal::charger_depuis(&chemin);
        assert_eq!(relu.install_id, e.install_id);
        assert_eq!((relu.compteurs.total, relu.compteurs.sain, relu.compteurs.dangereux), (2, 1, 1));
        // Fichier abîmé : on repart d'un état sain, sans planter.
        std::fs::write(&chemin, "{pas du json").unwrap();
        assert_eq!(EtatLocal::charger_depuis(&chemin).install_id.len(), 32);
    }
}
