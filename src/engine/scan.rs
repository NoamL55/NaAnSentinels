// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Orchestration : un fichier → un rapport informatif. Lecture seule, rien n'est bloqué, déplacé ni supprimé.

use std::fs;
use std::path::Path;

use super::config::Config;
use super::heuristics;
use super::pe;
use super::rules::Regles;
use super::verdict::{Constat, Niveau, Rapport, SectionVue};

/// Analyse un fichier contre les règles et la config. Ne lève jamais d'erreur : les problèmes deviennent des constats.
pub fn analyser_fichier(chemin: &Path, regles: &Regles, cfg: &Config) -> Rapport {
    let mut r = Rapport {
        chemin: chemin.display().to_string(),
        ..Default::default()
    };

    let octets = match fs::read(chemin) {
        Ok(o) => o,
        Err(e) => {
            r.ajouter(Constat::new(Niveau::ARegarder, "Lecture impossible", format!("{e}"), "Le fichier est verrouillé, protégé ou absent."));
            r.finaliser();
            return r;
        }
    };
    r.taille = octets.len() as u64;
    r.sha256 = heuristics::sha256_hex(&octets);

    let limite = cfg.analyse.taille_max_mo.saturating_mul(1_000_000);
    let vue: &[u8] = if limite > 0 && octets.len() > limite {
        r.ajouter(Constat::new(Niveau::Sain, "Fichier volumineux", format!("{} Mo", octets.len() / 1_000_000), "Analyse limitée au début du fichier (voir taille_max_mo dans config.toml)."));
        &octets[..limite]
    } else {
        &octets
    };

    r.entropie = heuristics::entropie(vue);
    r.etape("Lecture du fichier", true);

    // 1) Signatures (chaîne / hex / sha256)
    let empreinte = r.sha256.clone();
    appliquer_signatures(vue, &empreinte, regles, &mut r);
    r.etape("Signatures connues", !regles.signatures.is_empty());

    // 2) Chaînes lisibles révélatrices
    let chaines = heuristics::chaines_ascii(vue, 5);
    appliquer_chaines(&chaines, regles, &mut r);
    r.etape("Chaînes révélatrices", !regles.chaines_suspectes.is_empty());

    // 3) Analyse PE (si c'en est un) + désassemblage
    let analyse_pe = pe::analyser(vue);
    r.etape("Structure du programme", analyse_pe.is_ok());
    match analyse_pe {
        Ok(info) => {
            r.est_pe = true;
            r.resume_pe = info.resume.clone();
            appliquer_imports(&info.imports, regles, &mut r);

            // Carte des sections (taille, droits, entropie propre) pour les schémas de la vue experte.
            for s in &info.sections {
                let debut = (s.ptr_raw as usize).min(vue.len());
                let fin = debut.saturating_add(s.taille_raw as usize).min(vue.len());
                r.sections.push(SectionVue {
                    nom: s.nom.clone(),
                    taille: s.taille_raw as u64,
                    executable: s.executable,
                    inscriptible: s.inscriptible,
                    entropie: heuristics::entropie(&vue[debut..fin]),
                });
            }

            // Section à la fois inscriptible ET exécutable : rare pour un logiciel sain, fréquent chez les packers.
            for s in &info.sections {
                if s.executable && s.inscriptible {
                    r.ajouter(Constat::new(
                        Niveau::Suspect,
                        format!("Section {} inscriptible et exécutable", s.nom),
                        "Cette section peut être modifiée puis exécutée (W+X).",
                        "Signe classique de code auto-modifiant ou empaqueté ; rare dans un logiciel normal.",
                    ));
                }
            }

            // Entropie globale
            let ent = r.entropie;
            if ent >= cfg.analyse.entropie_suspecte {
                r.ajouter(Constat::new(
                    Niveau::ARegarder,
                    "Forte entropie",
                    format!("Entropie {ent:.2} / 8"),
                    "Le fichier est probablement compressé, chiffré ou empaqueté. Beaucoup d'installeurs légitimes le sont aussi : ce n'est pas une preuve.",
                ));
            }

            // Désassemblage du point d'entrée
            if let Some(code) = pe::code_entree(vue, &info) {
                let bitness = if info.est_64 { 64 } else { 32 };
                let rip = info.image_base.wrapping_add(info.entry_rva);
                r.apercu_asm = super::disasm::desassembler(code, bitness, rip, cfg.analyse.instructions_desassemblees);
            }
            let desassemble = !r.apercu_asm.is_empty();
            r.etape("Désassemblage du code", desassemble);
        }
        Err(_) => {
            r.etape("Désassemblage du code", false);
            r.ajouter(Constat::new(
                Niveau::Sain,
                "Pas un exécutable Windows",
                "Le fichier n'est pas au format PE.",
                "Analyse limitée aux signatures et aux chaînes ; pas de désassemblage.",
            ));
        }
    }

    // 4) Nom à double extension trompeuse
    if let Some(nom) = chemin.file_name().and_then(|n| n.to_str()) {
        if let Some(ext) = heuristics::double_extension(nom) {
            r.ajouter(Constat::new(
                Niveau::Suspect,
                "Double extension trompeuse",
                format!("Le fichier se termine réellement par « .{ext} »."),
                "Un document d'apparence inoffensive qui est en fait exécutable : procédé d'hameçonnage courant.",
            ));
        }
    }

    r.finaliser();

    // 5) Honnêteté : rien de connu ≠ sûr.
    if r.niveau == Niveau::Sain {
        r.ajouter(Constat::new(
            Niveau::Sain,
            "Rien de connu",
            "Aucune règle ni heuristique n'a réagi.",
            "NaAnSentinels ne connaît que ses propres règles : « sain » veut dire « rien de connu », pas « garanti sûr ». Recoupe avec Microsoft Defender ou VirusTotal pour un fichier important.",
        ));
    }

    r
}

fn appliquer_signatures(octets: &[u8], sha256: &str, regles: &Regles, r: &mut Rapport) {
    for sig in &regles.signatures {
        let touche = match sig.type_.to_lowercase().as_str() {
            "sha256" => sha256.eq_ignore_ascii_case(sig.motif.trim()),
            "hex" => heuristics::hex_vers_octets(&sig.motif)
                .map(|aig| heuristics::contient_octets(octets, &aig))
                .unwrap_or(false),
            _ => heuristics::contient_octets(octets, sig.motif.as_bytes()), // "chaine" par défaut
        };
        if touche {
            let niv = Niveau::depuis_texte(&sig.gravite);
            r.ajouter(Constat::new(
                niv,
                format!("Signature : {}", sig.nom),
                format!("Correspondance ({}).", sig.type_),
                if sig.source.is_empty() { "Règle éditable dans rules/signatures.toml.".to_string() } else { format!("Source : {}.", sig.source) },
            ));
        }
    }
}

fn appliquer_chaines(chaines: &[String], regles: &Regles, r: &mut Rapport) {
    for cs in &regles.chaines_suspectes {
        if chaines.iter().any(|c| c.contains(&cs.motif)) {
            r.ajouter(Constat::new(
                Niveau::depuis_texte(&cs.gravite),
                format!("Chaîne : « {} »", cs.motif),
                "Présente dans le fichier.",
                cs.raison.clone(),
            ));
        }
    }
}

fn appliquer_imports(imports: &[String], regles: &Regles, r: &mut Rapport) {
    for imp in &regles.imports_suspects {
        if imports.iter().any(|i| i.eq_ignore_ascii_case(&imp.nom)) {
            r.ajouter(Constat::new(
                Niveau::depuis_texte(&imp.gravite),
                format!("Import : {}", imp.nom),
                "Cette fonction système est importée par le programme.",
                imp.raison.clone(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::rules::Signature;
    use std::path::PathBuf;

    fn regles_test() -> Regles {
        Regles {
            signatures: vec![Signature {
                nom: "Motif-Test".into(),
                type_: "chaine".into(),
                motif: "MOTIF-TEST-NAANSENTINELS".into(),
                gravite: "dangereux".into(),
                source: "test".into(),
            }],
            ..Default::default()
        }
    }

    fn fichier_temp(nom: &str, contenu: &[u8]) -> PathBuf {
        let dossier = std::env::temp_dir().join(format!("naansentinels-test-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let p = dossier.join(nom);
        std::fs::write(&p, contenu).unwrap();
        p
    }

    #[test]
    fn regles_livrees_valides() {
        let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("rules/signatures.toml");
        let r = Regles::charger(&chemin).expect("rules/signatures.toml doit se charger");
        assert!(r.signatures.iter().any(|s| s.nom == "EICAR-Test-File"));
        assert!(!r.imports_suspects.is_empty() && !r.chaines_suspectes.is_empty());
        let cfg = Config::charger(&Path::new(env!("CARGO_MANIFEST_DIR")).join("config.toml"));
        assert!(cfg.analyse.entropie_suspecte > 0.0);
    }

    #[test]
    fn signature_exacte_donne_dangereux() {
        let p = fichier_temp("note.txt", b"bonjour MOTIF-TEST-NAANSENTINELS fin");
        let r = analyser_fichier(&p, &regles_test(), &Config::default());
        assert_eq!(r.niveau, Niveau::Dangereux);
        assert!(!r.est_pe);
        assert_eq!(r.sha256.len(), 64);
    }

    #[test]
    fn texte_banal_sain_mais_honnete() {
        let p = fichier_temp("banal.txt", b"rien de special ici");
        let r = analyser_fichier(&p, &regles_test(), &Config::default());
        assert_eq!(r.niveau, Niveau::Sain);
        assert!(r.constats.iter().any(|c| c.titre == "Rien de connu" && c.pourquoi.contains("pas")));
    }

    #[test]
    fn double_extension_suspecte() {
        let p = fichier_temp("facture.pdf.exe", b"ceci n'est pas un vrai executable");
        let r = analyser_fichier(&p, &Regles::default(), &Config::default());
        assert_eq!(r.niveau, Niveau::Suspect);
    }

    #[test]
    fn fichier_absent_ne_plante_pas() {
        let r = analyser_fichier(Path::new("Z:/nexiste/pas.exe"), &Regles::default(), &Config::default());
        assert!(r.constats.iter().any(|c| c.titre == "Lecture impossible"));
    }

    /// Vrai binaire Windows : goblin doit le parser et iced-x86 désassembler son point d'entrée.
    #[cfg(windows)]
    #[test]
    fn vrai_pe_notepad() {
        let p = Path::new(r"C:\Windows\System32\notepad.exe");
        if !p.exists() {
            return;
        }
        let r = analyser_fichier(p, &Regles::default(), &Config::default());
        assert!(r.est_pe, "notepad.exe doit être reconnu comme PE");
        assert!(!r.apercu_asm.is_empty(), "le point d'entrée doit être désassemblé");
        assert!(r.resume_pe.iter().any(|(k, v)| k == "Architecture" && v.contains("64")));
        // Un binaire Microsoft sans règle chargée ne doit jamais sortir « Dangereux ».
        assert!(r.niveau < Niveau::Dangereux);
    }
}
