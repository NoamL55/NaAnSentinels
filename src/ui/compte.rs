// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Onglet Compte : le signal de présence (désactivé par défaut, contenu montré tel quel) et le compte facultatif.
//! NaAnSentinels fonctionne entièrement sans compte et sans serveur.

use eframe::egui::{self, RichText, Ui};

use super::theme::{self, couleur, gras, p};
use super::App;
use crate::engine::verdict::Niveau;
use crate::signal;

impl App {
    pub(super) fn onglet_compte(&mut self, ui: &mut Ui) {
        // Le statut du signal change en arrière-plan : on rafraîchit doucement.
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
        let copie = self.local.lock().unwrap_or_else(|e| e.into_inner()).clone();

        theme::entete(ui, "Compte et partage", "Tout ceci est facultatif. Sans serveur ni compte, NaAnSentinels analyse tes fichiers exactement pareil.");
        egui::ScrollArea::vertical().show(ui, |ui| {
            // ---------- Serveur ----------
            theme::carte(ui, |ui| {
                theme::titre_section(ui, "Serveur");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.saisie_serveur).desired_width(340.0).margin(egui::Margin::symmetric(10.0, 8.0)).hint_text(RichText::new("https://…").color(p().texte_3)));
                    if theme::bouton_secondaire(ui, "Enregistrer").clicked() {
                        let message = if self.saisie_serveur.trim().is_empty() {
                            let mut l = self.local.lock().unwrap_or_else(|e| e.into_inner());
                            l.serveur.clear();
                            l.signal_actif = false;
                            l.sauver();
                            "Serveur retiré : rien n'est envoyé.".to_string()
                        } else {
                            match signal::url_sure(&self.saisie_serveur) {
                                Ok(url) => {
                                    let mut l = self.local.lock().unwrap_or_else(|e| e.into_inner());
                                    l.serveur = url.clone();
                                    l.sauver();
                                    self.saisie_serveur = url;
                                    "Serveur enregistré.".to_string()
                                }
                                Err(e) => e,
                            }
                        };
                        *self.message_compte.lock().unwrap_or_else(|e| e.into_inner()) = message;
                    }
                });
                ui.label(RichText::new("HTTPS obligatoire (HTTP accepté seulement vers cette machine, pour des essais).").small().color(p().texte_3));
            });

            // ---------- Signal de présence ----------
            let possible = !copie.serveur.is_empty();
            let mut actif = copie.signal_actif;
            theme::carte(ui, |ui| {
                theme::titre_section(ui, "Signal de présence anonyme");
                ui.add_enabled_ui(possible, |ui| {
                    if ui.checkbox(&mut actif, "Partager un signal anonyme toutes les minutes").changed() {
                        let mut l = self.local.lock().unwrap_or_else(|e| e.into_inner());
                        l.signal_actif = actif;
                        l.sauver();
                    }
                });
                let statut = self.statut_signal.lock().unwrap_or_else(|e| e.into_inner()).clone();
                let (teinte, phrase) = if !possible {
                    (p().texte_3, "Indique d'abord un serveur.".to_string())
                } else if !actif {
                    (p().texte_3, "Désactivé : rien n'est envoyé.".to_string())
                } else if statut.is_empty() {
                    (couleur(Niveau::ARegarder), "Premier signal dans quelques secondes…".to_string())
                } else if statut.contains("échec") {
                    (couleur(Niveau::Suspect), statut)
                } else {
                    (couleur(Niveau::Sain), statut)
                };
                ui.horizontal(|ui| {
                    theme::point(ui, teinte);
                    ui.label(RichText::new(phrase).color(p().texte_2));
                });
                ui.add_space(2.0);
                ui.label("Voici exactement ce qui part, et rien d'autre :");
                let apercu = serde_json::to_string_pretty(&signal::charge_utile(&copie)).unwrap_or_default();
                egui::Frame::none().fill(p().fond_2).rounding(10.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                    ui.label(RichText::new(apercu).monospace().small().color(p().texte_2));
                });
                ui.label(RichText::new("Jamais envoyé : tes fichiers, leurs noms, leurs chemins, leurs empreintes, tes mails.").small().color(p().texte_3));
            });

            // ---------- Compte ----------
            theme::carte(ui, |ui| {
                theme::titre_section(ui, "Compte (facultatif)");
                match (&copie.pseudo, &copie.jeton) {
                    (Some(pseudo), Some(jeton)) => {
                        ui.horizontal(|ui| {
                            theme::point(ui, couleur(Niveau::Sain));
                            ui.label(RichText::new(format!("Connecté en tant que {pseudo}")).font(gras(15.0)));
                        });
                        ui.label(RichText::new("Tes appareils apparaissent sous ce pseudo sur le serveur.").color(p().texte_2));
                        if theme::bouton_secondaire(ui, "Se déconnecter").clicked() {
                            let (serveur, jeton) = (copie.serveur.clone(), jeton.clone());
                            std::thread::spawn(move || signal::deconnexion(&serveur, &jeton));
                            let mut l = self.local.lock().unwrap_or_else(|e| e.into_inner());
                            l.pseudo = None;
                            l.jeton = None;
                            l.sauver();
                            *self.message_compte.lock().unwrap_or_else(|e| e.into_inner()) = "Déconnecté : l'appareil redevient anonyme.".into();
                        }
                    }
                    _ => {
                        let mut connecter = false;
                        ui.add_enabled_ui(possible, |ui| {
                            egui::Grid::new("connexion").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                                ui.label(RichText::new("Pseudo").color(p().texte_2));
                                ui.add(egui::TextEdit::singleline(&mut self.saisie_pseudo).desired_width(240.0).margin(egui::Margin::symmetric(10.0, 8.0)));
                                ui.end_row();
                                ui.label(RichText::new("Mot de passe").color(p().texte_2));
                                ui.add(egui::TextEdit::singleline(&mut self.saisie_mdp).password(true).desired_width(240.0).margin(egui::Margin::symmetric(10.0, 8.0)));
                                ui.end_row();
                            });
                            if theme::bouton_principal(ui, "Se connecter", 0.0).clicked() {
                                connecter = true;
                            }
                        });
                        if connecter && possible && !self.saisie_pseudo.trim().is_empty() {
                            // Le mot de passe quitte le champ tout de suite ; il n'est ni gardé ni écrit sur le disque.
                            let mdp = std::mem::take(&mut self.saisie_mdp);
                            let (serveur, pseudo) = (copie.serveur.clone(), self.saisie_pseudo.trim().to_string());
                            let (local, message, ctx) = (self.local.clone(), self.message_compte.clone(), ui.ctx().clone());
                            *message.lock().unwrap_or_else(|e| e.into_inner()) = "Connexion…".into();
                            std::thread::spawn(move || {
                                let texte = match signal::connexion(&serveur, &pseudo, &mdp) {
                                    Ok(c) => {
                                        let mut l = local.lock().unwrap_or_else(|e| e.into_inner());
                                        l.pseudo = Some(c.pseudo.clone());
                                        l.jeton = Some(c.jeton);
                                        l.sauver();
                                        format!("Connecté : {} ({}).", c.pseudo, c.role)
                                    }
                                    Err(e) => e,
                                };
                                *message.lock().unwrap_or_else(|e| e.into_inner()) = texte;
                                ctx.request_repaint();
                            });
                        }
                        ui.label(RichText::new("Pas encore de compte ? Crée-le sur la page du serveur, dans ton navigateur.").small().color(p().texte_3));
                    }
                }
                let message = self.message_compte.lock().unwrap_or_else(|e| e.into_inner()).clone();
                if !message.is_empty() {
                    ui.label(RichText::new(message).color(p().texte_2));
                }
            });

            ui.label(RichText::new(format!("Identifiant aléatoire de cette installation : {}", copie.install_id)).monospace().small().color(p().texte_3));
            ui.add_space(8.0);
        });
    }
}
