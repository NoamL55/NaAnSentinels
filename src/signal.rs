// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Signal de présence FACULTATIF et compte. Rien ne part tant que l'utilisateur n'a pas activé le signal.
//! Ce qui part est exactement `charge_utile` (affichée telle quelle dans l'onglet Compte) : identifiant aléatoire,
//! version, système, compteurs. Jamais de nom de fichier, de chemin ni d'empreinte.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::etat::EtatLocal;

pub const PERIODE: Duration = Duration::from_secs(60);

pub fn systeme() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "autre"
    }
}

/// Le contenu exact du signal. C'est aussi ce que l'interface montre à l'utilisateur.
pub fn charge_utile(e: &EtatLocal) -> Value {
    json!({
        "install_id": e.install_id,
        "version": env!("CARGO_PKG_VERSION"),
        "os": systeme(),
        "analyses": {
            "total": e.compteurs.total, "sain": e.compteurs.sain, "a_regarder": e.compteurs.a_regarder,
            "suspect": e.compteurs.suspect, "dangereux": e.compteurs.dangereux,
        },
    })
}

/// Adresse de serveur acceptable : HTTPS, ou HTTP seulement vers cette machine (essais locaux).
/// Renvoie l'adresse normalisée, sans barre finale.
pub fn url_sure(serveur: &str) -> Result<String, String> {
    let s = serveur.trim().trim_end_matches('/');
    if s.is_empty() {
        return Err("Aucun serveur indiqué.".into());
    }
    if s.contains(char::is_whitespace) || s.contains('@') {
        return Err("Adresse de serveur invalide.".into());
    }
    if let Some(reste) = s.strip_prefix("https://") {
        return if reste.is_empty() { Err("Adresse de serveur invalide.".into()) } else { Ok(s.to_string()) };
    }
    if let Some(reste) = s.strip_prefix("http://") {
        let hote = reste.split(['/', ':']).next().unwrap_or("");
        if hote == "localhost" || hote == "127.0.0.1" {
            return Ok(s.to_string());
        }
        return Err("Connexion non chiffrée refusée : l'adresse doit commencer par https://".into());
    }
    Err("L'adresse doit commencer par https://".into())
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(8)).build()
}

fn message_erreur(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, rep) => rep
            .into_json::<Value>()
            .ok()
            .and_then(|v| v.get("erreur").and_then(|m| m.as_str()).map(|m| m.to_string()))
            .unwrap_or_else(|| format!("Le serveur a répondu {code}.")),
        ureq::Error::Transport(_) => "Serveur injoignable.".to_string(),
    }
}

pub fn envoyer_signal(e: &EtatLocal) -> Result<(), String> {
    let base = url_sure(&e.serveur)?;
    let mut req = agent().post(&format!("{base}/api/v1/signal")).set("X-NaAnSentinels", "1");
    if let Some(j) = &e.jeton {
        req = req.set("Authorization", &format!("Bearer {j}"));
    }
    req.send_json(charge_utile(e)).map(|_| ()).map_err(message_erreur)
}

pub struct Connexion {
    pub jeton: String,
    pub pseudo: String,
    pub role: String,
}

pub fn connexion(serveur: &str, pseudo: &str, mot_de_passe: &str) -> Result<Connexion, String> {
    let base = url_sure(serveur)?;
    let rep = agent()
        .post(&format!("{base}/api/v1/connexion"))
        .set("X-NaAnSentinels", "1")
        .send_json(json!({ "pseudo": pseudo, "mot_de_passe": mot_de_passe }))
        .map_err(message_erreur)?;
    let v: Value = rep.into_json().map_err(|_| "Réponse du serveur illisible.".to_string())?;
    let champ = |c: &str| v.get(c).and_then(|x| x.as_str()).map(|x| x.to_string());
    match (champ("jeton"), champ("pseudo"), champ("role")) {
        (Some(jeton), Some(pseudo), Some(role)) => Ok(Connexion { jeton, pseudo, role }),
        _ => Err("Réponse du serveur incomplète.".into()),
    }
}

pub fn deconnexion(serveur: &str, jeton: &str) {
    if let Ok(base) = url_sure(serveur) {
        let _ = agent()
            .post(&format!("{base}/api/v1/deconnexion"))
            .set("X-NaAnSentinels", "1")
            .set("Authorization", &format!("Bearer {jeton}"))
            .send_json(json!({}));
    }
}

/// Fil d'arrière-plan : envoie le signal toutes les minutes, seulement si l'utilisateur l'a activé.
/// `statut` reçoit une phrase lisible affichée dans l'onglet Compte.
pub fn demarrer(local: Arc<Mutex<EtatLocal>>, statut: Arc<Mutex<String>>) {
    std::thread::spawn(move || {
        let mut dernier: Option<Instant> = None;
        let mut etait_actif = false;
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let copie = local.lock().unwrap_or_else(|e| e.into_inner()).clone();
            if !copie.signal_actif {
                if etait_actif {
                    *statut.lock().unwrap_or_else(|e| e.into_inner()) = "Signal désactivé : rien n'est envoyé.".into();
                }
                etait_actif = false;
                dernier = None;
                continue;
            }
            etait_actif = true;
            if dernier.map(|d| d.elapsed() < PERIODE).unwrap_or(false) {
                continue;
            }
            dernier = Some(Instant::now());
            let phrase = match envoyer_signal(&copie) {
                Ok(()) => "Dernier signal : envoyé.".to_string(),
                Err(e) => format!("Dernier signal : échec — {e}"),
            };
            *statut.lock().unwrap_or_else(|e| e.into_inner()) = phrase;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_obligatoire_hors_machine_locale() {
        assert_eq!(url_sure("https://exemple.org/").unwrap(), "https://exemple.org");
        assert!(url_sure("http://127.0.0.1:8791").is_ok());
        assert!(url_sure("http://localhost:8791/").is_ok());
        assert!(url_sure("http://exemple.org").is_err(), "HTTP en clair vers Internet : refusé");
        assert!(url_sure("http://127.0.0.1.evil.org").is_err(), "faux hôte local : refusé");
        assert!(url_sure("ftp://exemple.org").is_err());
        assert!(url_sure("https://utilisateur@exemple.org").is_err());
        assert!(url_sure("").is_err());
    }

    #[test]
    fn le_signal_ne_contient_que_les_champs_annonces() {
        let mut e = EtatLocal::default();
        e.pseudo = Some("quelquun".into());
        e.jeton = Some("secret".into());
        e.serveur = "https://exemple.org".into();
        let v = charge_utile(&e);
        let mut cles: Vec<&str> = v.as_object().unwrap().keys().map(|k| k.as_str()).collect();
        cles.sort();
        assert_eq!(cles, vec!["analyses", "install_id", "os", "version"]);
        let texte = v.to_string();
        assert!(!texte.contains("secret") && !texte.contains("quelquun"), "ni jeton ni pseudo dans le corps du signal");
    }
}
