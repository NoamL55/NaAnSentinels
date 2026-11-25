// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Habillage de l'interface : palette, polices, cartes, boutons, dégradés et halos.
//! Tout ce qui donne son allure à l'application est ici, pour pouvoir la restyler d'un seul endroit.

use std::sync::OnceLock;

use eframe::egui::{
    self, epaint::Shadow, Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin, Mesh, Painter, Pos2, Rect,
    Response, Rounding, Sense, Shape, Stroke, TextStyle, Ui, Vec2,
};

use crate::engine::verdict::Niveau;

pub const ACCENT: Color32 = Color32::from_rgb(59, 130, 246);
pub const ACCENT_2: Color32 = Color32::from_rgb(34, 211, 238);

pub struct Palette {
    pub sombre: bool,
    pub fond: Color32,
    pub fond_2: Color32,
    pub carte: Color32,
    pub carte_2: Color32,
    pub bord: Color32,
    pub texte: Color32,
    pub texte_2: Color32,
    pub texte_3: Color32,
}

static NUIT: Palette = Palette {
    sombre: true,
    fond: Color32::from_rgb(9, 13, 22),
    fond_2: Color32::from_rgb(13, 18, 30),
    carte: Color32::from_rgb(18, 25, 40),
    carte_2: Color32::from_rgb(26, 35, 55),
    bord: Color32::from_rgb(36, 47, 72),
    texte: Color32::from_rgb(236, 241, 250),
    texte_2: Color32::from_rgb(150, 163, 190),
    texte_3: Color32::from_rgb(98, 111, 138),
};

static JOUR: Palette = Palette {
    sombre: false,
    fond: Color32::from_rgb(244, 246, 251),
    fond_2: Color32::from_rgb(234, 238, 247),
    carte: Color32::from_rgb(255, 255, 255),
    carte_2: Color32::from_rgb(240, 243, 250),
    bord: Color32::from_rgb(214, 221, 235),
    texte: Color32::from_rgb(15, 22, 36),
    texte_2: Color32::from_rgb(82, 95, 120),
    texte_3: Color32::from_rgb(132, 143, 165),
};

static SOMBRE: OnceLock<bool> = OnceLock::new();

/// La palette en cours (nuit par défaut, jour si `theme = "clair"` dans config.toml).
pub fn p() -> &'static Palette {
    if *SOMBRE.get().unwrap_or(&true) {
        &NUIT
    } else {
        &JOUR
    }
}

/// Couleurs des verdicts : toujours accompagnées d'un symbole dessiné et d'un libellé, jamais seules.
pub fn couleur(n: Niveau) -> Color32 {
    match n {
        Niveau::Sain => Color32::from_rgb(34, 197, 94),
        Niveau::ARegarder => Color32::from_rgb(245, 178, 40),
        Niveau::Suspect => Color32::from_rgb(244, 124, 72),
        Niveau::Dangereux => Color32::from_rgb(232, 68, 68),
    }
}

/// Police grasse (titres, boutons).
pub fn gras(taille: f32) -> FontId {
    FontId::new(taille, FontFamily::Name("gras".into()))
}

pub fn melange(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()), m(a.a(), b.a()))
}

pub fn alpha(c: Color32, a: f32) -> Color32 {
    c.linear_multiply(a.clamp(0.0, 1.0))
}

/// Installe polices, couleurs et espacements. À appeler une fois au démarrage.
pub fn installer(ctx: &egui::Context, sombre: bool) {
    let _ = SOMBRE.set(sombre);
    let pal = p();

    // Polices du système quand elles existent (Segoe UI sous Windows), sinon celles d'egui.
    let mut polices = FontDefinitions::default();
    let dossier = std::env::var("WINDIR").map(|w| format!("{w}\\Fonts")).unwrap_or_default();
    let charger = |nom: &str| if dossier.is_empty() { None } else { std::fs::read(format!("{dossier}\\{nom}")).ok() };
    if let Some(o) = charger("segoeui.ttf") {
        polices.font_data.insert("ui".into(), FontData::from_owned(o));
        polices.families.entry(FontFamily::Proportional).or_default().insert(0, "ui".into());
    }
    let mut famille_grasse = polices.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(o) = charger("seguisb.ttf") {
        polices.font_data.insert("ui-gras".into(), FontData::from_owned(o));
        famille_grasse.insert(0, "ui-gras".into());
    }
    polices.families.insert(FontFamily::Name("gras".into()), famille_grasse);
    ctx.set_fonts(polices);

    let mut v = if sombre { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = pal.fond;
    v.window_fill = pal.carte;
    v.extreme_bg_color = pal.fond_2;
    v.faint_bg_color = pal.carte_2;
    v.override_text_color = Some(pal.texte);
    v.hyperlink_color = ACCENT_2;
    v.selection.bg_fill = alpha(ACCENT, 0.45);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    let arrondi = Rounding::same(10.0);
    v.widgets.noninteractive.rounding = arrondi;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, pal.bord);
    v.widgets.inactive.rounding = arrondi;
    v.widgets.inactive.bg_fill = pal.carte_2;
    v.widgets.inactive.weak_bg_fill = pal.carte_2;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, pal.bord);
    v.widgets.hovered.rounding = arrondi;
    v.widgets.hovered.bg_fill = melange(pal.carte_2, ACCENT, 0.18);
    v.widgets.hovered.weak_bg_fill = melange(pal.carte_2, ACCENT, 0.18);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, alpha(ACCENT, 0.7));
    v.widgets.active.rounding = arrondi;
    v.widgets.active.bg_fill = melange(pal.carte_2, ACCENT, 0.32);
    v.widgets.active.weak_bg_fill = melange(pal.carte_2, ACCENT, 0.32);
    v.widgets.open.rounding = arrondi;
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.text_styles.insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Button, FontId::new(15.0, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));
        s.text_styles.insert(TextStyle::Heading, gras(24.0));
        s.spacing.item_spacing = Vec2::new(10.0, 9.0);
        s.spacing.button_padding = Vec2::new(14.0, 8.0);
    });
}

// ---------------------------------------------------------------- Formes

/// Halo lumineux : disque dont la couleur s'éteint vers le bord.
pub fn halo(peintre: &Painter, centre: Pos2, rayon: Vec2, teinte: Color32, force: f32) {
    let mut m = Mesh::default();
    m.colored_vertex(centre, alpha(teinte, force));
    const N: u32 = 48;
    for i in 0..=N {
        let a = i as f32 / N as f32 * std::f32::consts::TAU;
        m.colored_vertex(Pos2::new(centre.x + a.cos() * rayon.x, centre.y + a.sin() * rayon.y), Color32::TRANSPARENT);
    }
    for i in 1..=N {
        m.add_triangle(0, i, i + 1);
    }
    peintre.add(Shape::mesh(m));
}

/// Rectangle arrondi rempli d'un dégradé horizontal.
pub fn degrade_arrondi(peintre: &Painter, r: Rect, rayon: f32, gauche: Color32, droite: Color32) {
    let rayon = rayon.min(r.height() / 2.0).min(r.width() / 2.0);
    let coins = [
        (Pos2::new(r.right() - rayon, r.top() + rayon), -90.0_f32),
        (Pos2::new(r.right() - rayon, r.bottom() - rayon), 0.0),
        (Pos2::new(r.left() + rayon, r.bottom() - rayon), 90.0),
        (Pos2::new(r.left() + rayon, r.top() + rayon), 180.0),
    ];
    let teinte = |x: f32| melange(gauche, droite, (x - r.left()) / r.width().max(1.0));
    let mut m = Mesh::default();
    m.colored_vertex(r.center(), teinte(r.center().x));
    let mut n = 0u32;
    for (c, depart) in coins {
        for k in 0..=8 {
            let a = (depart + k as f32 * 90.0 / 8.0).to_radians();
            let pt = Pos2::new(c.x + a.cos() * rayon, c.y + a.sin() * rayon);
            m.colored_vertex(pt, teinte(pt.x));
            n += 1;
        }
    }
    for i in 1..=n {
        m.add_triangle(0, i, if i == n { 1 } else { i + 1 });
    }
    peintre.add(Shape::mesh(m));
}

/// Fond de page : deux lueurs douces derrière le contenu.
pub fn fond(ui: &Ui) {
    let r = ui.max_rect();
    let peintre = ui.painter();
    let force = if p().sombre { 0.16 } else { 0.10 };
    halo(peintre, Pos2::new(r.center().x, r.top() + 120.0), Vec2::new(r.width() * 0.62, 460.0), ACCENT, force);
    halo(peintre, Pos2::new(r.right() - 40.0, r.bottom() + 40.0), Vec2::new(520.0, 300.0), ACCENT_2, force * 0.28);
}

// ---------------------------------------------------------------- Composants

/// Carte : surface arrondie, bordée, avec une ombre douce.
pub fn carte<R>(ui: &mut Ui, contenu: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    Frame::none()
        .fill(p().carte)
        .stroke(Stroke::new(1.0_f32, p().bord))
        .rounding(14.0)
        .inner_margin(Margin::same(16.0))
        .shadow(Shadow { offset: Vec2::new(0.0, 6.0), blur: 18.0, spread: 0.0, color: Color32::from_black_alpha(if p().sombre { 80 } else { 22 }) })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            contenu(ui)
        })
}

/// Carte avec une barre de couleur à gauche (pour un constat).
pub fn carte_marquee<R>(ui: &mut Ui, teinte: Color32, contenu: impl FnOnce(&mut Ui) -> R) -> R {
    let rep = carte(ui, contenu);
    let r = rep.response.rect;
    let barre = Rect::from_min_max(Pos2::new(r.left() + 1.0, r.top() + 12.0), Pos2::new(r.left() + 5.0, r.bottom() - 12.0));
    ui.painter().rect_filled(barre, 2.0, teinte);
    rep.inner
}

/// Bouton principal : pilule en dégradé, qui s'illumine au survol.
pub fn bouton_principal(ui: &mut Ui, texte: &str, largeur_min: f32) -> Response {
    let galley = ui.painter().layout_no_wrap(texte.to_owned(), gras(16.0), Color32::WHITE);
    let taille = Vec2::new((galley.size().x + 52.0).max(largeur_min), 46.0);
    let (rect, rep) = ui.allocate_exact_size(taille, Sense::click());
    let survol = ui.ctx().animate_bool(rep.id, rep.hovered());
    let peintre = ui.painter();
    halo(peintre, Pos2::new(rect.center().x, rect.bottom() - 4.0), Vec2::new(rect.width() * 0.62, 30.0), ACCENT, 0.30 + 0.35 * survol);
    let r = rect.expand(survol * 1.5);
    degrade_arrondi(peintre, r, r.height() / 2.0, melange(ACCENT, Color32::WHITE, 0.10 * survol), melange(ACCENT_2, Color32::WHITE, 0.10 * survol));
    peintre.galley(r.center() - galley.size() / 2.0, galley, Color32::WHITE);
    rep.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Bouton secondaire : contour discret.
pub fn bouton_secondaire(ui: &mut Ui, texte: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(texte.to_owned(), FontId::new(15.0, FontFamily::Proportional), p().texte);
    let taille = Vec2::new(galley.size().x + 34.0, 38.0);
    let (rect, rep) = ui.allocate_exact_size(taille, Sense::click());
    let survol = ui.ctx().animate_bool(rep.id, rep.hovered());
    let peintre = ui.painter();
    peintre.rect(rect, rect.height() / 2.0, melange(p().carte, ACCENT, 0.16 * survol), Stroke::new(1.0_f32, melange(p().bord, ACCENT, survol)));
    peintre.galley(rect.center() - galley.size() / 2.0, galley, p().texte);
    rep.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Pastille de verdict : point de couleur + libellé, dans une pilule teintée.
pub fn pastille(ui: &mut Ui, n: Niveau) {
    let c = couleur(n);
    let galley = ui.painter().layout_no_wrap(n.libelle().to_owned(), gras(13.5), p().texte);
    let taille = Vec2::new(galley.size().x + 36.0, 26.0);
    let (rect, _) = ui.allocate_exact_size(taille, Sense::hover());
    let peintre = ui.painter();
    peintre.rect(rect, 13.0, alpha(c, 0.16), Stroke::new(1.0_f32, alpha(c, 0.55)));
    peintre.circle_filled(Pos2::new(rect.left() + 14.0, rect.center().y), 4.5, c);
    peintre.galley(Pos2::new(rect.left() + 25.0, rect.center().y - galley.size().y / 2.0), galley, p().texte);
}

/// Petit disque de couleur (repère d'état).
pub fn point(ui: &mut Ui, c: Color32) {
    let (zone, _) = ui.allocate_exact_size(Vec2::new(12.0, 14.0), Sense::hover());
    ui.painter().circle_filled(zone.center(), 4.5, c);
}

/// Titre de page et sa phrase d'explication.
pub fn entete(ui: &mut Ui, titre: &str, sous_titre: &str) {
    ui.label(egui::RichText::new(titre).font(gras(26.0)).color(p().texte));
    if !sous_titre.is_empty() {
        ui.label(egui::RichText::new(sous_titre).color(p().texte_2));
    }
    ui.add_space(8.0);
}

/// Titre de section à l'intérieur d'une carte.
pub fn titre_section(ui: &mut Ui, texte: &str) {
    ui.label(egui::RichText::new(texte).font(gras(15.5)).color(p().texte));
}

/// Barre de progression fine, en dégradé.
pub fn progression(ui: &mut Ui, largeur: f32, part: f32) {
    let (zone, _) = ui.allocate_exact_size(Vec2::new(largeur, 6.0), Sense::hover());
    let peintre = ui.painter();
    peintre.rect_filled(zone, 3.0, p().carte_2);
    let plein = Rect::from_min_size(zone.min, Vec2::new((zone.width() * part.clamp(0.0, 1.0)).max(6.0), zone.height()));
    degrade_arrondi(peintre, plein, 3.0, ACCENT, ACCENT_2);
}
