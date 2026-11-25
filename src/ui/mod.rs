// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Interface graphique (egui).
//!
//! - theme.rs      : palette, polices, cartes, boutons, dégradés (toute l'allure se règle là)
//! - bouclier3d.rs : le bouclier en volume, éclairé et animé
//! - simple.rs     : vue simple (par défaut) — le bouclier, une phrase, trois points au plus
//! - expert.rs     : vue experte — schémas, carte des sections, désassemblage
//! - compte.rs     : signal de présence et compte, tous deux facultatifs

mod bouclier3d;
mod compte;
mod expert;
mod simple;
pub mod theme;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Align, Align2, FontFamily, FontId, Frame, Layout, Margin, Painter, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

use crate::engine::verdict::{Constat, Niveau, Rapport};
use crate::engine::{Config, Regles};
use crate::etat::EtatLocal;
use crate::phishing::{self, RapportMail};
use crate::signature;
use theme::{alpha, couleur, degrade_arrondi, gras, melange, p, ACCENT, ACCENT_2};

/// Durée de l'animation d'analyse en vue simple (elle rejoue les étapes réellement effectuées).
const DUREE_ANIMATION: f64 = 2.2;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Onglet {
    Analyser,
    Phishing,
    Regles,
    Journal,
    Compte,
    Apropos,
}

#[derive(Clone, Copy)]
enum Icone {
    Bouclier,
    Enveloppe,
    Curseurs,
    Liste,
    Personne,
    Info,
}

pub struct App {
    onglet: Onglet,
    cfg: Config,
    regles: Regles,
    chemin_config: PathBuf,
    chemin_regles: PathBuf,
    etat_regles: String,

    rapport: Option<Rapport>,
    /// Rapport prêt, dévoilé à la fin de l'animation (vue simple) : (rapport, heure de début).
    en_attente: Option<(Rapport, f64)>,
    /// Heure à laquelle le rapport courant a été dévoilé (pour l'apparition en fondu).
    devoile_a: f64,

    regles_texte: String,
    message_regles: String,

    mail_texte: String,
    rapport_mail: Option<RapportMail>,

    journal: Vec<(Niveau, String, String)>,

    local: Arc<Mutex<EtatLocal>>,
    statut_signal: Arc<Mutex<String>>,
    message_compte: Arc<Mutex<String>>,
    vue_experte: bool,
    signature: signature::Statut,
    saisie_serveur: String,
    saisie_pseudo: String,
    saisie_mdp: String,
    /// Fichier passé au lancement (« Ouvrir avec NaAnSentinels ») : analysé dès la première image.
    a_ouvrir: Option<PathBuf>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, a_ouvrir: Option<PathBuf>) -> App {
        let chemin_config = trouver_ressource("config.toml");
        let chemin_regles = trouver_ressource("rules/signatures.toml");
        let cfg = Config::charger(&chemin_config);

        let sombre = !cfg.interface.theme.to_lowercase().starts_with('c');
        theme::installer(&cc.egui_ctx, sombre);

        let (regles, etat_regles) = match Regles::charger(&chemin_regles) {
            Ok(r) => {
                let msg = format!("{} règle(s) chargée(s).", r.total());
                (r, msg)
            }
            Err(e) => (Regles::default(), e),
        };
        let regles_texte = fs::read_to_string(&chemin_regles).unwrap_or_default();

        let etat = EtatLocal::charger();
        let vue_experte = etat.vue_experte;
        let saisie_serveur = etat.serveur.clone();
        let local = Arc::new(Mutex::new(etat));
        let statut_signal = Arc::new(Mutex::new(String::new()));
        crate::signal::demarrer(local.clone(), statut_signal.clone());

        // Onglet d'ouverture au choix (pratique pour faire des captures d'écran de la documentation).
        let onglet = match std::env::var("NAANSENTINELS_ONGLET").unwrap_or_default().as_str() {
            "phishing" => Onglet::Phishing,
            "compte" => Onglet::Compte,
            "apropos" => Onglet::Apropos,
            "regles" if vue_experte => Onglet::Regles,
            _ => Onglet::Analyser,
        };

        App {
            onglet,
            cfg,
            regles,
            chemin_config,
            chemin_regles,
            etat_regles,
            rapport: None,
            en_attente: None,
            devoile_a: 0.0,
            regles_texte,
            message_regles: String::new(),
            mail_texte: String::new(),
            rapport_mail: None,
            journal: Vec::new(),
            local,
            statut_signal,
            message_compte: Arc::new(Mutex::new(String::new())),
            vue_experte,
            signature: signature::statut_executable(),
            saisie_serveur,
            saisie_pseudo: String::new(),
            saisie_mdp: String::new(),
            a_ouvrir,
        }
    }

    /// Analyse un fichier (jamais exécuté). En vue simple, le résultat est dévoilé après l'animation.
    fn analyser_chemin(&mut self, chemin: &Path, t: f64) {
        let r = crate::engine::analyser_fichier(chemin, &self.regles, &self.cfg);
        let nom = chemin.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string();
        self.journal.push((r.niveau, nom, r.sha256.get(..16).unwrap_or("").to_string()));
        {
            let mut l = self.local.lock().unwrap_or_else(|e| e.into_inner());
            l.compteurs.compter(r.niveau);
            l.sauver();
        }
        self.onglet = Onglet::Analyser;
        if self.vue_experte {
            self.rapport = Some(r);
            self.en_attente = None;
            self.devoile_a = t;
        } else {
            self.rapport = None;
            self.en_attente = Some((r, t));
        }
    }

    fn choisir_fichier(&mut self, t: f64) {
        if let Some(chemin) = rfd::FileDialog::new().pick_file() {
            self.analyser_chemin(&chemin, t);
        }
    }

    fn changer_vue(&mut self, experte: bool) {
        if self.vue_experte == experte {
            return;
        }
        self.vue_experte = experte;
        if !experte && matches!(self.onglet, Onglet::Regles | Onglet::Journal) {
            self.onglet = Onglet::Analyser;
        }
        let mut l = self.local.lock().unwrap_or_else(|e| e.into_inner());
        l.vue_experte = experte;
        l.sauver();
    }

    fn recharger_regles_depuis_disque(&mut self) {
        match fs::read_to_string(&self.chemin_regles) {
            Ok(t) => self.regles_texte = t,
            Err(e) => self.message_regles = format!("Lecture impossible : {e}"),
        }
        self.appliquer_regles_texte();
    }

    fn appliquer_regles_texte(&mut self) {
        match toml::from_str::<Regles>(&self.regles_texte) {
            Ok(r) => {
                self.etat_regles = format!("{} règle(s) chargée(s).", r.total());
                self.message_regles = "Règles appliquées.".into();
                self.regles = r;
            }
            Err(e) => self.message_regles = format!("Règles invalides (non appliquées) : {e}"),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let t = ctx.input(|i| i.time);

        // Glisser-déposer un fichier n'importe où dans la fenêtre → analyse.
        let deposes: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        for chemin in deposes {
            self.analyser_chemin(&chemin, t);
        }
        if let Some(chemin) = self.a_ouvrir.take() {
            self.analyser_chemin(&chemin, t);
        }

        // Fin de l'animation : on dévoile le rapport.
        if let Some((_, debut)) = &self.en_attente {
            if t - *debut >= DUREE_ANIMATION || self.vue_experte {
                self.rapport = self.en_attente.take().map(|(r, _)| r);
                self.devoile_a = t;
            }
        }

        egui::SidePanel::left("navigation")
            .exact_width(222.0)
            .resizable(false)
            .frame(Frame::none().fill(p().fond_2).inner_margin(Margin::symmetric(12.0, 16.0)))
            .show(ctx, |ui| self.barre_laterale(ui));

        egui::CentralPanel::default().frame(Frame::none().fill(p().fond)).show(ctx, |ui| {
            theme::fond(ui);
            Frame::none().inner_margin(Margin { left: 34.0, right: 34.0, top: 26.0, bottom: 18.0 }).show(ui, |ui| match self.onglet {
                Onglet::Analyser if self.vue_experte => self.vue_experte_analyse(ui, t),
                Onglet::Analyser => self.vue_simple(ui, t),
                Onglet::Phishing => self.onglet_phishing(ui),
                Onglet::Regles => self.onglet_regles(ui),
                Onglet::Journal => self.onglet_journal(ui),
                Onglet::Compte => self.onglet_compte(ui),
                Onglet::Apropos => self.onglet_apropos(ui),
            });
        });
    }
}

// ---------------------------------------------------------------- Barre latérale

impl App {
    fn barre_laterale(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (zone, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
            theme::halo(ui.painter(), zone.center(), Vec2::splat(26.0), ACCENT, 0.45);
            bouclier3d::logo(ui.painter(), zone.center(), 15.0, ACCENT);
            ui.label(RichText::new("NaAnSentinels").font(gras(17.5)).color(p().texte));
        });
        ui.add_space(18.0);

        let mut entrees = vec![(Onglet::Analyser, Icone::Bouclier, "Analyser"), (Onglet::Phishing, Icone::Enveloppe, "Anti-phishing")];
        if self.vue_experte {
            entrees.push((Onglet::Regles, Icone::Curseurs, "Règles"));
            entrees.push((Onglet::Journal, Icone::Liste, "Journal"));
        }
        entrees.push((Onglet::Compte, Icone::Personne, "Compte"));
        entrees.push((Onglet::Apropos, Icone::Info, "À propos"));
        ui.spacing_mut().item_spacing.y = 4.0;
        for (onglet, icone, texte) in entrees {
            if element_nav(ui, icone, texte, self.onglet == onglet) {
                self.onglet = onglet;
            }
        }

        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).small().color(p().texte_3));
            ui.add_space(6.0);
            let mut experte = self.vue_experte;
            bascule(ui, &mut experte);
            self.changer_vue(experte);
            ui.label(RichText::new("Niveau de détail").small().color(p().texte_3));
        });
    }
}

/// Entrée du menu : icône dessinée + libellé, surlignée en dégradé quand elle est active.
fn element_nav(ui: &mut Ui, icone: Icone, texte: &str, actif: bool) -> bool {
    let (rect, rep) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
    let survol = ui.ctx().animate_bool(rep.id, rep.hovered());
    let peintre = ui.painter();
    if actif {
        degrade_arrondi(peintre, rect, 10.0, alpha(ACCENT, 0.34), alpha(ACCENT_2, 0.08));
        peintre.rect_filled(Rect::from_min_max(Pos2::new(rect.left(), rect.top() + 9.0), Pos2::new(rect.left() + 3.0, rect.bottom() - 9.0)), 2.0, ACCENT_2);
    } else if survol > 0.0 {
        peintre.rect_filled(rect, 10.0, alpha(p().carte_2, survol));
    }
    let c = if actif { p().texte } else { melange(p().texte_2, p().texte, survol) };
    dessiner_icone(peintre, Pos2::new(rect.left() + 24.0, rect.center().y), icone, c);
    let police = if actif { gras(15.0) } else { FontId::new(15.0, FontFamily::Proportional) };
    peintre.text(Pos2::new(rect.left() + 46.0, rect.center().y), Align2::LEFT_CENTER, texte, police, c);
    rep.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Icônes du menu, tracées à la main (aucune police d'icônes ni image à embarquer).
fn dessiner_icone(peintre: &Painter, c: Pos2, icone: Icone, teinte: egui::Color32) {
    let trait_ = Stroke::new(1.7_f32, teinte);
    let q = |x: f32, y: f32| Pos2::new(c.x + x, c.y + y);
    match icone {
        Icone::Bouclier => {
            let pts: Vec<Pos2> = bouclier3d::contour().iter().map(|v| q(v[0] * 8.5, v[1] * 8.5)).collect();
            peintre.add(egui::Shape::closed_line(pts, trait_));
        }
        Icone::Enveloppe => {
            peintre.rect_stroke(Rect::from_center_size(c, Vec2::new(17.0, 12.5)), 2.0, trait_);
            peintre.line_segment([q(-8.0, -5.5), q(0.0, 1.0)], trait_);
            peintre.line_segment([q(8.0, -5.5), q(0.0, 1.0)], trait_);
        }
        Icone::Curseurs => {
            for (y, x) in [(-5.5_f32, 3.0_f32), (0.0, -3.5), (5.5, 1.0)] {
                peintre.line_segment([q(-8.0, y), q(8.0, y)], trait_);
                peintre.circle_filled(q(x, y), 2.6, teinte);
            }
        }
        Icone::Liste => {
            for y in [-5.5_f32, 0.0, 5.5] {
                peintre.circle_filled(q(-7.0, y), 1.5, teinte);
                peintre.line_segment([q(-3.0, y), q(8.0, y)], trait_);
            }
        }
        Icone::Personne => {
            peintre.circle_stroke(q(0.0, -4.0), 3.6, trait_);
            let epaules: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
                    q(a.cos() * 7.5, 8.0 + a.sin() * 6.5)
                })
                .collect();
            peintre.add(egui::Shape::line(epaules, trait_));
        }
        Icone::Info => {
            peintre.circle_stroke(c, 8.0, trait_);
            peintre.line_segment([q(0.0, -1.0), q(0.0, 4.0)], trait_);
            peintre.circle_filled(q(0.0, -4.2), 1.3, teinte);
        }
    }
}

/// Sélecteur « Simple | Experte » en deux moitiés.
fn bascule(ui: &mut Ui, experte: &mut bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::hover());
    ui.painter().rect(rect, 19.0, p().carte, Stroke::new(1.0_f32, p().bord));
    for (i, (texte, valeur)) in [("Simple", false), ("Experte", true)].into_iter().enumerate() {
        let moitie = rect.width() / 2.0;
        let r = Rect::from_min_size(Pos2::new(rect.left() + moitie * i as f32, rect.top()), Vec2::new(moitie, rect.height())).shrink(3.0);
        let rep = ui.interact(r, ui.id().with(("vue", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let choisi = *experte == valeur;
        if choisi {
            degrade_arrondi(ui.painter(), r, 16.0, ACCENT, ACCENT_2);
        }
        let c = if choisi { egui::Color32::WHITE } else { p().texte_2 };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, texte, if choisi { gras(14.0) } else { FontId::new(14.0, FontFamily::Proportional) }, c);
        if rep.clicked() {
            *experte = valeur;
        }
    }
}

/// Colonne centrée de largeur maximale donnée.
fn colonne<R>(ui: &mut Ui, largeur_max: f32, contenu: impl FnOnce(&mut Ui) -> R) -> R {
    let largeur = largeur_max.min(ui.available_width());
    let marge = ((ui.available_width() - largeur) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(marge);
        ui.vertical(|ui| {
            ui.set_width(largeur);
            contenu(ui)
        })
        .inner
    })
    .inner
}

/// Un constat : ce qui est observé, puis pourquoi ça compte.
fn carte_constat(ui: &mut Ui, c: &Constat) {
    theme::carte_marquee(ui, couleur(c.niveau), |ui| {
        ui.horizontal(|ui| {
            theme::pastille(ui, c.niveau);
            ui.label(RichText::new(&c.titre).font(gras(15.0)));
        });
        ui.label(&c.observation);
        ui.label(RichText::new(&c.pourquoi).color(p().texte_2));
    });
}

// ---------------------------------------------------------------- Pages

impl App {
    fn onglet_phishing(&mut self, ui: &mut Ui) {
        theme::entete(ui, "Anti-phishing", "Colle un mail ou ouvre un fichier .eml. L'analyse se fait sur ton PC, sans aucune connexion.");
        egui::ScrollArea::vertical().show(ui, |ui| {
            theme::carte(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.mail_texte)
                        .desired_rows(9)
                        .desired_width(f32::INFINITY)
                        .hint_text(RichText::new("En-têtes et corps du mail…").color(p().texte_3)),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if theme::bouton_principal(ui, "Analyser le mail", 0.0).clicked() {
                        self.rapport_mail = Some(phishing::analyser_mail(&self.mail_texte));
                    }
                    if theme::bouton_secondaire(ui, "Ouvrir un .eml…").clicked() {
                        if let Some(chemin) = rfd::FileDialog::new().add_filter("E-mail", &["eml", "txt"]).pick_file() {
                            if let Ok(t) = fs::read_to_string(&chemin) {
                                self.mail_texte = t;
                            }
                        }
                    }
                    if theme::bouton_secondaire(ui, "Effacer").clicked() {
                        self.mail_texte.clear();
                        self.rapport_mail = None;
                    }
                });
            });
            if let Some(r) = &self.rapport_mail {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    theme::titre_section(ui, "Verdict");
                    theme::pastille(ui, r.niveau);
                });
                for c in &r.constats {
                    carte_constat(ui, c);
                }
                if self.vue_experte && !r.urls.is_empty() {
                    theme::carte(ui, |ui| {
                        theme::titre_section(ui, &format!("Liens trouvés ({})", r.urls.len()));
                        for u in &r.urls {
                            ui.label(RichText::new(u).monospace().small().color(p().texte_2));
                        }
                    });
                }
            }
        });
    }

    fn onglet_regles(&mut self, ui: &mut Ui) {
        theme::entete(ui, "Règles", "Tes règles de détection. Modifie-les ici, puis enregistre : elles s'appliquent tout de suite.");
        ui.horizontal(|ui| {
            if theme::bouton_principal(ui, "Enregistrer et appliquer", 0.0).clicked() {
                match fs::write(&self.chemin_regles, &self.regles_texte) {
                    Ok(_) => self.appliquer_regles_texte(),
                    Err(e) => self.message_regles = format!("Écriture impossible : {e}"),
                }
            }
            if theme::bouton_secondaire(ui, "Recharger depuis le disque").clicked() {
                self.recharger_regles_depuis_disque();
            }
            ui.label(RichText::new(if self.message_regles.is_empty() { &self.etat_regles } else { &self.message_regles }).color(p().texte_2));
        });
        ui.label(RichText::new(format!("Règles : {}    ·    Config : {}", self.chemin_regles.display(), self.chemin_config.display())).small().color(p().texte_3));
        ui.add_space(4.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            theme::carte(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.regles_texte).code_editor().desired_rows(24).desired_width(f32::INFINITY).frame(false));
            });
        });
    }

    fn onglet_journal(&mut self, ui: &mut Ui) {
        theme::entete(ui, "Journal", "Les fichiers analysés depuis l'ouverture de l'application.");
        if !self.journal.is_empty() && theme::bouton_secondaire(ui, "Vider le journal").clicked() {
            self.journal.clear();
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.journal.is_empty() {
                theme::carte(ui, |ui| {
                    ui.label(RichText::new("Rien encore. Analyse un fichier pour le voir apparaître ici.").color(p().texte_2));
                });
            }
            for (niveau, nom, empreinte) in self.journal.iter().rev() {
                theme::carte(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::pastille(ui, *niveau);
                        ui.label(RichText::new(nom).font(gras(15.0)));
                        ui.label(RichText::new(format!("SHA-256 {empreinte}…")).monospace().small().color(p().texte_3));
                    });
                });
            }
        });
    }

    fn onglet_apropos(&mut self, ui: &mut Ui) {
        theme::entete(ui, "À propos", "L'antivirus qui t'explique ce qu'il voit au lieu de t'interdire sans raison.");
        egui::ScrollArea::vertical().show(ui, |ui| {
            let (niveau, phrase) = match self.signature {
                signature::Statut::Officielle => (Niveau::Sain, "Version officielle : sa signature est valide pour la clé officielle."),
                signature::Statut::Absente => (Niveau::ARegarder, "Non signé : compilation personnelle ou version dérivée du projet original."),
                signature::Statut::Invalide => (Niveau::Dangereux, "Signature invalide : ce fichier a été modifié ou signé par une autre clé."),
            };
            theme::carte_marquee(ui, couleur(niveau), |ui| {
                theme::titre_section(ui, "Origine de cet exécutable");
                ui.horizontal(|ui| {
                    theme::point(ui, couleur(niveau));
                    ui.label(phrase);
                });
                ui.label(RichText::new(format!("Empreinte de la clé officielle : {}", signature::empreinte_cle(signature::CLE_OFFICIELLE))).monospace().small().color(p().texte_2));
                ui.label(RichText::new(format!("Version {} · open source, licence MIT · projet original de Noam Lellouche", env!("CARGO_PKG_VERSION"))).small().color(p().texte_3));
            });
            theme::carte(ui, |ui| {
                theme::titre_section(ui, "Ce qu'il fait vraiment");
                for ligne in [
                    "Il analyse un fichier avant que tu l'exécutes : il le lit et le désassemble, il ne le lance jamais.",
                    "Il liste les fonctions dangereuses importées, les sections anormales, la forte entropie, les signatures connues.",
                    "Il montre l'assembleur du point d'entrée.",
                    "Il repère l'hameçonnage dans un mail que tu déposes, sans connexion.",
                    "Ses règles et sa configuration sont entièrement modifiables.",
                ] {
                    puce(ui, couleur(Niveau::Sain), ligne);
                }
            });
            theme::carte(ui, |ui| {
                theme::titre_section(ui, "Ses limites, dites franchement");
                for ligne in [
                    "Il ne connaît que ses règles : « Sain » veut dire « rien de connu », pas « garanti sûr ».",
                    "Il ne remplace pas l'antivirus du système et ne surveille pas encore en temps réel.",
                    "Seule une signature exacte donne « Dangereux » ; les heuristiques donnent au mieux « Suspect ».",
                    "Il ne bloque, ne déplace ni ne supprime aucun fichier : la décision reste à toi.",
                    "L'absence de signature Authenticode n'est jamais traitée comme une preuve de malveillance.",
                ] {
                    puce(ui, couleur(Niveau::ARegarder), ligne);
                }
            });
        });
    }
}

/// Ligne de liste précédée d'un point de couleur.
fn puce(ui: &mut Ui, teinte: egui::Color32, texte: &str) {
    ui.horizontal_wrapped(|ui| {
        theme::point(ui, teinte);
        ui.label(RichText::new(texte).color(p().texte_2));
    });
}

/// Icône de la fenêtre : le bouclier, dessiné point par point (aucun fichier image à embarquer).
pub fn icone() -> egui::IconData {
    const T: usize = 64;
    let bord = bouclier3d::contour();
    let dedans = |x: f32, y: f32| {
        (0..bord.len()).all(|i| {
            let (a, b) = (bord[i], bord[(i + 1) % bord.len()]);
            (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0]) >= 0.0
        })
    };
    // Distance d'un point à un segment : sert à tracer la coche blanche.
    let pres = |x: f32, y: f32, a: (f32, f32), b: (f32, f32)| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let u = (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        ((x - a.0 - u * dx).powi(2) + (y - a.1 - u * dy).powi(2)).sqrt() < 0.10
    };
    let mut rgba = vec![0u8; T * T * 4];
    for j in 0..T {
        for i in 0..T {
            let (x, y) = ((i as f32 + 0.5) / T as f32 * 2.0 - 1.0, (j as f32 + 0.5) / T as f32 * 2.0 - 1.0);
            let (x, y) = (x / 0.94, y / 0.94);
            if dedans(x, y) {
                let coche = pres(x, y, (-0.36, 0.02), (-0.08, 0.30)) || pres(x, y, (-0.08, 0.30), (0.40, -0.26));
                // Dégradé du bleu vers le cyan, de haut en bas.
                let teinte = melange(ACCENT, ACCENT_2, (y + 1.0) / 2.0);
                let c = if coche { [255, 255, 255] } else { [teinte.r(), teinte.g(), teinte.b()] };
                let k = (j * T + i) * 4;
                rgba[k..k + 3].copy_from_slice(&c);
                rgba[k + 3] = 255;
            }
        }
    }
    egui::IconData { rgba, width: T as u32, height: T as u32 }
}

/// Cherche un fichier de ressource : d'abord le dossier courant, puis à côté de l'exécutable (et au-dessus).
pub fn trouver_ressource(nom: &str) -> PathBuf {
    let direct = PathBuf::from(nom);
    if direct.exists() {
        return direct;
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dossier = exe.parent().map(|d| d.to_path_buf());
        for _ in 0..4 {
            if let Some(d) = &dossier {
                let candidat = d.join(nom);
                if candidat.exists() {
                    return candidat;
                }
                dossier = d.parent().map(|x| x.to_path_buf());
            }
        }
    }
    direct
}
