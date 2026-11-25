// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Serveur de NaAnSentinels : comptes, rôles, signal de présence facultatif et tableau de bord.
//!
//! Utilisation :
//!   naansentinels-server                      lance le serveur (127.0.0.1:8791 par défaut)
//!   naansentinels-server creer-admin <pseudo> crée le premier administrateur (mot de passe demandé, jamais affiché)
//!
//! Réglages par variables d'environnement :
//!   NAANSENTINELS_ADRESSE       adresse d'écoute (défaut 127.0.0.1:8791)
//!   NAANSENTINELS_BASE          fichier SQLite (défaut naansentinels-serveur.db)
//!   NAANSENTINELS_HTTPS=1       le serveur est derrière HTTPS : cookies marqués « Secure »
//!   NAANSENTINELS_INSCRIPTIONS  « ouvertes » (défaut) ou « fermees »

mod auth;
mod db;
mod limite;
mod routes;

use std::io::{BufRead, IsTerminal};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

pub struct Cfg {
    pub https: bool,
    pub inscriptions_ouvertes: bool,
    /// Empreinte factice, vérifiée quand le pseudo n'existe pas : la durée de réponse ne trahit pas les pseudos.
    pub leurre: String,
}

#[derive(Clone)]
pub struct Etat {
    pub db: Arc<Mutex<rusqlite::Connection>>,
    pub limites: Arc<limite::Limiteur>,
    pub cfg: Arc<Cfg>,
}

fn env(nom: &str, defaut: &str) -> String {
    std::env::var(nom).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| defaut.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base = env("NAANSENTINELS_BASE", "naansentinels-serveur.db");
    let conn = match db::ouvrir(&base) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Base de données inutilisable ({base}) : {e}");
            std::process::exit(1);
        }
    };

    if args.get(1).map(|s| s.as_str()) == Some("creer-admin") {
        let Some(pseudo) = args.get(2) else {
            eprintln!("Usage : naansentinels-server creer-admin <pseudo>");
            std::process::exit(2);
        };
        std::process::exit(creer_admin(&conn, pseudo));
    }

    let adresse: SocketAddr = match env("NAANSENTINELS_ADRESSE", "127.0.0.1:8791").parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("NAANSENTINELS_ADRESSE invalide : {e}");
            std::process::exit(2);
        }
    };
    let etat = Etat {
        db: Arc::new(Mutex::new(conn)),
        limites: Arc::new(limite::Limiteur::default()),
        cfg: Arc::new(Cfg {
            https: env("NAANSENTINELS_HTTPS", "0") == "1",
            inscriptions_ouvertes: env("NAANSENTINELS_INSCRIPTIONS", "ouvertes") != "fermees",
            leurre: auth::hacher(&auth::nouveau_jeton()).unwrap_or_default(),
        }),
    };

    let rt = tokio::runtime::Runtime::new().expect("runtime tokio");
    rt.block_on(async move {
        let app = routes::routeur(etat);
        let ecoute = match tokio::net::TcpListener::bind(adresse).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Impossible d'écouter sur {adresse} : {e}");
                std::process::exit(1);
            }
        };
        println!("NaAnSentinels serveur : http://{adresse}");
        if let Err(e) = axum::serve(ecoute, app.into_make_service_with_connect_info::<SocketAddr>()).await {
            eprintln!("Serveur arrêté : {e}");
        }
    });
}

/// Crée un administrateur. Le mot de passe est lu sans écho au terminal (ou sur l'entrée standard si elle est
/// redirigée, pour l'automatisation) ; il n'est jamais affiché ni journalisé.
fn creer_admin(conn: &rusqlite::Connection, pseudo: &str) -> i32 {
    if let Err(e) = auth::valider_pseudo(pseudo) {
        eprintln!("{e}");
        return 2;
    }
    let mdp = if std::io::stdin().is_terminal() {
        let a = rpassword::prompt_password("Mot de passe (10 caractères au moins) : ").unwrap_or_default();
        let b = rpassword::prompt_password("Encore une fois : ").unwrap_or_default();
        if a != b {
            eprintln!("Les deux saisies diffèrent.");
            return 2;
        }
        a
    } else {
        let mut ligne = String::new();
        let _ = std::io::stdin().lock().read_line(&mut ligne);
        ligne.trim_end_matches(['\r', '\n']).to_string()
    };
    if let Err(e) = auth::valider_mot_de_passe(&mdp) {
        eprintln!("{e}");
        return 2;
    }
    let hash = match auth::hacher(&mdp) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    match db::creer_utilisateur(conn, pseudo, &hash, "admin") {
        Ok(_) => {
            let _ = db::journaliser(conn, pseudo, "admin_cree", "créé en ligne de commande");
            println!("Administrateur « {pseudo} » créé.");
            0
        }
        Err(e) => {
            eprintln!("Création impossible : {e}");
            1
        }
    }
}
