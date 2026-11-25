// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Limiteur de débit en mémoire (fenêtre glissante) : freine le bourrage de mots de passe et le spam de signaux.
//! Les adresses IP ne servent qu'ici, en mémoire ; elles ne sont jamais écrites dans la base.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct Limiteur {
    essais: Mutex<HashMap<String, Vec<u64>>>,
}

pub fn maintenant() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Limiteur {
    /// Enregistre un essai pour `cle` et dit s'il est permis (au plus `max` essais par `fenetre_s` secondes).
    pub fn autoriser(&self, cle: &str, max: usize, fenetre_s: u64) -> bool {
        self.autoriser_a(cle, max, fenetre_s, maintenant())
    }

    pub fn autoriser_a(&self, cle: &str, max: usize, fenetre_s: u64, t: u64) -> bool {
        let mut carte = self.essais.lock().unwrap_or_else(|e| e.into_inner());
        if carte.len() > 20_000 {
            carte.retain(|_, v| v.last().map(|d| t.saturating_sub(*d) < 3600).unwrap_or(false));
        }
        let liste = carte.entry(cle.to_string()).or_default();
        liste.retain(|d| t.saturating_sub(*d) < fenetre_s);
        if liste.len() >= max {
            return false;
        }
        liste.push(t);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenetre_glissante() {
        let l = Limiteur::default();
        assert!(l.autoriser_a("a", 2, 60, 1000));
        assert!(l.autoriser_a("a", 2, 60, 1010));
        assert!(!l.autoriser_a("a", 2, 60, 1020), "3e essai dans la fenêtre : refusé");
        assert!(l.autoriser_a("b", 2, 60, 1020), "autre clé : indépendante");
        assert!(l.autoriser_a("a", 2, 60, 1061), "le 1er essai est sorti de la fenêtre");
    }
}
