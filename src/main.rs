// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! NaAnSentinels — l'antivirus open source qui explique au lieu d'interdire.
//! Analyse statique de fichiers (reverse engineering AVANT exécution), anti-phishing, règles éditables.
//!
//! Utilisation :
//!   naansentinels                         interface graphique
//!   naansentinels <fichier>               interface graphique, ouverte sur l'analyse de ce fichier
//!   naansentinels --analyser <fichier>    rapport texte dans le terminal
//!   naansentinels --verifier <fichier>    dit si un exécutable est une version officielle (signature)
//!   naansentinels --generer-cle           crée ta propre paire de clés de signature
//!   naansentinels --signer <fichier>      signe un fichier avec ta clé privée (écrit <fichier>.sig)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod etat;
mod phishing;
mod signal;
mod signature;
mod ui;

use std::path::Path;

/// En version finale l'exécutable est une application fenêtrée, sans console. Pour que les commandes du terminal
/// (`--verifier`, `--analyser`…) affichent leur résultat, on se rattache à la console du terminal qui l'a lancé.
#[cfg(windows)]
fn rattacher_console() {
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(processus: u32) -> i32;
    }
    // u32::MAX = « le processus parent ». Sans effet (et sans danger) s'il n'y a pas de console parente.
    unsafe {
        AttachConsole(u32::MAX);
    }
}
#[cfg(not(windows))]
fn rattacher_console() {}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().skip(1).any(|a| a.starts_with("--")) {
        rattacher_console();
    }
    let apres = |option: &str| args.iter().position(|a| a == option).map(|i| args.get(i + 1).cloned());

    if let Some(valeur) = apres("--analyser") {
        match valeur {
            Some(chemin) => rapport_texte(Path::new(&chemin)),
            None => eprintln!("Usage : naansentinels --analyser <fichier>"),
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--generer-cle") {
        std::process::exit(signature::commande_generer_cle());
    }
    for (option, commande) in [("--signer", signature::commande_signer as fn(&Path) -> i32), ("--verifier", signature::commande_verifier)] {
        if let Some(valeur) = apres(option) {
            match valeur {
                Some(chemin) => std::process::exit(commande(Path::new(&chemin))),
                None => {
                    eprintln!("Usage : naansentinels {option} <fichier>");
                    std::process::exit(2);
                }
            }
        }
    }

    // `naansentinels <fichier>` (ou un fichier glissé sur l'exécutable) : l'interface s'ouvre sur son analyse.
    let a_ouvrir = args.iter().skip(1).find(|a| !a.starts_with("--")).map(std::path::PathBuf::from);

    let options = eframe::NativeOptions {
        // Lissage des bords du bouclier 3D et des dégradés.
        multisampling: 4,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1140.0, 800.0])
            .with_min_inner_size([860.0, 600.0])
            .with_icon(std::sync::Arc::new(ui::icone()))
            .with_title("NaAnSentinels"),
        ..Default::default()
    };
    eframe::run_native(
        "NaAnSentinels",
        options,
        Box::new(move |cc| Ok(Box::new(ui::App::new(cc, a_ouvrir)))),
    )
}

/// Mode terminal : même moteur, mêmes règles, sortie texte. Lecture seule, le fichier n'est jamais exécuté.
fn rapport_texte(chemin: &Path) {
    let cfg = engine::Config::charger(&ui::trouver_ressource("config.toml"));
    let regles = match engine::Regles::charger(&ui::trouver_ressource("rules/signatures.toml")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Attention : {e} — analyse sans règles.");
            engine::Regles::default()
        }
    };
    let r = engine::analyser_fichier(chemin, &regles, &cfg);
    println!("Fichier  : {}", r.chemin);
    println!("Taille   : {} octets", r.taille);
    println!("SHA-256  : {}", r.sha256);
    println!("Entropie : {:.2} / 8", r.entropie);
    println!("VERDICT  : {}", r.niveau.libelle());
    for (k, v) in &r.resume_pe {
        println!("  {k} : {v}");
    }
    if !r.sections.is_empty() {
        println!("Sections :");
        for s in &r.sections {
            println!("  {:<10} {:>9} o  entropie {:.2}  {}{}", s.nom, s.taille, s.entropie, if s.executable { "X" } else { "-" }, if s.inscriptible { "W" } else { "-" });
        }
    }
    println!("Constats ({}) :", r.constats.len());
    for c in &r.constats {
        println!("  [{}] {} — {}", c.niveau.libelle(), c.titre, c.observation);
        println!("      -> {}", c.pourquoi);
    }
    if !r.apercu_asm.is_empty() {
        println!("Désassemblage du point d'entrée ({} instructions) :", r.apercu_asm.len());
        for l in &r.apercu_asm {
            println!("  {l}");
        }
    }
}
