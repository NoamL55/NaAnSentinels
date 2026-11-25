// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Base SQLite. Ce qui est stocké, et rien d'autre :
//!   utilisateurs : pseudo, empreinte Argon2 du mot de passe, rôle
//!   sessions     : empreinte SHA-256 du jeton (jamais le jeton), échéance
//!   appareils    : identifiant aléatoire d'installation, version, système, compteurs d'analyses, dernière présence
//!   journal      : actions sensibles (connexions, changements de rôle)
//! Jamais d'adresse IP, de nom de fichier, de chemin ni d'empreinte de fichier.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

/// Un appareil est « connecté » s'il a donné signe de vie depuis moins de 150 secondes (signal toutes les 60 s).
pub const FENETRE_EN_LIGNE: u64 = 150;

pub struct Session {
    pub user_id: i64,
    pub pseudo: String,
    pub role: String,
}

pub struct Utilisateur {
    pub id: i64,
    pub pseudo: String,
    pub hash: String,
    pub role: String,
    pub actif: bool,
}

pub struct Signal<'a> {
    pub install_id: &'a str,
    pub version: &'a str,
    pub os: &'a str,
    pub user_id: Option<i64>,
    pub total: u64,
    pub sain: u64,
    pub a_regarder: u64,
    pub suspect: u64,
    pub dangereux: u64,
}

pub fn ouvrir(chemin: &str) -> rusqlite::Result<Connection> {
    let c = Connection::open(chemin)?;
    c.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         CREATE TABLE IF NOT EXISTS utilisateurs(
           id INTEGER PRIMARY KEY, pseudo TEXT NOT NULL UNIQUE COLLATE NOCASE, hash TEXT NOT NULL,
           role TEXT NOT NULL, actif INTEGER NOT NULL DEFAULT 1, cree INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS sessions(
           jeton_hash TEXT PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES utilisateurs(id) ON DELETE CASCADE,
           expire INTEGER NOT NULL, cree INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS appareils(
           install_id TEXT PRIMARY KEY, user_id INTEGER REFERENCES utilisateurs(id) ON DELETE SET NULL,
           version TEXT NOT NULL, os TEXT NOT NULL, premier_vu INTEGER NOT NULL, dernier_vu INTEGER NOT NULL,
           total INTEGER NOT NULL DEFAULT 0, sain INTEGER NOT NULL DEFAULT 0, a_regarder INTEGER NOT NULL DEFAULT 0,
           suspect INTEGER NOT NULL DEFAULT 0, dangereux INTEGER NOT NULL DEFAULT 0);
         CREATE TABLE IF NOT EXISTS journal(
           id INTEGER PRIMARY KEY, quand INTEGER NOT NULL, acteur TEXT NOT NULL, action TEXT NOT NULL, detail TEXT NOT NULL);",
    )?;
    Ok(c)
}

fn maintenant() -> u64 {
    crate::limite::maintenant()
}

pub fn journaliser(c: &Connection, acteur: &str, action: &str, detail: &str) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO journal(quand, acteur, action, detail) VALUES(?1, ?2, ?3, ?4)",
        params![maintenant() as i64, acteur, action, detail],
    )?;
    Ok(())
}

pub fn creer_utilisateur(c: &Connection, pseudo: &str, hash: &str, role: &str) -> Result<i64, String> {
    c.execute(
        "INSERT INTO utilisateurs(pseudo, hash, role, actif, cree) VALUES(?1, ?2, ?3, 1, ?4)",
        params![pseudo, hash, role, maintenant() as i64],
    )
    .map(|_| c.last_insert_rowid())
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ce pseudo est déjà pris.".to_string()
        } else {
            format!("erreur de base : {e}")
        }
    })
}

pub fn utilisateur_par_pseudo(c: &Connection, pseudo: &str) -> Option<Utilisateur> {
    c.query_row(
        "SELECT id, pseudo, hash, role, actif FROM utilisateurs WHERE pseudo = ?1",
        params![pseudo],
        |r| Ok(Utilisateur { id: r.get(0)?, pseudo: r.get(1)?, hash: r.get(2)?, role: r.get(3)?, actif: r.get::<_, i64>(4)? != 0 }),
    )
    .optional()
    .ok()
    .flatten()
}

pub fn creer_session(c: &Connection, jeton_hash: &str, user_id: i64, duree_s: u64) -> rusqlite::Result<()> {
    let t = maintenant() as i64;
    c.execute("DELETE FROM sessions WHERE expire < ?1", params![t])?;
    c.execute(
        "INSERT INTO sessions(jeton_hash, user_id, expire, cree) VALUES(?1, ?2, ?3, ?4)",
        params![jeton_hash, user_id, t + duree_s as i64, t],
    )?;
    Ok(())
}

/// Session valide (non expirée, compte actif) pour cette empreinte de jeton.
pub fn session(c: &Connection, jeton_hash: &str) -> Option<Session> {
    c.query_row(
        "SELECT u.id, u.pseudo, u.role FROM sessions s JOIN utilisateurs u ON u.id = s.user_id
         WHERE s.jeton_hash = ?1 AND s.expire > ?2 AND u.actif = 1",
        params![jeton_hash, maintenant() as i64],
        |r| Ok(Session { user_id: r.get(0)?, pseudo: r.get(1)?, role: r.get(2)? }),
    )
    .optional()
    .ok()
    .flatten()
}

pub fn supprimer_session(c: &Connection, jeton_hash: &str) {
    let _ = c.execute("DELETE FROM sessions WHERE jeton_hash = ?1", params![jeton_hash]);
}

pub fn enregistrer_signal(c: &Connection, s: &Signal) -> rusqlite::Result<()> {
    let t = maintenant() as i64;
    c.execute(
        "INSERT INTO appareils(install_id, user_id, version, os, premier_vu, dernier_vu, total, sain, a_regarder, suspect, dangereux)
         VALUES(?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(install_id) DO UPDATE SET user_id = excluded.user_id, version = excluded.version, os = excluded.os,
           dernier_vu = excluded.dernier_vu, total = excluded.total, sain = excluded.sain,
           a_regarder = excluded.a_regarder, suspect = excluded.suspect, dangereux = excluded.dangereux",
        params![s.install_id, s.user_id, s.version, s.os, t, s.total as i64, s.sain as i64, s.a_regarder as i64, s.suspect as i64, s.dangereux as i64],
    )?;
    Ok(())
}

fn ligne_appareil(r: &rusqlite::Row, t: i64) -> rusqlite::Result<Value> {
    let id: String = r.get(0)?;
    let dernier: i64 = r.get(4)?;
    Ok(json!({
        "id": id.chars().take(8).collect::<String>(),
        "pseudo": r.get::<_, Option<String>>(1)?,
        "version": r.get::<_, String>(2)?,
        "os": r.get::<_, String>(3)?,
        "dernier_vu": dernier,
        "en_ligne": t - dernier < FENETRE_EN_LIGNE as i64,
        "analyses": { "total": r.get::<_, i64>(5)?, "sain": r.get::<_, i64>(6)?, "a_regarder": r.get::<_, i64>(7)?,
                      "suspect": r.get::<_, i64>(8)?, "dangereux": r.get::<_, i64>(9)? },
    }))
}

const COLONNES_APPAREIL: &str = "a.install_id, u.pseudo, a.version, a.os, a.dernier_vu, a.total, a.sain, a.a_regarder, a.suspect, a.dangereux";

pub fn appareils_de(c: &Connection, user_id: i64) -> rusqlite::Result<Vec<Value>> {
    let t = maintenant() as i64;
    let mut q = c.prepare(&format!(
        "SELECT {COLONNES_APPAREIL} FROM appareils a LEFT JOIN utilisateurs u ON u.id = a.user_id
         WHERE a.user_id = ?1 ORDER BY a.dernier_vu DESC LIMIT 50"
    ))?;
    let lignes = q.query_map(params![user_id], |r| ligne_appareil(r, t))?;
    lignes.collect()
}

fn repartition(c: &Connection, colonne: &str) -> rusqlite::Result<Vec<Value>> {
    // `colonne` vient du code (jamais d'une requête) : « version » ou « os ».
    let mut q = c.prepare(&format!("SELECT {colonne}, COUNT(*) FROM appareils GROUP BY {colonne} ORDER BY COUNT(*) DESC, {colonne} LIMIT 12"))?;
    let lignes = q.query_map([], |r| Ok(json!({ "nom": r.get::<_, String>(0)?, "appareils": r.get::<_, i64>(1)? })))?;
    lignes.collect()
}

pub fn tableau(c: &Connection) -> rusqlite::Result<Value> {
    let t = maintenant() as i64;
    let seuil = t - FENETRE_EN_LIGNE as i64;
    let (appareils, en_ligne, total, sain, a_regarder, suspect, dangereux): (i64, i64, i64, i64, i64, i64, i64) = c.query_row(
        "SELECT COUNT(*), COALESCE(SUM(dernier_vu > ?1), 0), COALESCE(SUM(total), 0), COALESCE(SUM(sain), 0),
                COALESCE(SUM(a_regarder), 0), COALESCE(SUM(suspect), 0), COALESCE(SUM(dangereux), 0) FROM appareils",
        params![seuil],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
    )?;
    let comptes: i64 = c.query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0))?;
    let actifs_7j: i64 = c.query_row("SELECT COUNT(*) FROM appareils WHERE dernier_vu > ?1", params![t - 7 * 86400], |r| r.get(0))?;
    let mut q = c.prepare(&format!(
        "SELECT {COLONNES_APPAREIL} FROM appareils a LEFT JOIN utilisateurs u ON u.id = a.user_id ORDER BY a.dernier_vu DESC LIMIT 200"
    ))?;
    let liste: Vec<Value> = q.query_map([], |r| ligne_appareil(r, t))?.collect::<rusqlite::Result<_>>()?;
    Ok(json!({
        "maintenant": t,
        "connectes": en_ligne,
        "appareils": appareils,
        "actifs_7j": actifs_7j,
        "comptes": comptes,
        "analyses": { "total": total, "sain": sain, "a_regarder": a_regarder, "suspect": suspect, "dangereux": dangereux },
        "versions": repartition(c, "version")?,
        "systemes": repartition(c, "os")?,
        "liste": liste,
    }))
}

pub fn utilisateurs(c: &Connection) -> rusqlite::Result<Vec<Value>> {
    let mut q = c.prepare(
        "SELECT u.id, u.pseudo, u.role, u.actif, u.cree, (SELECT COUNT(*) FROM appareils a WHERE a.user_id = u.id)
         FROM utilisateurs u ORDER BY u.cree LIMIT 500",
    )?;
    let lignes = q.query_map([], |r| {
        Ok(json!({ "id": r.get::<_, i64>(0)?, "pseudo": r.get::<_, String>(1)?, "role": r.get::<_, String>(2)?,
                   "actif": r.get::<_, i64>(3)? != 0, "cree": r.get::<_, i64>(4)?, "appareils": r.get::<_, i64>(5)? }))
    })?;
    lignes.collect()
}

pub fn role_de(c: &Connection, id: i64) -> Option<(String, String)> {
    c.query_row("SELECT pseudo, role FROM utilisateurs WHERE id = ?1", params![id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()
        .ok()
        .flatten()
}

pub fn nombre_admins_actifs(c: &Connection) -> i64 {
    c.query_row("SELECT COUNT(*) FROM utilisateurs WHERE role = 'admin' AND actif = 1", [], |r| r.get(0)).unwrap_or(0)
}

pub fn changer_role(c: &Connection, id: i64, role: &str) -> rusqlite::Result<usize> {
    c.execute("UPDATE utilisateurs SET role = ?1 WHERE id = ?2", params![role, id])
}

/// Active ou désactive un compte ; le désactiver ferme aussi toutes ses sessions.
pub fn changer_actif(c: &Connection, id: i64, actif: bool) -> rusqlite::Result<usize> {
    if !actif {
        c.execute("DELETE FROM sessions WHERE user_id = ?1", params![id])?;
    }
    c.execute("UPDATE utilisateurs SET actif = ?1 WHERE id = ?2", params![actif as i64, id])
}

pub fn journal(c: &Connection) -> rusqlite::Result<Vec<Value>> {
    let mut q = c.prepare("SELECT quand, acteur, action, detail FROM journal ORDER BY id DESC LIMIT 100")?;
    let lignes = q.query_map([], |r| {
        Ok(json!({ "quand": r.get::<_, i64>(0)?, "acteur": r.get::<_, String>(1)?, "action": r.get::<_, String>(2)?, "detail": r.get::<_, String>(3)? }))
    })?;
    lignes.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Connection {
        ouvrir(":memory:").unwrap()
    }

    #[test]
    fn comptes_et_sessions() {
        let c = base();
        let id = creer_utilisateur(&c, "Alice", "hash", "utilisateur").unwrap();
        assert!(creer_utilisateur(&c, "alice", "hash", "utilisateur").unwrap_err().contains("déjà pris"), "pseudo insensible à la casse");
        creer_session(&c, "empreinte", id, 3600).unwrap();
        assert_eq!(session(&c, "empreinte").unwrap().pseudo, "Alice");
        assert!(session(&c, "autre").is_none());
        changer_actif(&c, id, false).unwrap();
        assert!(session(&c, "empreinte").is_none(), "désactiver un compte ferme ses sessions");
    }

    #[test]
    fn session_expiree_refusee() {
        let c = base();
        let id = creer_utilisateur(&c, "bob", "hash", "utilisateur").unwrap();
        c.execute("INSERT INTO sessions(jeton_hash, user_id, expire, cree) VALUES('vieux', ?1, 10, 5)", params![id]).unwrap();
        assert!(session(&c, "vieux").is_none());
    }

    #[test]
    fn signaux_et_tableau() {
        let c = base();
        let id = creer_utilisateur(&c, "carol", "hash", "utilisateur").unwrap();
        let s = |install: &'static str, user: Option<i64>, total: u64| Signal {
            install_id: install, version: "0.1.0", os: "windows", user_id: user, total, sain: total, a_regarder: 0, suspect: 0, dangereux: 0,
        };
        enregistrer_signal(&c, &s("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None, 3)).unwrap();
        enregistrer_signal(&c, &s("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", Some(id), 5)).unwrap();
        enregistrer_signal(&c, &s("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None, 4)).unwrap(); // même appareil : remplacé, pas additionné
        let t = tableau(&c).unwrap();
        assert_eq!(t["appareils"], 2);
        assert_eq!(t["connectes"], 2);
        assert_eq!(t["analyses"]["total"], 9);
        assert_eq!(t["liste"][0]["id"].as_str().unwrap().len(), 8, "identifiant tronqué dans le tableau");
        let miens = appareils_de(&c, id).unwrap();
        assert_eq!(miens.len(), 1);
        assert_eq!(miens[0]["pseudo"], "carol");
        // Un appareil silencieux depuis longtemps n'est plus « connecté ».
        c.execute("UPDATE appareils SET dernier_vu = dernier_vu - 1000", []).unwrap();
        assert_eq!(tableau(&c).unwrap()["connectes"], 0);
    }
}
