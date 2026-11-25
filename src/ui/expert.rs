// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Vue experte : tout le rapport, avec des schémas pour le lire vite.
//! Échelle du verdict, parcours de l'analyse, jauge d'entropie, carte des sections, désassemblage.

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

use super::simple::texte_centre;
use super::theme::{self, alpha, couleur, degrade_arrondi, gras, p, ACCENT, ACCENT_2};
use super::{carte_constat, App};
use crate::engine::verdict::{Niveau, Rapport, SectionVue};

const NIVEAUX: [Niveau; 4] = [Niveau::Sain, Niveau::ARegarder, Niveau::Suspect, Niveau::Dangereux];

/// Échelle des quatre verdicts : le niveau du fichier est plein et lumineux, les autres en contour.
fn echelle_verdict(ui: &mut Ui, actuel: Niveau) {
    let largeur = ui.available_width().min(620.0);
    let (zone, _) = ui.allocate_exact_size(Vec2::new(largeur, 38.0), Sense::hover());
    let peintre = ui.painter();
    let pas = (largeur - 18.0) / 4.0;
    for (i, n) in NIVEAUX.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(zone.left() + i as f32 * (pas + 6.0), zone.top()), Vec2::new(pas, 38.0));
        if *n == actuel {
            theme::halo(peintre, Pos2::new(r.center().x, r.bottom() - 2.0), Vec2::new(pas * 0.6, 22.0), couleur(*n), 0.5);
            peintre.rect_filled(r, 19.0, couleur(*n));
            let encre = if matches!(n, Niveau::ARegarder | Niveau::Suspect) { Color32::from_rgb(20, 16, 8) } else { Color32::WHITE };
            texte_centre(peintre, r.center(), n.libelle(), gras(14.5), encre);
        } else {
            peintre.rect(r, 19.0, Color32::TRANSPARENT, Stroke::new(1.0_f32, p().bord));
            texte_centre(peintre, r.center(), n.libelle(), FontId::new(14.0, FontFamily::Proportional), p().texte_3);
        }
    }
}

/// Petite flèche entre deux étapes du parcours.
fn fleche(peintre: &egui::Painter, zone: Rect) {
    let trait_ = Stroke::new(1.5_f32, p().texte_3);
    let (g, d, y) = (zone.left() + 2.0, zone.right() - 2.0, zone.center().y);
    peintre.line_segment([Pos2::new(g, y), Pos2::new(d, y)], trait_);
    peintre.line_segment([Pos2::new(d - 5.0, y - 4.0), Pos2::new(d, y)], trait_);
    peintre.line_segment([Pos2::new(d - 5.0, y + 4.0), Pos2::new(d, y)], trait_);
}

/// Schéma du parcours : chaque étape dit si elle a réellement eu lieu pour ce fichier.
fn parcours(ui: &mut Ui, rapport: &Rapport) {
    ui.spacing_mut().item_spacing = Vec2::new(2.0, 8.0);
    ui.horizontal_wrapped(|ui| {
        for (i, (nom, faite)) in rapport.etapes.iter().enumerate() {
            let g = ui.painter().layout_no_wrap(nom.clone(), FontId::new(13.5, FontFamily::Proportional), p().texte);
            // La flèche et son étape sont réservées d'un bloc : elles passent à la ligne ensemble.
            let avant = if i > 0 { 24.0 } else { 0.0 };
            let (bloc, _) = ui.allocate_exact_size(Vec2::new(avant + g.size().x + 34.0, 30.0), Sense::hover());
            if i > 0 {
                fleche(ui.painter(), Rect::from_min_size(bloc.min, Vec2::new(20.0, 30.0)));
            }
            let rect = Rect::from_min_max(Pos2::new(bloc.left() + avant, bloc.top()), bloc.max);
            let peintre = ui.painter();
            if *faite {
                peintre.rect(rect, 15.0, alpha(couleur(Niveau::Sain), 0.13), Stroke::new(1.0_f32, alpha(couleur(Niveau::Sain), 0.5)));
                peintre.circle_filled(Pos2::new(rect.left() + 14.0, rect.center().y), 4.0, couleur(Niveau::Sain));
                peintre.galley(Pos2::new(rect.left() + 24.0, rect.center().y - g.size().y / 2.0), g, p().texte);
            } else {
                peintre.rect(rect, 15.0, Color32::TRANSPARENT, Stroke::new(1.0_f32, p().bord));
                peintre.circle_stroke(Pos2::new(rect.left() + 14.0, rect.center().y), 3.6, Stroke::new(1.4_f32, p().texte_3));
                peintre.galley(Pos2::new(rect.left() + 24.0, rect.center().y - g.size().y / 2.0), g, p().texte_3);
            }
        }
    });
    ui.label(RichText::new("Disque plein : étape effectuée. Cercle vide : sans objet pour ce fichier.").small().color(p().texte_3));
}

/// Jauge horizontale en dégradé sur sa piste, avec un repère de seuil facultatif.
fn jauge(ui: &mut Ui, largeur: f32, part: f32, seuil: Option<f32>) {
    let (zone, _) = ui.allocate_exact_size(Vec2::new(largeur, 10.0), Sense::hover());
    let peintre = ui.painter();
    peintre.rect_filled(zone, 5.0, p().carte_2);
    let plein = Rect::from_min_size(zone.min, Vec2::new((zone.width() * part.clamp(0.0, 1.0)).max(4.0), zone.height()));
    degrade_arrondi(peintre, plein, 5.0, ACCENT, ACCENT_2);
    if let Some(s) = seuil {
        let x = zone.left() + zone.width() * s.clamp(0.0, 1.0);
        peintre.line_segment([Pos2::new(x, zone.top() - 4.0), Pos2::new(x, zone.bottom() + 4.0)], Stroke::new(2.0_f32, p().texte));
    }
}

fn droits(s: &SectionVue) -> &'static str {
    match (s.executable, s.inscriptible) {
        (true, true) => "exécutable + modifiable",
        (true, false) => "exécutable",
        (false, true) => "modifiable",
        (false, false) => "lecture seule",
    }
}

fn teinte_section(s: &SectionVue) -> Color32 {
    match (s.executable, s.inscriptible) {
        (true, true) => couleur(Niveau::Dangereux),
        (true, false) => ACCENT,
        _ => p().texte_3,
    }
}

/// Carte des sections : taille relative, droits et entropie de chaque morceau du programme.
fn carte_sections(ui: &mut Ui, sections: &[SectionVue]) {
    let max = sections.iter().map(|s| s.taille).max().unwrap_or(1).max(1) as f32;
    egui::Grid::new("sections").spacing([16.0, 9.0]).show(ui, |ui| {
        for titre in ["Section", "Taille", "", "Droits", "Entropie", ""] {
            ui.label(RichText::new(titre).small().color(p().texte_3));
        }
        ui.end_row();
        for s in sections {
            ui.label(RichText::new(&s.nom).monospace());
            let (zone, _) = ui.allocate_exact_size(Vec2::new(210.0, 10.0), Sense::hover());
            ui.painter().rect_filled(zone, 5.0, p().carte_2);
            let barre = Rect::from_min_size(zone.min, Vec2::new((210.0 * s.taille as f32 / max).max(4.0), 10.0));
            ui.painter().rect_filled(barre, 5.0, teinte_section(s));
            ui.label(RichText::new(format!("{} o", s.taille)).color(p().texte_2));
            ui.label(droits(s));
            jauge(ui, 84.0, (s.entropie / 8.0) as f32, None);
            ui.label(RichText::new(format!("{:.2}", s.entropie)).color(p().texte_2));
            ui.end_row();
        }
    });
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        theme::point(ui, ACCENT);
        ui.label(RichText::new("code exécutable").small().color(p().texte_2));
        theme::point(ui, p().texte_3);
        ui.label(RichText::new("données").small().color(p().texte_2));
        theme::point(ui, couleur(Niveau::Dangereux));
        ui.label(RichText::new("exécutable et modifiable à la fois (anormal)").small().color(p().texte_2));
    });
}

impl App {
    pub(super) fn vue_experte_analyse(&mut self, ui: &mut Ui, t: f64) {
        theme::entete(ui, "Analyse détaillée", "Glisse un fichier dans la fenêtre, ou choisis-le. Il est lu et désassemblé, jamais exécuté.");
        if theme::bouton_principal(ui, "Ouvrir un fichier…", 0.0).clicked() {
            self.choisir_fichier(t);
        }
        ui.add_space(6.0);

        let seuil = self.cfg.analyse.entropie_suspecte;
        let Some(rapport) = &self.rapport else {
            theme::carte(ui, |ui| {
                ui.label(RichText::new("Aucun fichier analysé pour l'instant.").font(gras(15.0)));
                ui.label(RichText::new("Dépose un .exe, un .dll ou un installeur pour l'inspecter avant de le lancer.").color(p().texte_2));
            });
            return;
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            theme::carte(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::titre_section(ui, "Verdict");
                    theme::pastille(ui, rapport.niveau);
                });
                echelle_verdict(ui, rapport.niveau);
                ui.add_space(2.0);
                ui.label(RichText::new(&rapport.chemin).monospace());
                ui.label(RichText::new(format!("{} octets   ·   SHA-256 {}", rapport.taille, rapport.sha256)).monospace().small().color(p().texte_3));
            });

            theme::carte(ui, |ui| {
                theme::titre_section(ui, "Parcours de l'analyse");
                parcours(ui, rapport);
            });

            theme::carte(ui, |ui| {
                theme::titre_section(ui, "Entropie du fichier");
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.add_space(6.0);
                        jauge(ui, 340.0, (rapport.entropie / 8.0) as f32, Some((seuil / 8.0) as f32));
                    });
                    ui.label(RichText::new(format!("{:.2} / 8", rapport.entropie)).font(gras(15.0)));
                });
                ui.label(RichText::new(format!("Le trait marque le seuil ({seuil:.1}) au-delà duquel un programme est probablement compressé ou chiffré.")).small().color(p().texte_3));
            });

            if rapport.est_pe && !rapport.resume_pe.is_empty() {
                theme::carte(ui, |ui| {
                    theme::titre_section(ui, "Carte d'identité du binaire");
                    // Largeur de colonne bornée : une valeur longue (liste de DLL) passe à la ligne au lieu de déborder.
                    let largeur_valeur = (ui.available_width() - 230.0).max(160.0);
                    egui::Grid::new("resume_pe").spacing([22.0, 7.0]).max_col_width(largeur_valeur).show(ui, |ui| {
                        for (k, v) in &rapport.resume_pe {
                            ui.label(RichText::new(k).color(p().texte_2));
                            ui.add(egui::Label::new(v).wrap());
                            ui.end_row();
                        }
                    });
                });
            }

            if !rapport.sections.is_empty() {
                theme::carte(ui, |ui| {
                    theme::titre_section(ui, "Carte des sections");
                    carte_sections(ui, &rapport.sections);
                });
            }

            ui.add_space(4.0);
            theme::titre_section(ui, &format!("Constats ({})", rapport.constats.len()));
            for c in &rapport.constats {
                carte_constat(ui, c);
            }

            if !rapport.apercu_asm.is_empty() {
                theme::carte(ui, |ui| {
                    ui.collapsing(RichText::new(format!("Désassemblage du point d'entrée ({} instructions)", rapport.apercu_asm.len())).font(gras(15.0)), |ui| {
                        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                            for ligne in &rapport.apercu_asm {
                                ui.label(RichText::new(ligne).monospace().small().color(p().texte_2));
                            }
                        });
                    });
                });
            }
            ui.add_space(8.0);
        });
    }
}
