// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Le bouclier en volume : un vrai modèle 3D (face bombée, chanfrein, tranche), éclairé et projeté en perspective.
//! Il tourne doucement, s'incline vers la souris, porte un halo, un anneau en orbite et, pendant l'analyse,
//! un faisceau lumineux qui le balaie. Tout est calculé ici, sans carte graphique dédiée ni fichier de modèle.

use eframe::egui::{Color32, Mesh, Painter, Pos2, Sense, Shape, Stroke, Ui, Vec2};

use super::theme::{alpha, halo, melange, p};

#[derive(Clone, Copy, PartialEq)]
pub enum Embleme {
    Aucun,
    Coche,
    Exclamation,
    Triangle,
    Croix,
}

type V3 = [f32; 3];

fn norme(v: V3) -> V3 {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}
fn scalaire(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Rotation autour de l'axe vertical (lacet) puis de l'axe horizontal (tangage).
fn tourner(v: V3, lacet: f32, tangage: f32) -> V3 {
    let (sl, cl) = lacet.sin_cos();
    let (x, z) = (v[0] * cl + v[2] * sl, -v[0] * sl + v[2] * cl);
    let (st, ct) = tangage.sin_cos();
    [x, v[1] * ct - z * st, v[1] * st + z * ct]
}

/// Contour du bouclier, dans le sens des aiguilles d'une montre (y vers le bas), dans un repère [-1, 1].
pub(super) fn contour() -> Vec<[f32; 2]> {
    let mut droite: Vec<[f32; 2]> = Vec::new();
    // Côté droit, sous l'épaule.
    for i in 1..=5 {
        let u = i as f32 / 5.0;
        droite.push([0.80 - 0.02 * u, -0.72 + 0.77 * u]);
    }
    // Courbe jusqu'à la pointe (Bézier quadratique).
    let (p0, c, p2) = ([0.78_f32, 0.05_f32], [0.74_f32, 0.70_f32], [0.0_f32, 1.0_f32]);
    for i in 1..=14 {
        let u = i as f32 / 14.0;
        let a = (1.0 - u) * (1.0 - u);
        let b = 2.0 * u * (1.0 - u);
        droite.push([a * p0[0] + b * c[0] + u * u * p2[0], a * p0[1] + b * c[1] + u * u * p2[1]]);
    }
    let mut pts: Vec<[f32; 2]> = Vec::new();
    // Bord supérieur légèrement bombé, de gauche à droite.
    for i in 0..=12 {
        let x = -0.80 + 1.60 * (i as f32 / 12.0);
        pts.push([x, -0.72 - 0.14 * (1.0 - (x / 0.80).powi(2))]);
    }
    pts.extend(droite.iter().copied());
    // Côté gauche : le miroir du droit, en remontant (la pointe n'est pas répétée).
    for q in droite.iter().rev().skip(1) {
        pts.push([-q[0], q[1]]);
    }
    pts
}

struct Sommet {
    p: V3,
    n: V3,
    /// Luminosité propre de la zone (face, chanfrein, tranche).
    k: f32,
}

/// Dessine le bouclier dans une zone carrée de côté `taille`.
/// `analyse` : un faisceau balaie le bouclier et l'anneau accélère. `apparition` : 0 → 1, petit rebond d'entrée.
pub fn dessiner(ui: &mut Ui, taille: f32, teinte: Color32, embleme: Embleme, t: f64, analyse: bool, apparition: f32) {
    let (rep, peintre) = ui.allocate_painter(Vec2::splat(taille), Sense::hover());
    let centre = rep.rect.center();
    let t = t as f32;

    // Inclinaison : une oscillation lente, plus un penchant vers la souris.
    let souris = ui.input(|i| i.pointer.hover_pos());
    let (mut dx, mut dy) = (0.0, 0.0);
    if let Some(s) = souris {
        dx = ((s.x - centre.x) / (taille * 1.6)).clamp(-1.0, 1.0);
        dy = ((s.y - centre.y) / (taille * 1.6)).clamp(-1.0, 1.0);
    }
    let lacet = 0.34 * (t * 0.8).sin() + 0.45 * dx;
    let tangage = 0.10 * (t * 0.6 + 1.0).sin() - 0.32 * dy;

    let rebond = 1.0 + 0.22 * (1.0 - apparition) * (1.0 - apparition);
    let echelle = taille * 0.27 * rebond;
    let projeter = |v: V3| -> Pos2 {
        let persp = 3.6 / (3.6 - v[2]);
        Pos2::new(centre.x + v[0] * persp * echelle, centre.y + v[1] * persp * echelle)
    };

    // ---------- Décor derrière le bouclier ----------
    halo(&peintre, centre, Vec2::splat(taille * 0.52), teinte, if p().sombre { 0.34 } else { 0.22 });
    halo(&peintre, Pos2::new(centre.x, centre.y + echelle * 1.32), Vec2::new(echelle * 0.85, echelle * 0.13), Color32::BLACK, if p().sombre { 0.55 } else { 0.22 });

    // Poussières lumineuses qui montent doucement.
    for i in 0..16 {
        let f = i as f32;
        let hasard = |k: f32| ((f * 12.9898 + k * 78.233).sin() * 43758.547).fract().abs();
        let cycle = (t * (0.05 + 0.05 * hasard(1.0)) + hasard(2.0)).fract();
        let x = centre.x + (hasard(3.0) - 0.5) * taille * 0.9;
        let y = centre.y + taille * 0.45 - cycle * taille * 0.9;
        let eclat = (cycle * std::f32::consts::PI).sin() * (0.35 + 0.65 * (t * 2.0 + f).sin().abs());
        peintre.circle_filled(Pos2::new(x, y), 1.0 + hasard(4.0) * 1.4, alpha(melange(teinte, Color32::WHITE, 0.5), 0.55 * eclat));
    }

    // ---------- Anneau en orbite (calculé une fois, dessiné en deux moitiés) ----------
    const SEG: usize = 96;
    let anneau: Vec<V3> = (0..SEG)
        .map(|i| {
            let a = i as f32 / SEG as f32 * std::f32::consts::TAU;
            let v = [a.cos() * 1.46, 0.0, a.sin() * 1.46];
            // Bascule de l'orbite, puis léger entraînement par la rotation du bouclier.
            let (s, c) = 1.22_f32.sin_cos();
            let v = [v[0], v[1] * c - v[2] * s, v[1] * s + v[2] * c];
            let (s, c) = (-0.30_f32).sin_cos();
            let v = [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]];
            tourner(v, lacet * 0.5, tangage * 0.5)
        })
        .collect();
    let satellite = ((t * if analyse { 4.2 } else { 0.9 }) / std::f32::consts::TAU).fract() * SEG as f32;
    let dessiner_anneau = |devant: bool| {
        for i in 0..SEG {
            let (a, b) = (anneau[i], anneau[(i + 1) % SEG]);
            if (a[2] + b[2] >= 0.0) != devant {
                continue;
            }
            // Traînée : les segments juste derrière le satellite sont plus vifs.
            let retard = (satellite - i as f32).rem_euclid(SEG as f32) / SEG as f32;
            let trainee = (1.0 - retard * 3.2).max(0.0);
            let profondeur = 0.45 + 0.55 * ((a[2] + 1.5) / 3.0).clamp(0.0, 1.0);
            let force = (0.16 + 0.84 * trainee) * profondeur;
            peintre.line_segment([projeter(a), projeter(b)], Stroke::new(1.2_f32 + 2.2 * trainee, alpha(melange(teinte, Color32::WHITE, 0.35 * trainee), force)));
        }
        let s = anneau[(satellite as usize) % SEG];
        if (s[2] >= 0.0) == devant {
            let pos = projeter(s);
            halo(&peintre, pos, Vec2::splat(13.0), melange(teinte, Color32::WHITE, 0.4), 0.9);
            peintre.circle_filled(pos, 3.2, Color32::WHITE);
        }
    };
    dessiner_anneau(false);

    // ---------- Le modèle : face bombée, chanfrein, tranche ----------
    let bord = contour();
    let n = bord.len();
    let milieu = [0.0_f32, 0.04_f32];
    // Normale 2D sortante en chaque point du contour.
    let dehors: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let (a, b) = (bord[(i + n - 1) % n], bord[(i + 1) % n]);
            let (tx, ty) = (b[0] - a[0], b[1] - a[1]);
            let l = (tx * tx + ty * ty).sqrt().max(1e-6);
            [ty / l, -tx / l]
        })
        .collect();
    let anneau_de = |ech: f32, z: f32, pente: f32, k: f32| -> Vec<Sommet> {
        (0..n)
            .map(|i| Sommet {
                p: [milieu[0] + (bord[i][0] - milieu[0]) * ech, milieu[1] + (bord[i][1] - milieu[1]) * ech, z],
                n: norme([dehors[i][0] * pente, dehors[i][1] * pente, if pente >= 99.0 { 0.0 } else { 1.0 }]),
                k,
            })
            .collect()
    };
    let coeur = Sommet { p: [milieu[0], milieu[1], 0.11], n: [0.0, 0.0, 1.0], k: 1.0 };
    let interieur = anneau_de(0.52, 0.075, 0.16, 1.0);
    let face = anneau_de(0.90, 0.0, 0.36, 1.0);
    let chanfrein_haut = anneau_de(0.90, 0.0, 1.05, 1.22);
    let chanfrein_bas = anneau_de(1.0, -0.10, 1.05, 1.22);
    let tranche_haut = anneau_de(1.0, -0.10, 99.0, 0.62);
    let tranche_bas = anneau_de(1.0, -0.30, 99.0, 0.50);

    // Éclairage : une lumière en haut à gauche, devant ; un liseré clair sur les bords fuyants.
    let lumiere = norme([-0.45, -0.62, 0.64]);
    let mi_chemin = norme([lumiere[0], lumiere[1], lumiere[2] + 1.0]);
    let faisceau = if analyse { -1.15 + 2.3 * (t * 0.85).fract() } else { -9.0 };
    let eclairer = |s: &Sommet| -> (V3, Color32) {
        let pos = tourner(s.p, lacet, tangage);
        let nor = tourner(s.n, lacet, tangage);
        let diffus = scalaire(nor, lumiere).max(0.0);
        let brillant = scalaire(nor, mi_chemin).max(0.0).powf(38.0);
        let lisere = (1.0 - nor[2].max(0.0)).powi(3);
        let degrade = 1.0 - 0.20 * ((s.p[1] + 1.0) / 2.0);
        let force = (0.30 + 0.82 * diffus) * degrade * s.k;
        let mut c = [teinte.r() as f32 * force, teinte.g() as f32 * force, teinte.b() as f32 * force];
        let balayage = (-((s.p[1] - faisceau) / 0.11).powi(2)).exp();
        let blanc = 255.0 * (0.60 * brillant + 0.16 * lisere + 0.75 * balayage);
        for canal in c.iter_mut() {
            *canal = (*canal + blanc).min(255.0);
        }
        (pos, Color32::from_rgb(c[0] as u8, c[1] as u8, c[2] as u8))
    };

    // Triangles, triés du plus lointain au plus proche (algorithme du peintre).
    let mut triangles: Vec<(f32, [(Pos2, Color32); 3])> = Vec::with_capacity(n * 7);
    let mut ajouter = |a: &Sommet, b: &Sommet, c: &Sommet| {
        let (pa, ca) = eclairer(a);
        let (pb, cb) = eclairer(b);
        let (pc, cc) = eclairer(c);
        triangles.push(((pa[2] + pb[2] + pc[2]) / 3.0, [(projeter(pa), ca), (projeter(pb), cb), (projeter(pc), cc)]));
    };
    for i in 0..n {
        let j = (i + 1) % n;
        ajouter(&coeur, &interieur[i], &interieur[j]);
        ajouter(&interieur[i], &face[i], &face[j]);
        ajouter(&interieur[i], &face[j], &interieur[j]);
        ajouter(&chanfrein_haut[i], &chanfrein_bas[i], &chanfrein_bas[j]);
        ajouter(&chanfrein_haut[i], &chanfrein_bas[j], &chanfrein_haut[j]);
        ajouter(&tranche_haut[i], &tranche_bas[i], &tranche_bas[j]);
        ajouter(&tranche_haut[i], &tranche_bas[j], &tranche_haut[j]);
    }
    triangles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut maillage = Mesh::default();
    for (_, sommets) in &triangles {
        let base = maillage.vertices.len() as u32;
        for (pos, c) in sommets {
            maillage.colored_vertex(*pos, *c);
        }
        maillage.add_triangle(base, base + 1, base + 2);
    }
    peintre.add(Shape::mesh(maillage));

    // ---------- Emblème en relief sur la face ----------
    let sur_face = |x: f32, y: f32| projeter(tourner([x, y + 0.04, 0.125], lacet, tangage));
    let epais = (echelle * 0.105).max(3.0);
    let trace = |peintre: &Painter, points: &[(f32, f32)], ferme: bool| {
        let pts: Vec<Pos2> = points.iter().map(|(x, y)| sur_face(*x, *y)).collect();
        for passe in 0..2 {
            // Une ombre portée, puis le trait blanc : l'emblème semble posé sur le métal.
            let (decal, c, e) = if passe == 0 { (Vec2::new(1.5, 2.5), Color32::from_black_alpha(70), epais + 1.0) } else { (Vec2::ZERO, Color32::WHITE, epais) };
            let m = if ferme { pts.len() } else { pts.len() - 1 };
            for i in 0..m {
                peintre.line_segment([pts[i] + decal, pts[(i + 1) % pts.len()] + decal], Stroke::new(e, c));
            }
            for q in &pts {
                peintre.circle_filled(*q + decal, e / 2.0, c);
            }
        }
    };
    match embleme {
        Embleme::Coche => trace(&peintre, &[(-0.36, 0.00), (-0.09, 0.27), (0.38, -0.26)], false),
        Embleme::Croix => {
            trace(&peintre, &[(-0.28, -0.30), (0.28, 0.26)], false);
            trace(&peintre, &[(0.28, -0.30), (-0.28, 0.26)], false);
        }
        Embleme::Triangle => trace(&peintre, &[(0.0, -0.36), (0.36, 0.26), (-0.36, 0.26)], true),
        Embleme::Exclamation => {
            trace(&peintre, &[(0.0, -0.38), (0.0, 0.08)], false);
            trace(&peintre, &[(0.0, 0.34), (0.0, 0.345)], false);
        }
        Embleme::Aucun => {}
    }

    dessiner_anneau(true);
}

/// Version miniature et fixe, pour le logo de la barre latérale.
pub fn logo(peintre: &Painter, centre: Pos2, rayon: f32, teinte: Color32) {
    let pts: Vec<Pos2> = contour().iter().map(|q| Pos2::new(centre.x + q[0] * rayon, centre.y + q[1] * rayon)).collect();
    let mut m = Mesh::default();
    m.colored_vertex(centre, melange(teinte, Color32::WHITE, 0.25));
    for q in pts.iter() {
        // Dégradé vertical : plus clair en haut.
        let h = (q.y - (centre.y - rayon)) / (2.0 * rayon);
        m.colored_vertex(*q, melange(melange(teinte, Color32::WHITE, 0.18), melange(teinte, Color32::BLACK, 0.25), h));
    }
    let n = pts.len() as u32;
    for i in 1..=n {
        m.add_triangle(0, i, if i == n { 1 } else { i + 1 });
    }
    peintre.add(Shape::mesh(m));
    let trait_ = Stroke::new((rayon * 0.16).max(1.6), Color32::WHITE);
    let q = |x: f32, y: f32| Pos2::new(centre.x + x * rayon, centre.y + y * rayon);
    peintre.line_segment([q(-0.34, 0.04), q(-0.08, 0.30)], trait_);
    peintre.line_segment([q(-0.08, 0.30), q(0.38, -0.22)], trait_);
}

