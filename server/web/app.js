// SPDX-License-Identifier: MIT — Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
// Tableau de bord. Tout le contenu venant du serveur est inséré avec textContent (jamais innerHTML).
'use strict';

const $ = (sel, racine = document) => racine.querySelector(sel);
const el = (balise, classe, texte) => {
  const e = document.createElement(balise);
  if (classe) e.className = classe;
  if (texte !== undefined) e.textContent = texte;
  return e;
};
const nombre = (n) => new Intl.NumberFormat('fr-FR').format(n);
const pluriel = (n, un, plusieurs) => `${nombre(n)} ${n > 1 ? plusieurs : un}`;
const mouvementReduit = matchMedia('(prefers-reduced-motion: reduce)').matches;

const VERDICTS = [
  { cle: 'sain', nom: 'Sain', symbole: '✓' },
  { cle: 'a_regarder', nom: 'À regarder', symbole: '!' },
  { cle: 'suspect', nom: 'Suspect', symbole: '▲' },
  { cle: 'dangereux', nom: 'Dangereux', symbole: '✕' },
];
const VUES = {
  mes: { titre: 'Mes appareils', droit: 'appareils:les-miens' },
  tableau: { titre: 'Tableau de bord', droit: 'tableau:lire' },
  admin: { titre: 'Comptes et sécurité', droit: 'utilisateurs:gerer' },
};

let moi = null;
let vueCourante = 'accueil';
let minuterie = null;
let modeDetail = false;
try { modeDetail = localStorage.getItem('nas-mode') === 'detail'; } catch { /* stockage indisponible */ }

async function api(chemin, corps) {
  const options = { headers: { 'X-NaAnSentinels': '1' } };
  if (corps !== undefined) {
    options.method = 'POST';
    options.headers['Content-Type'] = 'application/json';
    options.body = JSON.stringify(corps);
  }
  const rep = await fetch(chemin, options);
  let donnees = null;
  try { donnees = await rep.json(); } catch { /* réponse sans corps */ }
  if (!rep.ok) {
    const err = new Error((donnees && donnees.erreur) || `Erreur ${rep.status}`);
    err.statut = rep.status;
    throw err;
  }
  return donnees;
}

function ilYA(secondes) {
  if (secondes < 60) return 'à l’instant';
  if (secondes < 3600) return `il y a ${Math.round(secondes / 60)} min`;
  if (secondes < 86400) return `il y a ${Math.round(secondes / 3600)} h`;
  return `il y a ${Math.round(secondes / 86400)} j`;
}
const dateHeure = (t) => new Date(t * 1000).toLocaleString('fr-FR', { dateStyle: 'short', timeStyle: 'short' });

/** Fait défiler un nombre de son ancienne valeur vers la nouvelle (sauf si l'utilisateur limite les animations). */
function compter(noeud, cible) {
  const depart = Number(noeud.dataset.valeur || 0);
  noeud.dataset.valeur = String(cible);
  if (mouvementReduit || depart === cible) { noeud.textContent = nombre(cible); return; }
  const debut = performance.now();
  const pas = (t) => {
    const p = Math.min(1, (t - debut) / 700);
    noeud.textContent = nombre(Math.round(depart + (cible - depart) * (1 - Math.pow(1 - p, 3))));
    if (p < 1) requestAnimationFrame(pas);
  };
  requestAnimationFrame(pas);
}

// ---------------------------------------------------------------- Bulle d'information

const bulle = $('#bulle');
function avecBulle(noeud, texte) {
  noeud.addEventListener('mousemove', (e) => {
    bulle.textContent = texte();
    bulle.hidden = false;
    bulle.style.left = `${Math.min(e.clientX + 12, innerWidth - 250)}px`;
    bulle.style.top = `${e.clientY + 14}px`;
  });
  noeud.addEventListener('mouseleave', () => { bulle.hidden = true; });
}

// ---------------------------------------------------------------- Navigation selon les droits

function afficher(vue) {
  vueCourante = vue;
  for (const s of document.querySelectorAll('main > section')) s.hidden = s.id !== `vue-${vue}`;
  for (const b of document.querySelectorAll('#nav button')) {
    if (b.dataset.vue === vue) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current');
  }
  clearInterval(minuterie);
  const charge = { mes: chargerMes, tableau: chargerTableau, admin: chargerAdmin }[vue];
  if (charge) {
    charge().catch(surErreur);
    minuterie = setInterval(() => charge().catch(surErreur), 15000);
  }
}

function surErreur(e) {
  if (e.statut === 401) { moi = null; construireNav(); afficher('accueil'); }
}

function construireNav() {
  const nav = $('#nav');
  const compte = $('#compte');
  nav.replaceChildren();
  compte.replaceChildren();
  if (!moi) return;
  for (const [cle, vue] of Object.entries(VUES)) {
    if (!moi.droits.includes(vue.droit)) continue;
    const b = el('button', '', vue.titre);
    b.type = 'button';
    b.dataset.vue = cle;
    b.addEventListener('click', () => afficher(cle));
    nav.append(b);
  }
  compte.append(el('span', '', moi.pseudo), el('span', 'badge', moi.role));
  const sortir = el('button', 'bouton secondaire', 'Se déconnecter');
  sortir.type = 'button';
  sortir.addEventListener('click', async () => {
    try { await api('/api/v1/deconnexion', {}); } catch { /* déjà déconnecté */ }
    moi = null; construireNav(); afficher('accueil');
  });
  compte.append(sortir);
}

function entrer(profil) {
  moi = profil;
  construireNav();
  afficher(moi.droits.includes('tableau:lire') ? 'tableau' : 'mes');
}

// ---------------------------------------------------------------- Accueil : connexion et inscription

$('#form-connexion').addEventListener('submit', async (e) => {
  e.preventDefault();
  const f = e.currentTarget;
  const msg = $('.message', f);
  msg.textContent = 'Connexion…';
  try {
    const r = await api('/api/v1/connexion', { pseudo: f.pseudo.value.trim(), mot_de_passe: f.mot_de_passe.value });
    f.reset(); msg.textContent = '';
    entrer({ pseudo: r.pseudo, role: r.role, droits: r.droits });
  } catch (err) { msg.textContent = err.message; }
});

$('#form-inscription').addEventListener('submit', async (e) => {
  e.preventDefault();
  const f = e.currentTarget;
  const msg = $('.message', f);
  msg.textContent = 'Création…';
  try {
    const identifiants = { pseudo: f.pseudo.value.trim(), mot_de_passe: f.mot_de_passe.value };
    await api('/api/v1/comptes', identifiants);
    const r = await api('/api/v1/connexion', identifiants);
    f.reset(); msg.textContent = '';
    entrer({ pseudo: r.pseudo, role: r.role, droits: r.droits });
  } catch (err) { msg.textContent = err.message; }
});

// ---------------------------------------------------------------- Mes appareils (vue la plus simple)

async function chargerMes() {
  const { appareils } = await api('/api/v1/mes-appareils');
  const enLigne = appareils.filter((a) => a.en_ligne).length;
  $('#mes-titre').textContent = appareils.length ? `Bonjour ${moi.pseudo}` : 'Aucun appareil pour l’instant';
  $('#mes-phrase').textContent = appareils.length
    ? `${pluriel(appareils.length, 'appareil lié', 'appareils liés')} à ton compte, dont ${enLigne} en ligne en ce moment.`
    : 'Ouvre NaAnSentinels sur ton PC, onglet « Compte », connecte-toi et active le signal : ton appareil apparaîtra ici.';
  const liste = $('#mes-liste');
  liste.replaceChildren();
  for (const a of appareils) {
    const carte = el('div', 'carte appareil');
    const etat = el('div', 'ligne');
    etat.append(el('span', a.en_ligne ? 'pouls' : 'pouls eteint'), el('strong', '', a.en_ligne ? 'En ligne' : `Vu ${ilYA(Math.max(0, Date.now() / 1000 - a.dernier_vu))}`));
    carte.append(etat, el('div', 'gros', pluriel(a.analyses.total, 'fichier analysé', 'fichiers analysés')));
    const dangers = a.analyses.dangereux;
    carte.append(el('div', '', dangers ? `${pluriel(dangers, 'fichier dangereux repéré', 'fichiers dangereux repérés')}.` : 'Aucun fichier dangereux repéré.'));
    carte.append(el('div', 'note', `NaAnSentinels ${a.version} · ${a.os} · appareil ${a.id}`));
    liste.append(carte);
  }
}

// ---------------------------------------------------------------- Tableau de bord

function appliquerMode() {
  $('#mode-simple').setAttribute('aria-pressed', String(!modeDetail));
  $('#mode-detail').setAttribute('aria-pressed', String(modeDetail));
  for (const d of document.querySelectorAll('#vue-tableau .detail')) d.hidden = !modeDetail;
}
for (const [id, detail] of [['#mode-simple', false], ['#mode-detail', true]]) {
  $(id).addEventListener('click', () => {
    modeDetail = detail;
    try { localStorage.setItem('nas-mode', detail ? 'detail' : 'simple'); } catch { /* stockage indisponible */ }
    appliquerMode();
  });
}

function tuile(conteneur, cle, libelle, valeur, options = {}) {
  let t = $(`[data-tuile="${cle}"]`, conteneur);
  if (!t) {
    t = el('div', options.vedette ? 'tuile vedette' : 'tuile');
    t.dataset.tuile = cle;
    const lib = el('div', 'libelle');
    if (options.pouls) lib.append(el('span', 'pouls'));
    lib.append(el('span', '', libelle));
    t.append(el('div', 'valeur', '0'), lib);
    conteneur.append(t);
  }
  if (options.pouls) $('.pouls', t).className = valeur > 0 ? 'pouls' : 'pouls eteint';
  compter($('.valeur', t), valeur);
}

function barresClassees(conteneur, lignes, unite) {
  conteneur.replaceChildren();
  if (!lignes.length) { conteneur.append(el('div', 'vide', 'Aucun appareil pour l’instant.')); return; }
  const max = Math.max(...lignes.map((l) => l.appareils));
  for (const l of lignes) {
    const ligne = el('div', 'ligne-barre');
    const zone = el('div', 'zone');
    const barre = el('div', 'barre-h');
    barre.style.width = `${Math.max(2, (l.appareils / max) * 78)}%`;
    zone.append(barre, el('span', 'val', nombre(l.appareils)));
    ligne.append(el('span', 'nom', l.nom), zone);
    avecBulle(ligne, () => `${l.nom} : ${pluriel(l.appareils, unite, `${unite}s`)}`);
    conteneur.append(ligne);
  }
}

function verdicts(analyses) {
  const barre = $('#verdicts-barre');
  const legende = $('#verdicts-legende');
  barre.replaceChildren();
  legende.replaceChildren();
  if (!analyses.total) barre.append(el('div', 'vide', 'Aucune analyse remontée pour l’instant.'));
  for (const v of VERDICTS) {
    const n = analyses[v.cle] || 0;
    const part = analyses.total ? (n / analyses.total) * 100 : 0;
    if (n > 0) {
      const seg = el('div', `segment ${v.cle}`);
      seg.style.flexGrow = String(n);
      // La part n'est écrite dans le segment que si elle y tient largement ; sinon la légende la porte.
      if (part >= 9) seg.textContent = `${Math.round(part)} %`;
      avecBulle(seg, () => `${v.nom} : ${nombre(n)} (${part.toFixed(1).replace('.', ',')} %)`);
      barre.append(seg);
    }
    const li = el('li');
    li.append(el('span', `pastille ${v.cle}`, v.symbole), el('span', '', v.nom), el('span', 'chiffre', `${nombre(n)} · ${Math.round(part)} %`));
    legende.append(li);
  }
}

async function chargerTableau() {
  const t = await api('/api/v1/tableau');
  appliquerMode();
  const a = t.analyses;
  $('#tableau-phrase').textContent = t.appareils
    ? `${pluriel(t.connectes, 'appareil connecté', 'appareils connectés')} en ce moment, sur ${nombre(t.appareils)} connus. `
      + `Sur ${pluriel(a.total, 'analyse', 'analyses')}, ${pluriel(a.dangereux, 'fichier dangereux a été repéré', 'fichiers dangereux ont été repérés')}.`
    : 'Aucun appareil n’a encore envoyé de signal. Il apparaîtra ici dès qu’un utilisateur l’activera.';
  const tuiles = $('#tuiles');
  tuile(tuiles, 'connectes', 'connectés maintenant', t.connectes, { vedette: true, pouls: true });
  tuile(tuiles, 'appareils', 'appareils connus', t.appareils);
  tuile(tuiles, 'actifs', 'actifs sur 7 jours', t.actifs_7j);
  tuile(tuiles, 'analyses', 'analyses', a.total);
  tuile(tuiles, 'comptes', 'comptes', t.comptes);
  verdicts(a);
  barresClassees($('#barres-versions'), t.versions, 'appareil');
  barresClassees($('#barres-systemes'), t.systemes, 'appareil');
  const corps = $('#table-appareils tbody');
  corps.replaceChildren();
  for (const ap of t.liste) {
    const tr = el('tr');
    const etat = el('td');
    const boite = el('span', 'etat');
    boite.append(el('span', ap.en_ligne ? 'pouls' : 'pouls eteint'), el('span', '', ap.en_ligne ? 'En ligne' : ilYA(Math.max(0, t.maintenant - ap.dernier_vu))));
    etat.append(boite);
    const id = el('td'); id.append(el('code', '', ap.id));
    tr.append(etat, id, el('td', '', ap.pseudo || 'anonyme'), el('td', '', ap.version), el('td', '', ap.os),
      el('td', 'num', nombre(ap.analyses.total)), el('td', 'num', nombre(ap.analyses.dangereux)));
    corps.append(tr);
  }
  if (!t.liste.length) { const tr = el('tr'); const td = el('td', 'vide', 'Aucun appareil.'); td.colSpan = 7; tr.append(td); corps.append(tr); }
}

// ---------------------------------------------------------------- Administration

async function chargerAdmin() {
  const [{ utilisateurs, roles }, { journal }] = await Promise.all([api('/api/v1/utilisateurs'), api('/api/v1/journal')]);
  const message = $('#admin-message');
  const corps = $('#table-comptes tbody');
  // Ne pas reconstruire la table pendant qu'un menu de rôle est ouvert.
  if (!corps.contains(document.activeElement)) {
    corps.replaceChildren();
    for (const u of utilisateurs) {
      const tr = el('tr');
      const choix = el('select');
      choix.setAttribute('aria-label', `Rôle de ${u.pseudo}`);
      for (const r of roles) { const o = el('option', '', r); o.value = r; o.selected = r === u.role; choix.append(o); }
      const cEstMoi = u.pseudo === moi.pseudo;
      choix.disabled = cEstMoi;
      choix.addEventListener('change', async () => {
        try { await api(`/api/v1/utilisateurs/${u.id}/role`, { role: choix.value }); message.textContent = `${u.pseudo} est maintenant ${choix.value}.`; }
        catch (err) { message.textContent = err.message; }
        choix.blur(); chargerAdmin().catch(surErreur);
      });
      const tdRole = el('td'); tdRole.append(choix);
      const bouton = el('button', 'bouton secondaire', u.actif ? 'Désactiver' : 'Réactiver');
      bouton.type = 'button';
      bouton.disabled = cEstMoi;
      bouton.addEventListener('click', async () => {
        try { await api(`/api/v1/utilisateurs/${u.id}/actif`, { actif: !u.actif }); message.textContent = `${u.pseudo} : compte ${u.actif ? 'désactivé' : 'réactivé'}.`; }
        catch (err) { message.textContent = err.message; }
        bouton.blur(); chargerAdmin().catch(surErreur);
      });
      const tdAction = el('td'); tdAction.append(bouton);
      tr.append(el('td', '', cEstMoi ? `${u.pseudo} (toi)` : u.pseudo), tdRole, el('td', 'num', nombre(u.appareils)), el('td', '', u.actif ? 'Actif' : 'Désactivé'), tdAction);
      corps.append(tr);
    }
  }
  const lignes = $('#table-journal tbody');
  lignes.replaceChildren();
  for (const j of journal) {
    const tr = el('tr');
    tr.append(el('td', '', dateHeure(j.quand)), el('td', '', j.acteur), el('td', '', j.action), el('td', '', j.detail));
    lignes.append(tr);
  }
}

// ---------------------------------------------------------------- Démarrage

(async () => {
  try { entrer(await api('/api/v1/moi')); }
  catch { afficher('accueil'); }
})();
