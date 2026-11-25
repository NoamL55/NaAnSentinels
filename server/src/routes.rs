// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Routes HTTP. Chaque route protégée vérifie la session PUIS le droit demandé.
//! Défenses : en-têtes de sécurité (CSP stricte), cookie HttpOnly + SameSite=Strict, en-tête obligatoire
//! « X-NaAnSentinels » sur toute écriture (anti-CSRF), limites de débit, corps de requête plafonné.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Request, State};
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{auth, db, Etat};

const DUREE_SESSION: u64 = 7 * 24 * 3600;
const SYSTEMES: [&str; 4] = ["windows", "linux", "macos", "autre"];

type Rep = Result<Response, Response>;

fn erreur(code: StatusCode, message: &str) -> Response {
    (code, Json(json!({ "erreur": message }))).into_response()
}

fn interne<E: std::fmt::Display>(e: E) -> Response {
    eprintln!("erreur interne : {e}");
    erreur(StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne.")
}

pub fn routeur(etat: Etat) -> Router {
    Router::new()
        .route("/", get(page))
        .route("/app.css", get(feuille))
        .route("/app.js", get(script))
        .route("/api/v1/signal", post(signal))
        .route("/api/v1/comptes", post(creer_compte))
        .route("/api/v1/connexion", post(connexion))
        .route("/api/v1/deconnexion", post(deconnexion))
        .route("/api/v1/moi", get(moi))
        .route("/api/v1/mes-appareils", get(mes_appareils))
        .route("/api/v1/tableau", get(tableau))
        .route("/api/v1/utilisateurs", get(utilisateurs))
        .route("/api/v1/utilisateurs/:id/role", post(changer_role))
        .route("/api/v1/utilisateurs/:id/actif", post(changer_actif))
        .route("/api/v1/journal", get(journal))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn(securite))
        .with_state(etat)
}

/// En-têtes de sécurité sur toute réponse, et garde anti-CSRF sur toute écriture de l'API.
async fn securite(req: Request, next: Next) -> Response {
    let api = req.uri().path().starts_with("/api/");
    let ecriture = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if api && ecriture && req.headers().get("x-naansentinels").is_none() {
        return erreur(StatusCode::FORBIDDEN, "En-tête X-NaAnSentinels manquant.");
    }
    let mut rep = next.run(req).await;
    let h = rep.headers_mut();
    h.insert("content-security-policy", HeaderValue::from_static(
        "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"));
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    if api {
        h.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    rep
}

// ---------------------------------------------------------------- Pages (embarquées dans l'exécutable)

async fn page() -> Response {
    ([(CONTENT_TYPE, "text/html; charset=utf-8")], include_str!("../web/index.html")).into_response()
}
async fn feuille() -> Response {
    ([(CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("../web/app.css")).into_response()
}
async fn script() -> Response {
    ([(CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../web/app.js")).into_response()
}

// ---------------------------------------------------------------- Session et droits

fn jeton_de(headers: &HeaderMap) -> Option<String> {
    if let Some(v) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(j) = v.strip_prefix("Bearer ") {
            return Some(j.trim().to_string());
        }
    }
    let cookies = headers.get(COOKIE)?.to_str().ok()?;
    cookies.split(';').filter_map(|c| c.trim().strip_prefix("nas_session=")).next().map(|s| s.to_string())
}

fn jeton_plausible(j: &str) -> bool {
    j.len() == 64 && j.chars().all(|c| c.is_ascii_hexdigit())
}

fn session(etat: &Etat, headers: &HeaderMap) -> Option<db::Session> {
    let jeton = jeton_de(headers).filter(|j| jeton_plausible(j))?;
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    db::session(&c, &auth::empreinte(&jeton))
}

/// Session obligatoire avec le droit demandé : 401 sans session, 403 sans le droit.
fn exiger(etat: &Etat, headers: &HeaderMap, droit: &str) -> Result<db::Session, Response> {
    let s = session(etat, headers).ok_or_else(|| erreur(StatusCode::UNAUTHORIZED, "Connexion requise."))?;
    if !auth::a_le_droit(&s.role, droit) {
        return Err(erreur(StatusCode::FORBIDDEN, "Ton rôle ne donne pas accès à cette page."));
    }
    Ok(s)
}

fn cookie_session(etat: &Etat, jeton: &str, duree: u64) -> HeaderValue {
    let securise = if etat.cfg.https { "; Secure" } else { "" };
    HeaderValue::from_str(&format!("nas_session={jeton}; HttpOnly; SameSite=Strict; Path=/; Max-Age={duree}{securise}"))
        .unwrap_or_else(|_| HeaderValue::from_static(""))
}

// ---------------------------------------------------------------- Signal de présence (facultatif, anonyme)

#[derive(Deserialize, Default)]
struct Compteurs {
    #[serde(default)]
    total: u64,
    #[serde(default)]
    sain: u64,
    #[serde(default)]
    a_regarder: u64,
    #[serde(default)]
    suspect: u64,
    #[serde(default)]
    dangereux: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Signal {
    install_id: String,
    version: String,
    os: String,
    #[serde(default)]
    analyses: Compteurs,
}

async fn signal(State(etat): State<Etat>, ConnectInfo(ip): ConnectInfo<SocketAddr>, headers: HeaderMap, Json(s): Json<Signal>) -> Rep {
    if !etat.limites.autoriser(&format!("signal:{}", ip.ip()), 30, 60) {
        return Err(erreur(StatusCode::TOO_MANY_REQUESTS, "Trop de signaux, réessaie dans une minute."));
    }
    let id_ok = s.install_id.len() == 32 && s.install_id.chars().all(|c| c.is_ascii_hexdigit());
    let version_ok = (1..=32).contains(&s.version.len()) && s.version.chars().all(|c| c.is_ascii_alphanumeric() || ".+-".contains(c));
    if !id_ok || !version_ok || !SYSTEMES.contains(&s.os.as_str()) {
        return Err(erreur(StatusCode::BAD_REQUEST, "Signal invalide."));
    }
    let plafond = 1_000_000_000u64;
    let a = &s.analyses;
    if [a.total, a.sain, a.a_regarder, a.suspect, a.dangereux].iter().any(|v| *v > plafond) {
        return Err(erreur(StatusCode::BAD_REQUEST, "Compteurs invalides."));
    }
    let user_id = session(&etat, &headers).map(|x| x.user_id);
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    db::enregistrer_signal(&c, &db::Signal {
        install_id: &s.install_id.to_lowercase(), version: &s.version, os: &s.os, user_id,
        total: a.total, sain: a.sain, a_regarder: a.a_regarder, suspect: a.suspect, dangereux: a.dangereux,
    })
    .map_err(interne)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---------------------------------------------------------------- Comptes

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identifiants {
    pseudo: String,
    mot_de_passe: String,
}

async fn creer_compte(State(etat): State<Etat>, ConnectInfo(ip): ConnectInfo<SocketAddr>, Json(i): Json<Identifiants>) -> Rep {
    if !etat.cfg.inscriptions_ouvertes {
        return Err(erreur(StatusCode::FORBIDDEN, "Les inscriptions sont fermées sur ce serveur."));
    }
    if !etat.limites.autoriser(&format!("inscription:{}", ip.ip()), 5, 3600) {
        return Err(erreur(StatusCode::TOO_MANY_REQUESTS, "Trop de créations de compte, réessaie plus tard."));
    }
    auth::valider_pseudo(&i.pseudo).map_err(|e| erreur(StatusCode::BAD_REQUEST, &e))?;
    auth::valider_mot_de_passe(&i.mot_de_passe).map_err(|e| erreur(StatusCode::BAD_REQUEST, &e))?;
    let mdp = i.mot_de_passe;
    let hash = tokio::task::spawn_blocking(move || auth::hacher(&mdp)).await.map_err(interne)?.map_err(interne)?;
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    db::creer_utilisateur(&c, &i.pseudo, &hash, "utilisateur").map_err(|e| erreur(StatusCode::CONFLICT, &e))?;
    let _ = db::journaliser(&c, &i.pseudo, "compte_cree", "inscription");
    Ok((StatusCode::CREATED, Json(json!({ "ok": true }))).into_response())
}

async fn connexion(State(etat): State<Etat>, ConnectInfo(ip): ConnectInfo<SocketAddr>, Json(i): Json<Identifiants>) -> Rep {
    let pseudo = i.pseudo.trim().to_string();
    let cle = format!("connexion:{}:{}", ip.ip(), pseudo.to_lowercase());
    if !etat.limites.autoriser(&cle, 8, 900) {
        return Err(erreur(StatusCode::TOO_MANY_REQUESTS, "Trop d'essais. Réessaie dans un quart d'heure."));
    }
    let utilisateur = {
        let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
        db::utilisateur_par_pseudo(&c, &pseudo)
    };
    // Même travail que le compte existe ou non : la durée de réponse ne révèle pas quels pseudos existent.
    let hash = utilisateur.as_ref().map(|u| u.hash.clone()).unwrap_or_else(|| etat.cfg.leurre.clone());
    let mdp = i.mot_de_passe;
    let correct = tokio::task::spawn_blocking(move || auth::verifier(&mdp, &hash)).await.map_err(interne)?;
    let u = match utilisateur {
        Some(u) if correct && u.actif => u,
        _ => {
            let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
            let _ = db::journaliser(&c, &pseudo.chars().take(24).collect::<String>(), "connexion_refusee", "identifiants refusés");
            return Err(erreur(StatusCode::UNAUTHORIZED, "Pseudo ou mot de passe incorrect."));
        }
    };
    let jeton = auth::nouveau_jeton();
    {
        let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
        db::creer_session(&c, &auth::empreinte(&jeton), u.id, DUREE_SESSION).map_err(interne)?;
        let _ = db::journaliser(&c, &u.pseudo, "connexion", "");
    }
    let corps = Json(json!({ "jeton": jeton, "pseudo": u.pseudo, "role": u.role, "droits": auth::droits(&u.role) }));
    let mut rep = corps.into_response();
    rep.headers_mut().insert(SET_COOKIE, cookie_session(&etat, &jeton, DUREE_SESSION));
    Ok(rep)
}

async fn deconnexion(State(etat): State<Etat>, headers: HeaderMap) -> Rep {
    if let Some(j) = jeton_de(&headers).filter(|j| jeton_plausible(j)) {
        let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
        db::supprimer_session(&c, &auth::empreinte(&j));
    }
    let mut rep = Json(json!({ "ok": true })).into_response();
    rep.headers_mut().insert(SET_COOKIE, cookie_session(&etat, "", 0));
    Ok(rep)
}

async fn moi(State(etat): State<Etat>, headers: HeaderMap) -> Rep {
    let s = session(&etat, &headers).ok_or_else(|| erreur(StatusCode::UNAUTHORIZED, "Connexion requise."))?;
    Ok(Json(json!({ "pseudo": s.pseudo, "role": s.role, "droits": auth::droits(&s.role) })).into_response())
}

// ---------------------------------------------------------------- Données (par droit)

async fn mes_appareils(State(etat): State<Etat>, headers: HeaderMap) -> Rep {
    let s = exiger(&etat, &headers, "appareils:les-miens")?;
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    Ok(Json(json!({ "appareils": db::appareils_de(&c, s.user_id).map_err(interne)? })).into_response())
}

async fn tableau(State(etat): State<Etat>, headers: HeaderMap) -> Rep {
    exiger(&etat, &headers, "tableau:lire")?;
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    Ok(Json(db::tableau(&c).map_err(interne)?).into_response())
}

async fn utilisateurs(State(etat): State<Etat>, headers: HeaderMap) -> Rep {
    exiger(&etat, &headers, "utilisateurs:gerer")?;
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    Ok(Json(json!({ "utilisateurs": db::utilisateurs(&c).map_err(interne)?, "roles": auth::ROLES })).into_response())
}

async fn journal(State(etat): State<Etat>, headers: HeaderMap) -> Rep {
    exiger(&etat, &headers, "journal:lire")?;
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    Ok(Json(json!({ "journal": db::journal(&c).map_err(interne)? })).into_response())
}

async fn changer_role(State(etat): State<Etat>, headers: HeaderMap, Path(id): Path<i64>, Json(corps): Json<Value>) -> Rep {
    let moi = exiger(&etat, &headers, "utilisateurs:gerer")?;
    let role = corps.get("role").and_then(|r| r.as_str()).unwrap_or("");
    if !auth::ROLES.contains(&role) {
        return Err(erreur(StatusCode::BAD_REQUEST, "Rôle inconnu."));
    }
    if id == moi.user_id {
        return Err(erreur(StatusCode::CONFLICT, "Tu ne peux pas changer ton propre rôle."));
    }
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    let (pseudo, ancien) = db::role_de(&c, id).ok_or_else(|| erreur(StatusCode::NOT_FOUND, "Compte introuvable."))?;
    if ancien == "admin" && role != "admin" && db::nombre_admins_actifs(&c) <= 1 {
        return Err(erreur(StatusCode::CONFLICT, "Il doit rester au moins un administrateur."));
    }
    db::changer_role(&c, id, role).map_err(interne)?;
    let _ = db::journaliser(&c, &moi.pseudo, "role_change", &format!("{pseudo} : {ancien} -> {role}"));
    Ok(Json(json!({ "ok": true })).into_response())
}

async fn changer_actif(State(etat): State<Etat>, headers: HeaderMap, Path(id): Path<i64>, Json(corps): Json<Value>) -> Rep {
    let moi = exiger(&etat, &headers, "utilisateurs:gerer")?;
    let actif = corps.get("actif").and_then(|a| a.as_bool()).ok_or_else(|| erreur(StatusCode::BAD_REQUEST, "Champ « actif » manquant."))?;
    if id == moi.user_id {
        return Err(erreur(StatusCode::CONFLICT, "Tu ne peux pas désactiver ton propre compte."));
    }
    let c = etat.db.lock().unwrap_or_else(|e| e.into_inner());
    let (pseudo, role) = db::role_de(&c, id).ok_or_else(|| erreur(StatusCode::NOT_FOUND, "Compte introuvable."))?;
    if !actif && role == "admin" && db::nombre_admins_actifs(&c) <= 1 {
        return Err(erreur(StatusCode::CONFLICT, "Il doit rester au moins un administrateur actif."));
    }
    db::changer_actif(&c, id, actif).map_err(interne)?;
    let _ = db::journaliser(&c, &moi.pseudo, if actif { "compte_active" } else { "compte_desactive" }, &pseudo);
    Ok(Json(json!({ "ok": true })).into_response())
}
