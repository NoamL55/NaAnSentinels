# Feuille de route de NaAnSentinels

**Langues :** [English](../en/roadmap.md) · Français · [Español](../es/hoja-de-ruta.md)

## Fait

- Moteur d'analyse statique : structure PE, sections, fonctions importées, entropie, SHA-256, désassemblage x86/x64.
- Verdict expliqué, aucune action destructrice.
- Anti-phishing hors-ligne (mail collé ou fichier `.eml`).
- Règles et config éditables, rechargeables depuis l'application.
- Vue simple animée et vue experte avec schémas.
- Signal de présence facultatif et anonyme, compte facultatif.
- Serveur : comptes, rôles (utilisateur, analyste, admin), tableau de bord, journal de sécurité.
- Signature des versions officielles (Ed25519).

## Ensuite

### Anti-phishing relié à la boîte mail

- Connexion IMAP facultative, mot de passe d'application rangé hors du dépôt et jamais affiché.
- Lecture des en-têtes d'authentification : SPF, DKIM, DMARC.
- Détection de domaines sosies.

### Reverse engineering approfondi

- Suivi des appels depuis le point d'entrée et arbre d'appels.
- Détection de packers connus, imports retardés, chaînes chiffrées.
- Règles YARA en plus du format maison.
- Vérification de la signature Authenticode des fichiers analysés.
- Consultation d'une base de réputation par empreinte, sans envoyer le fichier.

### Surveillance

- Surveillance d'un dossier (Téléchargements) : analyse de chaque nouveau programme avant son ouverture.
- Quarantaine locale réversible, toujours sur décision de l'utilisateur.
- Analyse d'un dossier entier avec synthèse.

### Serveur

- Limites de débit persistantes et prise en charge d'un proxy de confiance.
- Double authentification, réinitialisation de mot de passe.
- Historique des appareils connectés dans le temps.

## Hors périmètre (assumé)

- Pas de pilote noyau, pas d'interception d'exécution au niveau du système, pas d'action administrateur.
  NaAnSentinels reste un outil en espace utilisateur qui **analyse et explique**.
- Il ne remplace pas l'antivirus du système : il le complète.
