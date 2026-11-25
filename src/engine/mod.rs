// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Moteur d'analyse de NaAnSentinels — logique pure, sans interface. Lecture seule : rien n'est bloqué ni supprimé.

pub mod config;
pub mod disasm;
pub mod heuristics;
pub mod pe;
pub mod rules;
pub mod scan;
pub mod verdict;

pub use config::Config;
pub use rules::Regles;
pub use scan::analyser_fichier;
