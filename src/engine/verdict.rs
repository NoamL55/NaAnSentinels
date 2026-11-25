// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Le verdict de NaAnSentinels : INFORMATIF. On ne dit jamais « bloqué » sans raison.
//! Chaque constat porte une explication (ce qu'on observe) ET un « pourquoi » (ce que ça implique, honnêtement).

/// Niveau d'un constat ou d'un rapport. Ordonné : Dangereux est le pire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Niveau {
    #[default]
    Sain,
    ARegarder,
    Suspect,
    Dangereux,
}

impl Niveau {
    /// Lit un niveau depuis une règle éditée à la main (tolérant aux accents et variantes).
    pub fn depuis_texte(s: &str) -> Niveau {
        match s.trim().to_lowercase().replace('à', "a").as_str() {
            "dangereux" | "danger" | "malveillant" => Niveau::Dangereux,
            "suspect" => Niveau::Suspect,
            "a_regarder" | "aregarder" | "a regarder" => Niveau::ARegarder,
            _ => Niveau::Sain,
        }
    }
    pub fn libelle(&self) -> &'static str {
        match self {
            Niveau::Sain => "Sain",
            Niveau::ARegarder => "À regarder",
            Niveau::Suspect => "Suspect",
            Niveau::Dangereux => "Dangereux",
        }
    }
}

/// Un point relevé sur un fichier : jamais un ordre, toujours une explication.
#[derive(Debug, Clone)]
pub struct Constat {
    pub niveau: Niveau,
    pub titre: String,
    /// Ce qui est observé (le fait brut).
    pub observation: String,
    /// Pourquoi ça compte, dit honnêtement (y compris « c'est souvent légitime »).
    pub pourquoi: String,
}

impl Constat {
    pub fn new(niveau: Niveau, titre: impl Into<String>, observation: impl Into<String>, pourquoi: impl Into<String>) -> Constat {
        Constat { niveau, titre: titre.into(), observation: observation.into(), pourquoi: pourquoi.into() }
    }
}

/// Le rapport complet d'un fichier analysé.
#[derive(Debug, Clone, Default)]
pub struct Rapport {
    pub chemin: String,
    pub taille: u64,
    pub sha256: String,
    pub est_pe: bool,
    /// Résumé lisible du binaire (clé → valeur), pour l'affichage.
    pub resume_pe: Vec<(String, String)>,
    pub constats: Vec<Constat>,
    /// Aperçu désassemblé à partir du point d'entrée (une ligne par instruction).
    pub apercu_asm: Vec<String>,
    pub niveau: Niveau,
    /// Entropie globale du fichier (0 à 8).
    pub entropie: f64,
    /// Carte des sections du programme (vide si ce n'est pas un exécutable Windows).
    pub sections: Vec<SectionVue>,
    /// Étapes de l'analyse, dans l'ordre : (nom, réellement effectuée ?). Sert au schéma « parcours de l'analyse ».
    pub etapes: Vec<(String, bool)>,
}

/// Une section du programme, telle que la vue experte la dessine.
#[derive(Debug, Clone)]
pub struct SectionVue {
    pub nom: String,
    pub taille: u64,
    pub executable: bool,
    pub inscriptible: bool,
    pub entropie: f64,
}

impl Rapport {
    pub fn ajouter(&mut self, c: Constat) {
        self.constats.push(c);
    }
    pub fn etape(&mut self, nom: &str, faite: bool) {
        self.etapes.push((nom.to_string(), faite));
    }
    /// Le niveau global = le pire constat. Rien = Sain (mais « Sain » ne veut dire que « rien de connu »).
    pub fn finaliser(&mut self) {
        self.niveau = self.constats.iter().map(|c| c.niveau).max().unwrap_or(Niveau::Sain);
        self.constats.sort_by(|a, b| b.niveau.cmp(&a.niveau));
    }
}
