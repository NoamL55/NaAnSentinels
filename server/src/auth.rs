// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Mots de passe (Argon2id), jetons de session, rôles et droits.
//!
//! Rôles → droits (« scopes ») :
//!   admin        tableau:lire, utilisateurs:gerer, journal:lire, appareils:les-miens
//!   analyste     tableau:lire, appareils:les-miens
//!   utilisateur  appareils:les-miens

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub const ROLES: [&str; 3] = ["admin", "analyste", "utilisateur"];

pub fn droits(role: &str) -> &'static [&'static str] {
    match role {
        "admin" => &["tableau:lire", "utilisateurs:gerer", "journal:lire", "appareils:les-miens"],
        "analyste" => &["tableau:lire", "appareils:les-miens"],
        "utilisateur" => &["appareils:les-miens"],
        _ => &[],
    }
}

pub fn a_le_droit(role: &str, droit: &str) -> bool {
    droits(role).contains(&droit)
}

pub fn valider_pseudo(p: &str) -> Result<(), String> {
    let ok = (3..=24).contains(&p.chars().count()) && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if ok {
        Ok(())
    } else {
        Err("Pseudo invalide : 3 à 24 caractères, lettres, chiffres, « _ » ou « - ».".into())
    }
}

pub fn valider_mot_de_passe(m: &str) -> Result<(), String> {
    let n = m.chars().count();
    if !(10..=128).contains(&n) {
        return Err("Mot de passe invalide : 10 à 128 caractères.".into());
    }
    Ok(())
}

/// Hache un mot de passe avec Argon2id et un sel aléatoire. Coûteux exprès : à appeler hors du fil asynchrone.
pub fn hacher(mdp: &str) -> Result<String, String> {
    let sel = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(mdp.as_bytes(), &sel)
        .map(|h| h.to_string())
        .map_err(|e| format!("hachage impossible : {e}"))
}

pub fn verifier(mdp: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| Argon2::default().verify_password(mdp.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// Jeton de session : 32 octets aléatoires du système, en hexadécimal. Seule son empreinte est stockée.
pub fn nouveau_jeton() -> String {
    let mut o = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut o);
    o.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn empreinte(jeton: &str) -> String {
    let mut h = Sha256::new();
    h.update(jeton.as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_et_droits() {
        assert!(a_le_droit("admin", "utilisateurs:gerer"));
        assert!(a_le_droit("analyste", "tableau:lire"));
        assert!(!a_le_droit("analyste", "utilisateurs:gerer"));
        assert!(!a_le_droit("analyste", "journal:lire"));
        assert!(a_le_droit("utilisateur", "appareils:les-miens"));
        assert!(!a_le_droit("utilisateur", "tableau:lire"));
        assert!(!a_le_droit("inconnu", "appareils:les-miens"));
    }

    #[test]
    fn validation() {
        assert!(valider_pseudo("noam_55").is_ok());
        assert!(valider_pseudo("ab").is_err());
        assert!(valider_pseudo("avec espace").is_err());
        assert!(valider_pseudo("<script>").is_err());
        assert!(valider_mot_de_passe("court").is_err());
        assert!(valider_mot_de_passe("assez-long-pour-passer").is_ok());
    }

    #[test]
    fn hachage_et_jetons() {
        let h = hacher("un-mot-de-passe-de-test").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verifier("un-mot-de-passe-de-test", &h));
        assert!(!verifier("un-autre-mot-de-passe", &h));
        assert!(!verifier("x", "pas-un-hash"));
        let (a, b) = (nouveau_jeton(), nouveau_jeton());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(empreinte(&a).len(), 64);
        assert_ne!(empreinte(&a), a);
    }
}
