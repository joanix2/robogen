---
title: "Optimisation topologique et profils d'impression parametrables"
lane: todo
created: 2026-10-07
updated: 2026-10-07
description: "En attente : alleger des volumes sous contraintes mecaniques et FDM, avec profils machine, materiau et solveur editables."
---

## Conversation

### user

Creer un ticket pour l'optimisation de matiere et l'imprimabilite, laisser
cette tache de cote pour le moment et rendre tout cela parametrable.

### agent

## Statut et dependances

Ticket differe a la demande de l'utilisateur. Aucune implementation lancee.
Reprendre sur autorisation explicite, pas avant les prerequisites du
[jalon M7](task_20260922_07_m7-topopt-beam.md). Completer sa base FEM/SIMP,
sans dupliquer son solveur ni declarer ce jalon termine. Les charges issues
de simulation restent dans le perimetre M8. Respecter ADR-008 et documenter
les nouveaux contrats persistants avant implementation.

## Objectif

Partir d'une enveloppe grossiere, conserver les interfaces utiles et reduire
la matiere sous contraintes explicites de rigidite et de fabrication FDM.
La tenue mecanique doit etre evaluee selon les capacites reelles du modele ;
une minimisation de compliance n'est pas une certification de resistance.

## Parametres a exposer

- Conception : volume admissible, zones preservees, volumes exclus, interfaces
  et fixations, avec references geometriques stables et provenance.
- Mecanique : cas de charge, forces/couples, appuis, limites de deplacement,
  proprietes du materiau avec unites, provenance et domaine de validite.
  Prevoir les limites de resistance et marges de securite sans les presenter
  comme garanties si le solveur ne sait pas les verifier.
- Machine : profil nomme, volume utile, diametre de buse, diametre de filament,
  limites de temperature et capacites de la machine, sans valeurs imposees
  pour des caracteristiques encore inconnues.
- Fabrication : hauteur de couche, largeur de ligne, nombre de parois,
  couches pleines haut/bas, motif et taux de remplissage, temperatures,
  vitesses, refroidissement, orientation et politique de supports/surplombs.
- Geometrie imprimable : epaisseurs et sections minimales, jeux, contraintes
  directionnelles et acces pour retrait des supports. Distinguer contraintes
  reellement appliquees par le solveur et controles apres reconstruction.
- Optimisation : fraction volumique cible, resolution FEM, filtre, penalisation
  SIMP, tolerances, limite d'iterations, budget de calcul et reconstruction.
  Fournir des valeurs par defaut documentees et des bornes de validation.

## Prereglage fourni par l'utilisateur

Finder ; buse 0.4 mm ; Flashforge-PLA ; filament 1.75 mm ; profil Standard ;
extrudeuse configuree a 220 C. Toutes ces valeurs doivent etre modifiables.
Ne pas en deduire la generation de Finder, le volume utile, les proprietes
mecaniques ou les autres parametres absents de la capture. Les informations
manquantes ne bloquent pas ce ticket en attente ; les demander a la reprise
seulement lorsqu'elles sont necessaires au calcul ou a l'export.

## Comportement et validation attendus

- Profils nommes sauvegardables, duplicables et reutilisables ; selection du
  profil et substitutions par projet/piece, avec priorites explicites.
  Edition UI et source coherentes, commandes annulables, schema versionne,
  parametres/unites types et rattachement au document de reference.
- Valeurs inconnues explicites ; diagnostics pour valeurs invalides,
  incoherentes ou non prises en charge. Aucun parametre silencieusement ignore.
- Calcul hors UI, progression, annulation et rejet des resultats obsoletes.
  Garder un etat indisponible tant que le backend n'est pas fonctionnel.
- Reverification mecanique de la geometrie reconstruite ; limites de la
  modelisation isotrope expliquees pour les couches FDM. Ne pas assimiler
  directement remplissage du trancheur et densite SIMP. Pas de constantes
  DisplayOnly pour justifier une solidite.
- Comparaison reference/optimise : volume, masse lorsque la densite est connue,
  deplacements, compliance, convergence et controles d'imprimabilite.
  Temps d'impression et supports uniquement issus d'un trancheur configure
  ou affiches indisponibles ; aucune equivalence automatique volume/temps.
- Tests unitaires et integration sauvegarde/rechargement, edition/undo,
  invalidation et calcul ; demonstration native, documentation et gates Rust.
  Pour une affirmation sur une piece imprimee : calibration et essais sous
  charge, avec conditions et limites rapportees.

Checklist : [TODO](todo_20261007_13-topopt-print-profile.md).
Origine : [discussion servo/CAD](task_20261007_12-servo-cad.md).

### user