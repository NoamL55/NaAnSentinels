// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Heuristiques pures : entropie, chaînes ASCII, double extension, correspondance de motifs.
//! Aucune n'est une preuve ; elles alimentent un constat qui l'explique.

use sha2::{Digest, Sha256};

/// Empreinte SHA-256 en hexadécimal minuscule.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

/// Entropie de Shannon des octets (0 = uniforme, 8 = aléatoire/chiffré/empaqueté).
pub fn entropie(octets: &[u8]) -> f64 {
    if octets.is_empty() {
        return 0.0;
    }
    let mut freq = [0u64; 256];
    for &b in octets {
        freq[b as usize] += 1;
    }
    let n = octets.len() as f64;
    let mut h = 0.0;
    for &f in freq.iter() {
        if f > 0 {
            let p = f as f64 / n;
            h -= p * p.log2();
        }
    }
    h
}

/// Suites ASCII imprimables d'au moins `min` caractères (pour chercher des chaînes révélatrices).
pub fn chaines_ascii(octets: &[u8], min: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for &b in octets {
        if (0x20..0x7f).contains(&b) {
            cur.push(b as char);
        } else {
            if cur.len() >= min {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() >= min {
        out.push(cur);
    }
    out
}

/// Nom de fichier à double extension trompeuse (« facture.pdf.exe »). Renvoie l'extension finale réelle.
pub fn double_extension(nom: &str) -> Option<String> {
    let bas = nom.to_lowercase();
    let exe_finales = ["exe", "scr", "com", "bat", "cmd", "pif", "js", "vbs", "msi"];
    let parts: Vec<&str> = bas.split('.').collect();
    if parts.len() >= 3 {
        let finale = parts.last().copied().unwrap_or("");
        let avant = parts[parts.len() - 2];
        // Ex. "facture" "pdf" "exe" : avant-dernière ressemble à un document, dernière est exécutable.
        let docs = ["pdf", "doc", "docx", "xls", "xlsx", "jpg", "png", "txt", "zip"];
        if exe_finales.contains(&finale) && docs.contains(&avant) {
            return Some(finale.to_string());
        }
    }
    None
}

/// Convertit une chaîne hex (« 4d5a90 ») en octets, en ignorant espaces et casse. None si invalide.
pub fn hex_vers_octets(s: &str) -> Option<Vec<u8>> {
    let nettoye: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if nettoye.is_empty() || nettoye.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(nettoye.len() / 2);
    let b = nettoye.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let hi = (b[i] as char).to_digit(16)?;
        let lo = (b[i + 1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(out)
}

/// Cherche une sous-séquence d'octets dans un tampon (recherche naïve, suffisante pour des motifs courts).
pub fn contient_octets(foin: &[u8], aiguille: &[u8]) -> bool {
    if aiguille.is_empty() || aiguille.len() > foin.len() {
        return false;
    }
    foin.windows(aiguille.len()).any(|f| f == aiguille)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropie_bornes() {
        assert_eq!(entropie(&[]), 0.0);
        assert!(entropie(&[7u8; 100]).abs() < 1e-9);
        let tous: Vec<u8> = (0..=255u8).collect();
        assert!((entropie(&tous) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn sha256_connu() {
        // Empreinte de la chaîne vide (valeur de référence publique).
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn chaines_et_hex() {
        let c = chaines_ascii(b"\x00\x01bonjour\x00ab\x00monde!!\x02", 5);
        assert_eq!(c, vec!["bonjour".to_string(), "monde!!".to_string()]);
        assert_eq!(hex_vers_octets("4d 5A90"), Some(vec![0x4d, 0x5a, 0x90]));
        assert_eq!(hex_vers_octets("4d5"), None);
        assert_eq!(hex_vers_octets("zz"), None);
        assert!(contient_octets(b"abcMZdef", b"MZ"));
        assert!(!contient_octets(b"abc", b"abcd"));
    }

    #[test]
    fn double_extension_detectee() {
        assert_eq!(double_extension("facture.pdf.exe"), Some("exe".to_string()));
        assert_eq!(double_extension("photo.JPG.SCR"), Some("scr".to_string()));
        assert_eq!(double_extension("setup.exe"), None);
        assert_eq!(double_extension("archive.tar.gz"), None);
    }
}
