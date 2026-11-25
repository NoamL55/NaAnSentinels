// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Signature des versions officielles (Ed25519).
//!
//! Le code est libre : tout le monde peut le copier, le modifier, le recompiler. Ce que la signature apporte,
//! c'est une preuve d'origine : seul le détenteur de la clé privée peut produire un fichier `.sig` valide pour la
//! clé publique `keys/officielle.pub`. Un clone ne peut donc pas se faire passer pour la version officielle ;
//! il peut seulement signer avec SA propre clé, dont l'empreinte sera différente.
//!
//! La clé privée ne quitte jamais le dossier de données de son propriétaire et n'entre jamais dans le dépôt.

use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

/// Clé publique officielle, en hexadécimal (64 caractères). Que des zéros = aucune clé officielle publiée.
pub const CLE_OFFICIELLE: &str = include_str!("../keys/officielle.pub");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statut {
    /// Signature valide pour la clé officielle.
    Officielle,
    /// Pas de fichier `.sig` à côté de l'exécutable : compilation personnelle ou version dérivée.
    Absente,
    /// Un `.sig` existe mais ne correspond pas : fichier modifié, ou signé par une autre clé.
    Invalide,
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|b| format!("{:02x}", b)).collect()
}

fn depuis_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let s = s.trim();
    if s.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// Empreinte courte d'une clé publique, à comparer avec celle publiée dans le README.
pub fn empreinte_cle(cle_hex: &str) -> String {
    match depuis_hex::<32>(cle_hex) {
        Some(o) if o != [0u8; 32] => {
            let d = Sha256::digest(o);
            hex(&d[..8]).as_bytes().chunks(4).map(|c| String::from_utf8_lossy(c).to_string()).collect::<Vec<_>>().join(" ")
        }
        _ => "aucune clé officielle publiée".to_string(),
    }
}

pub fn verifier(octets: &[u8], signature_hex: &str, cle_hex: &str) -> bool {
    let (Some(cle), Some(sig)) = (depuis_hex::<32>(cle_hex), depuis_hex::<64>(signature_hex)) else {
        return false;
    };
    // Une clé nulle signifie « aucune clé publiée » : elle ne valide jamais rien.
    if cle == [0u8; 32] {
        return false;
    }
    let Ok(cle) = VerifyingKey::from_bytes(&cle) else {
        return false;
    };
    // Vérification stricte : refuse les clés faibles et les signatures malléables, qu'une vérification
    // ordinaire accepterait (une clé nulle « validait » une signature nulle).
    cle.verify_strict(octets, &Signature::from_bytes(&sig)).is_ok()
}

pub fn signer(octets: &[u8], cle_privee_hex: &str) -> Option<String> {
    let cle = SigningKey::from_bytes(&depuis_hex::<32>(cle_privee_hex)?);
    Some(hex(&cle.sign(octets).to_bytes()))
}

fn chemin_signature(fichier: &Path) -> PathBuf {
    let mut nom = fichier.as_os_str().to_os_string();
    nom.push(".sig");
    PathBuf::from(nom)
}

/// Statut d'un fichier par rapport à la clé officielle (son `.sig` est cherché à côté de lui).
pub fn statut_fichier(fichier: &Path) -> Statut {
    let Ok(sig) = fs::read_to_string(chemin_signature(fichier)) else {
        return Statut::Absente;
    };
    match fs::read(fichier) {
        Ok(octets) if verifier(&octets, &sig, CLE_OFFICIELLE) => Statut::Officielle,
        _ => Statut::Invalide,
    }
}

/// Statut de l'exécutable en cours (calculé une fois au démarrage, affiché dans « À propos »).
pub fn statut_executable() -> Statut {
    std::env::current_exe().map(|p| statut_fichier(&p)).unwrap_or(Statut::Absente)
}

fn chemin_cle_privee() -> PathBuf {
    crate::etat::dossier_donnees().join("cle-signature-privee.key")
}

// ---------------------------------------------------------------- Commandes du terminal

/// `naansentinels --generer-cle` : crée une paire de clés. La clé privée va dans le dossier de données de
/// l'utilisateur (jamais écrasée, jamais affichée) ; seule la clé publique est imprimée.
pub fn commande_generer_cle() -> i32 {
    let chemin = chemin_cle_privee();
    if chemin.exists() {
        eprintln!("Une clé privée existe déjà : {}\nElle n'est pas écrasée. Déplace-la d'abord si tu veux en créer une nouvelle.", chemin.display());
        return 1;
    }
    let cle = SigningKey::generate(&mut rand::rngs::OsRng);
    if let Some(d) = chemin.parent() {
        let _ = fs::create_dir_all(d);
    }
    if let Err(e) = fs::write(&chemin, hex(&cle.to_bytes())) {
        eprintln!("Écriture de la clé privée impossible : {e}");
        return 1;
    }
    let publique = hex(&cle.verifying_key().to_bytes());
    println!("Clé privée enregistrée dans : {}", chemin.display());
    println!("  -> garde-la secrète et fais-en une sauvegarde hors ligne. Ne la mets jamais dans le dépôt.");
    println!("Clé publique (à mettre dans keys/officielle.pub) :\n{publique}");
    println!("Empreinte : {}", empreinte_cle(&publique));
    0
}

/// `naansentinels --signer <fichier>` : écrit `<fichier>.sig` avec la clé privée de l'utilisateur.
pub fn commande_signer(fichier: &Path) -> i32 {
    let Ok(cle) = fs::read_to_string(chemin_cle_privee()) else {
        eprintln!("Aucune clé privée. Crée-la d'abord : naansentinels --generer-cle");
        return 1;
    };
    let octets = match fs::read(fichier) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Lecture de {} impossible : {e}", fichier.display());
            return 1;
        }
    };
    let Some(sig) = signer(&octets, &cle) else {
        eprintln!("Clé privée illisible.");
        return 1;
    };
    let sortie = chemin_signature(fichier);
    if let Err(e) = fs::write(&sortie, &sig) {
        eprintln!("Écriture de {} impossible : {e}", sortie.display());
        return 1;
    }
    println!("Signature écrite : {}", sortie.display());
    0
}

/// `naansentinels --verifier <fichier>` : dit si le fichier est une version officielle.
pub fn commande_verifier(fichier: &Path) -> i32 {
    println!("Clé officielle : {}", empreinte_cle(CLE_OFFICIELLE));
    match statut_fichier(fichier) {
        Statut::Officielle => {
            println!("OFFICIELLE : la signature est valide pour la clé officielle.");
            0
        }
        Statut::Absente => {
            println!("NON SIGNÉE : aucun fichier .sig à côté. Compilation personnelle ou version dérivée.");
            2
        }
        Statut::Invalide => {
            println!("INVALIDE : la signature ne correspond pas. Fichier modifié, ou signé par une autre clé.");
            3
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signer_puis_verifier() {
        let cle = SigningKey::generate(&mut rand::rngs::OsRng);
        let (privee, publique) = (hex(&cle.to_bytes()), hex(&cle.verifying_key().to_bytes()));
        let sig = signer(b"version officielle", &privee).unwrap();
        assert!(verifier(b"version officielle", &sig, &publique));
        assert!(!verifier(b"version modifiee", &sig, &publique), "un octet changé invalide la signature");
        let autre = SigningKey::generate(&mut rand::rngs::OsRng);
        assert!(!verifier(b"version officielle", &sig, &hex(&autre.verifying_key().to_bytes())), "une autre clé ne valide pas");
        assert!(!verifier(b"version officielle", "pas une signature", &publique));
        // Régression : clé nulle + signature nulle ne doit JAMAIS passer pour une version officielle.
        assert!(!verifier(b"n'importe quoi", &"00".repeat(64), &"00".repeat(32)));
        assert_ne!(empreinte_cle(&publique), empreinte_cle(&hex(&autre.verifying_key().to_bytes())));
    }

    #[test]
    fn fichier_sans_signature_ou_falsifie() {
        let dossier = std::env::temp_dir().join(format!("naansentinels-sig-{}", std::process::id()));
        fs::create_dir_all(&dossier).unwrap();
        let f = dossier.join("programme.bin");
        fs::write(&f, b"contenu").unwrap();
        assert_eq!(statut_fichier(&f), Statut::Absente);
        fs::write(chemin_signature(&f), "00".repeat(64)).unwrap();
        assert_eq!(statut_fichier(&f), Statut::Invalide);
    }
}
