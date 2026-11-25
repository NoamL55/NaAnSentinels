// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Anti-phishing hors-ligne : on analyse un mail (texte collé ou fichier .eml) SANS se connecter à Internet.
//! Palier 1 : liens trompeurs, domaines en IP ou en punycode, incohérence texte/lien, mots d'urgence.
//! Palier 2 (config [mail]) : se connecter en IMAP à ta boîte et analyser automatiquement. Ton mot de passe
//! d'application ira dans un fichier .env, jamais dans le code ni dans Git, et NaAnSentinels ne le montre jamais.

use crate::engine::verdict::{Constat, Niveau};

pub struct RapportMail {
    pub niveau: Niveau,
    pub constats: Vec<Constat>,
    pub urls: Vec<String>,
}

/// Analyse le texte brut d'un mail (en-têtes + corps). Lecture seule, aucune connexion réseau.
pub fn analyser_mail(texte: &str) -> RapportMail {
    let mut constats = Vec::new();
    let urls = extraire_urls(texte);

    for url in &urls {
        if let Some(h) = hote(url) {
            if est_ip(&h) {
                constats.push(Constat::new(Niveau::Suspect, "Lien vers une adresse IP", format!("{url}"), "Un service légitime utilise un nom de domaine, pas une IP brute."));
            }
            if h.contains("xn--") {
                constats.push(Constat::new(Niveau::ARegarder, "Domaine en punycode", format!("{h}"), "Peut imiter une marque avec des caractères ressemblants (ex. « аpple » cyrillique)."));
            }
            if url.starts_with("http://") {
                constats.push(Constat::new(Niveau::ARegarder, "Lien non chiffré (http)", format!("{url}"), "Une page de connexion sérieuse est toujours en https."));
            }
        }
    }

    // Incohérence entre le texte affiché d'un lien et sa vraie destination.
    for (texte_lien, href) in liens_html(texte) {
        if let (Some(h_affiche), Some(h_reel)) = (domaine_dans_texte(&texte_lien), hote(&href)) {
            if !memes_domaines(&h_affiche, &h_reel) {
                constats.push(Constat::new(
                    Niveau::Suspect,
                    "Lien trompeur",
                    format!("Affiche « {h_affiche} » mais pointe vers « {h_reel} »."),
                    "Le texte du lien et sa destination ne correspondent pas : hameçonnage classique.",
                ));
            }
        }
    }

    // Mots d'urgence / de pression.
    let bas = texte.to_lowercase();
    let urgences = [
        ("mot de passe", "demande de mot de passe"),
        ("verrouill", "menace de verrouillage du compte"),
        ("suspendu", "menace de suspension du compte"),
        ("immédiat", "pression à agir immédiatement"),
        ("vérifiez votre compte", "demande de vérification de compte"),
        ("verify your account", "demande de vérification de compte"),
        ("identifiants", "demande d'identifiants"),
        ("connexion inhabituelle", "fausse alerte de connexion"),
    ];
    let trouves: Vec<&str> = urgences.iter().filter(|(m, _)| bas.contains(m)).map(|(_, r)| *r).collect();
    if !trouves.is_empty() {
        constats.push(Constat::new(
            Niveau::ARegarder,
            "Ton d'urgence",
            trouves.join(", "),
            "L'urgence et la peur sont les leviers habituels de l'hameçonnage. À recouper avec l'expéditeur réel.",
        ));
    }

    if constats.is_empty() {
        constats.push(Constat::new(Niveau::Sain, "Rien de flagrant", "Aucun signe courant d'hameçonnage.", "Ce n'est pas une garantie : vérifie toujours l'adresse exacte de l'expéditeur avant de cliquer."));
    }

    let niveau = constats.iter().map(|c| c.niveau).max().unwrap_or(Niveau::Sain);
    constats.sort_by(|a, b| b.niveau.cmp(&a.niveau));
    RapportMail { niveau, constats, urls }
}

fn extraire_urls(t: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for tok in t.split(|c: char| c.is_whitespace() || "\"'<>()[]{}".contains(c)) {
        let tok = tok.trim_end_matches(|c: char| ".,;:!?".contains(c));
        if tok.starts_with("http://") || tok.starts_with("https://") {
            urls.push(tok.to_string());
        }
    }
    urls.sort();
    urls.dedup();
    urls
}

fn hote(url: &str) -> Option<String> {
    let apres = url.split("://").nth(1)?;
    let hote = apres.split(|c| c == '/' || c == '?' || c == '#' || c == ':').next()?;
    if hote.is_empty() {
        None
    } else {
        Some(hote.to_lowercase())
    }
}

fn est_ip(h: &str) -> bool {
    let parts: Vec<&str> = h.split('.').collect();
    parts.len() == 4 && parts.iter().all(|x| x.parse::<u8>().is_ok())
}

/// Domaine enregistrable approximatif : les deux derniers labels (exemple.com à partir de a.b.exemple.com).
fn domaine_racine(h: &str) -> String {
    let parts: Vec<&str> = h.split('.').filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        h.to_string()
    }
}

fn memes_domaines(a: &str, b: &str) -> bool {
    domaine_racine(a) == domaine_racine(b)
}

/// Repère un domaine dans un texte affiché (ex. « connectez-vous sur paypal.com »).
fn domaine_dans_texte(t: &str) -> Option<String> {
    for tok in t.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-')) {
        let tok = tok.trim_matches('.');
        if tok.contains('.') && !tok.starts_with("http") {
            let parts: Vec<&str> = tok.split('.').collect();
            if parts.len() >= 2 && parts.last().map(|e| e.len() >= 2 && e.chars().all(|c| c.is_ascii_alphabetic())).unwrap_or(false) {
                return Some(tok.to_lowercase());
            }
        }
    }
    None
}

/// Extrait les paires (texte affiché, href) des balises <a href="...">texte</a>. Analyse légère, sans dépendance.
fn liens_html(t: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bas = t.to_lowercase();
    let mut debut = 0;
    while let Some(rel) = bas[debut..].find("href=") {
        let pos = debut + rel + 5; // juste après « href= »
        let (href, consomme) = lire_valeur_attribut(&t[pos..]);
        let apres_attr = pos + consomme;
        // Texte affiché : du premier « > » jusqu'à « </a ».
        let texte_lien = t
            .get(apres_attr..)
            .and_then(|r| r.split_once('>'))
            .map(|(_, apres_balise)| apres_balise.split("</a").next().unwrap_or(""))
            .unwrap_or("");
        if !href.is_empty() {
            out.push((nettoyer_html(texte_lien), href));
        }
        debut = apres_attr.max(pos); // toujours progresser
        if out.len() > 100 {
            break;
        }
    }
    out
}

/// Lit la valeur d'un attribut commençant à `s` (après « href= »). Renvoie (valeur, octets consommés).
fn lire_valeur_attribut(s: &str) -> (String, usize) {
    let espaces = s.len() - s.trim_start().len();
    let s2 = &s[espaces..];
    if let Some(reste) = s2.strip_prefix('"') {
        if let Some(fin) = reste.find('"') {
            return (reste[..fin].to_string(), espaces + 1 + fin + 1);
        }
    } else if let Some(reste) = s2.strip_prefix('\'') {
        if let Some(fin) = reste.find('\'') {
            return (reste[..fin].to_string(), espaces + 1 + fin + 1);
        }
    } else {
        let fin = s2.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(s2.len());
        return (s2[..fin].to_string(), espaces + fin);
    }
    (String::new(), espaces)
}

fn nettoyer_html(t: &str) -> String {
    // Retire les balises résiduelles du texte affiché.
    let mut out = String::new();
    let mut dans_balise = false;
    for c in t.chars() {
        match c {
            '<' => dans_balise = true,
            '>' => dans_balise = false,
            _ if !dans_balise => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_sain() {
        let r = analyser_mail("Salut, on se voit demain ? Le plan est sur https://www.exemple.fr/plan");
        assert_eq!(r.niveau, Niveau::Sain);
        assert_eq!(r.urls, vec!["https://www.exemple.fr/plan".to_string()]);
    }

    #[test]
    fn lien_ip_et_http() {
        let r = analyser_mail("Connectez-vous : http://192.168.4.20/login");
        assert_eq!(r.niveau, Niveau::Suspect);
        assert!(r.constats.iter().any(|c| c.titre.contains("adresse IP")));
        assert!(r.constats.iter().any(|c| c.titre.contains("non chiffré")));
    }

    #[test]
    fn lien_trompeur() {
        let r = analyser_mail(r#"<p>Votre compte : <a href="https://secure-login.evil-site.ru/x">www.paypal.com</a></p>"#);
        assert_eq!(r.niveau, Niveau::Suspect);
        assert!(r.constats.iter().any(|c| c.titre == "Lien trompeur"));
    }

    #[test]
    fn lien_coherent_non_signale() {
        let r = analyser_mail(r#"<a href="https://www.paypal.com/fr/home">paypal.com</a>"#);
        assert!(!r.constats.iter().any(|c| c.titre == "Lien trompeur"));
    }

    #[test]
    fn punycode_et_urgence() {
        let r = analyser_mail("Votre compte sera suspendu. Saisissez votre mot de passe sur https://xn--pple-43d.com/");
        assert!(r.constats.iter().any(|c| c.titre.contains("punycode")));
        assert!(r.constats.iter().any(|c| c.titre == "Ton d'urgence"));
    }
}
