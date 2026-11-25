// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Vue simple : pour quelqu'un qui veut juste savoir « je peux l'ouvrir ou pas ? ».
//! Le bouclier en volume, une phrase en langage courant, trois points au plus, et la porte vers les détails.
//! L'animation rejoue les étapes que le moteur a réellement effectuées ; elle n'invente rien.

use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, RichText, Sense, Stroke, Ui, Vec2};

use super::bouclier3d::{self, Embleme};
use super::theme::{self, alpha, couleur, gras, p, ACCENT, ACCENT_2};
use super::{colonne, App, DUREE_ANIMATION};
use crate::engine::verdict::Niveau;

fn embleme_de(n: Niveau) -> Embleme {
    match n {
        Niveau::Sain => Embleme::Coche,
        Niveau::ARegarder => Embleme::Exclamation,
        Niveau::Suspect => Embleme::Triangle,
        Niveau::Dangereux => Embleme::Croix,
    }
}

/// Phrases en langage courant, sans jargon.
fn phrases(n: Niveau) -> (&'static str, &'static str) {
    match n {
        Niveau::Sain => (
            "Rien d'inquiétant trouvé",
            "Aucune de mes règles n'a réagi. Ce n'est pas une garantie absolue : reste prudent si tu ne connais pas l'origine du fichier.",
        ),
        Niveau::ARegarder => (
            "Quelques points à regarder",
            "Rien de grave en soi, mais ce fichier a des particularités. Jette un œil avant de l'ouvrir.",
        ),
        Niveau::Suspect => (
            "Méfiance",
            "Ce fichier fait des choses que font souvent les logiciels malveillants. Ne l'ouvre que si tu es sûr de sa provenance.",
        ),
        Niveau::Dangereux => (
            "Danger : ne l'ouvre pas",
            "Ce fichier correspond à la signature d'un logiciel malveillant connu.",
        ),
    }
}

/// Met la première lettre en majuscule (les raisons des règles sont souvent écrites en minuscules).
fn majuscule(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(premiere) => premiere.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Étapes de l'analyse sous forme de pastilles : faites, en cours, à venir.
fn etapes(ui: &mut Ui, noms: &[&str], courante: usize) {
    let police = FontId::new(13.0, FontFamily::Proportional);
    let mesures: Vec<_> = noms.iter().map(|n| ui.painter().layout_no_wrap((*n).to_owned(), police.clone(), p().texte)).collect();
    let total: f32 = mesures.iter().map(|g| g.size().x + 34.0).sum::<f32>() + 8.0 * noms.len().saturating_sub(1) as f32;
    colonne(ui, total, |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
        ui.horizontal_wrapped(|ui| {
            for (i, g) in mesures.into_iter().enumerate() {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(g.size().x + 34.0, 30.0), Sense::hover());
                let peintre = ui.painter();
                let (fond, bord, texte, pt) = if i < courante {
                    (alpha(couleur(Niveau::Sain), 0.14), alpha(couleur(Niveau::Sain), 0.5), p().texte, couleur(Niveau::Sain))
                } else if i == courante {
                    (alpha(ACCENT, 0.24), ACCENT_2, p().texte, ACCENT_2)
                } else {
                    (Color32::TRANSPARENT, p().bord, p().texte_3, p().texte_3)
                };
                peintre.rect(rect, 15.0, fond, Stroke::new(1.0_f32, bord));
                peintre.circle_filled(Pos2::new(rect.left() + 14.0, rect.center().y), 3.6, pt);
                peintre.galley(Pos2::new(rect.left() + 24.0, rect.center().y - g.size().y / 2.0), g, texte);
            }
        });
    });
}

/// Trois promesses, sous le bouton d'accueil.
fn promesses(ui: &mut Ui) {
    let cartes = [
        ("Jamais exécuté", "Le fichier est lu et désassemblé. Il n'est jamais lancé."),
        ("Reste chez toi", "Rien ne quitte ton PC pendant l'analyse."),
        ("Tu décides", "Je t'explique ce que je vois. Je ne bloque et ne supprime rien."),
    ];
    colonne(ui, 760.0, |ui| {
        ui.columns(3, |colonnes| {
            for (ui, (titre, texte)) in colonnes.iter_mut().zip(cartes) {
                theme::carte(ui, |ui| {
                    // Alignement à gauche explicite : dans des colonnes, le texte serait sinon étiré.
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            theme::point(ui, ACCENT_2);
                            ui.label(RichText::new(titre).font(gras(15.0)));
                        });
                        ui.label(RichText::new(texte).small().color(p().texte_2));
                    });
                });
            }
        });
    });
}

impl App {
    pub(super) fn vue_simple(&mut self, ui: &mut Ui, t: f64) {
        // Les animations ont besoin d'être redessinées en continu.
        ui.ctx().request_repaint();

        // ---------- 1) Analyse en cours : on rejoue les étapes ----------
        if let Some((rapport, debut)) = &self.en_attente {
            let avancee = ((t - *debut) / DUREE_ANIMATION).clamp(0.0, 1.0) as f32;
            let faites: Vec<&str> = rapport.etapes.iter().filter(|(_, ok)| *ok).map(|(n, _)| n.as_str()).collect();
            let courante = ((avancee * faites.len() as f32) as usize).min(faites.len().saturating_sub(1));
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                bouclier3d::dessiner(ui, 330.0, ACCENT, Embleme::Aucun, t, true, 1.0);
                ui.label(RichText::new("Analyse en cours…").font(gras(28.0)));
                ui.label(RichText::new("Le fichier est lu, jamais exécuté.").color(p().texte_2));
                ui.add_space(10.0);
            });
            etapes(ui, &faites, courante);
            ui.add_space(10.0);
            ui.vertical_centered(|ui| theme::progression(ui, 380.0, avancee));
            return;
        }

        // ---------- 2) Rien à montrer : invitation ----------
        let Some(rapport) = self.rapport.clone() else {
            let mut ouvrir = false;
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    bouclier3d::dessiner(ui, 330.0, ACCENT, Embleme::Coche, t, false, 1.0);
                    ui.label(RichText::new("Un doute sur un fichier ?").font(gras(32.0)));
                    ui.label(RichText::new("Glisse-le ici avant de l'ouvrir. Je te dis ce que j'y vois.").size(17.0).color(p().texte_2));
                    ui.add_space(12.0);
                    if theme::bouton_principal(ui, "Choisir un fichier", 250.0).clicked() {
                        ouvrir = true;
                    }
                    ui.add_space(18.0);
                });
                promesses(ui);
            });
            if ouvrir {
                self.choisir_fichier(t);
            }
            return;
        };

        // ---------- 3) Résultat ----------
        let apparition = (((t - self.devoile_a) / 0.5).clamp(0.0, 1.0)) as f32;
        let (titre, phrase) = phrases(rapport.niveau);
        let nom = std::path::Path::new(&rapport.chemin).file_name().and_then(|n| n.to_str()).unwrap_or(&rapport.chemin).to_string();
        let importants: Vec<_> = rapport.constats.iter().filter(|c| c.niveau > Niveau::Sain).take(3).collect();
        let (mut autre, mut details) = (false, false);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                bouclier3d::dessiner(ui, 270.0, couleur(rapport.niveau), embleme_de(rapport.niveau), t, false, apparition);
                ui.label(RichText::new(titre).font(gras(31.0)));
                // Nom du fichier dans une pilule.
                let g = ui.painter().layout_no_wrap(nom.clone(), FontId::new(13.0, FontFamily::Monospace), p().texte_2);
                let (zone, _) = ui.allocate_exact_size(Vec2::new(g.size().x + 26.0, 26.0), Sense::hover());
                ui.painter().rect(zone, 13.0, p().carte, Stroke::new(1.0_f32, p().bord));
                ui.painter().galley(zone.center() - g.size() / 2.0, g, p().texte_2);
                ui.add_space(2.0);
            });
            colonne(ui, 700.0, |ui| {
                ui.vertical_centered(|ui| ui.label(RichText::new(phrase).size(16.5).color(p().texte_2)));
                ui.add_space(10.0);

                if !importants.is_empty() {
                    theme::titre_section(ui, "Ce qu'il faut savoir");
                    for (i, c) in importants.iter().enumerate() {
                        // Chaque point apparaît à son tour.
                        let visible = (((t - self.devoile_a - 0.3 - i as f64 * 0.2) / 0.4).clamp(0.0, 1.0)) as f32;
                        ui.scope(|ui| {
                            ui.set_opacity(visible);
                            theme::carte_marquee(ui, couleur(c.niveau), |ui| {
                                // L'explication d'abord, en langage courant ; le terme technique ensuite, en discret.
                                ui.label(RichText::new(majuscule(&c.pourquoi)).size(15.5));
                                ui.label(RichText::new(&c.titre).small().color(p().texte_3));
                            });
                        });
                    }
                    let reste = rapport.constats.iter().filter(|c| c.niveau > Niveau::Sain).count().saturating_sub(importants.len());
                    if reste > 0 {
                        ui.label(RichText::new(format!("… et {reste} autre(s) point(s) dans les détails.")).color(p().texte_3));
                    }
                }

                ui.add_space(6.0);
                ui.label(RichText::new("Je n'ai rien bloqué ni supprimé : la décision t'appartient.").color(p().texte_3));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if theme::bouton_principal(ui, "Analyser un autre fichier", 0.0).clicked() {
                        autre = true;
                    }
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        if theme::bouton_secondaire(ui, "Voir tous les détails").clicked() {
                            details = true;
                        }
                    });
                });
                ui.add_space(10.0);
            });
        });

        if details {
            self.changer_vue(true);
        }
        if autre {
            self.choisir_fichier(t);
        }
    }
}

/// Étiquette centrée dans une zone (utilitaire des schémas).
pub(super) fn texte_centre(peintre: &egui::Painter, pos: Pos2, texte: &str, police: FontId, c: Color32) {
    peintre.text(pos, Align2::CENTER_CENTER, texte, police, c);
}
