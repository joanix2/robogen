---
title: "M1 - Consolider le shell desktop selon la maquette"
lane: todo
created: 2026-09-22
updated: 2026-09-22
description: "Valider les trois vues natives et leur fidelite visuelle sans faux calculs."
---

## Conversation

### user

Decliner le jalon UI du cahier des charges en tache executable.

### agent

Source : [cahier des charges](../../docs/init-prompt.md), sections 9 a 11, 14, 17, 19, 22 et 30 ; maquette jointe a la conversation.

Responsables : UI et RENDER. Dependance : [M0](task_20260922_00_m0-foundations.md).

Le shell sombre Design/Optimisation/Apprentissage est documente comme existant. Auditer et completer cette base native Rust, sans creer une application web ou trois modeles independants.

Plan : barre RoboGen et workflow, taxonomie issue du domaine, viewport dominant, onglet DSL, assistant desactive, panneaux d'optimisation et d'apprentissage. Les actions non branchees sont explicitement indisponibles ; aucune courbe ni metrique ne doit simuler un calcul reel. L'historique assistant de demonstration doit etre identifie comme tel.

Acceptation : navigation fonctionnelle, interface lisible apres redimensionnement, controles raccordes a un etat explicite, captures des trois vues dans docs/screenshots. Assistant IA et foundation model non configures, solver et RL indisponibles tant que leurs jalons ne sont pas livres. Les mutations branchees passent par des commandes annulables.

Verification : tests unitaires des etats, integration de navigation/commandes, lancement desktop et inspection visuelle ; les trois gates Rust de M0 passent. Documenter les limites dans TECHNICAL.md.

Checklist : [M1](todo_20260922_01_m1-desktop.md).

### user

ajoute git git ignore et commit entre chaque ticket

### agent

Initialisation d'un depot Git propre a RoboGen, distinct du depot detecte dans le dossier parent Documents. Git et l'identite utilisateur sont deja configures ; aucune configuration globale ni depot distant modifie.

Exclusions ajoutees pour les toolchains locales, builds, environnements Python, caches Graphify, logs, fichiers .env et export STL de demonstration. Le corpus scientifique PDF de 409 Mo reste local ; son catalogue et ses notices restent versionnables. Cargo.lock, profils d'agents, CI, sources, tests et taches restent suivis.

La regle ajoutee a AGENTS.md impose un commit local apres chaque ticket valide et avant le suivant, avec verification du diff et des tests ; seul l'orchestrateur ecrit dans Git. Aucun push autorise implicitement. Le premier commit est un instantane initial, pas une cloture retroactive de M1.

Etat lors de l'initialisation : M0 valide ; M1 en cours. Les delegues ont ajoute ComponentTaxonomy dans le domaine et SourceDocument avec historique undo/redo dans le projet, ainsi qu'un test vertical 40 -> 70 mm -> undo/redo -> STL. Ces ajouts ne sont pas encore raccordes au shell ; M1 ne doit pas etre coche termine.

### user

En fait j'aimerais avoir deux top bars, une pour tout le logiciel avec juste le nom du logiciel et l'onglet profil a droite, et une autre pour le reste avec les differents onglets, un peu comme dans Blender.

### agent

Revision implementee dans robogen-ui : application_bar reservee a RoboGen et Profil, workspace_bar en dessous pour Design/Optimisation/Apprentissage et les actions projet. Suppression du slogan et des initiales fictives dans l'en-tete. Le menu Profil indique explicitement qu'aucun profil n'est configure ; pas de backend compte ajoute. Les commandes undo/redo, export et camera existantes sont conservees.

Validation : 12 tests UI passent, dont le test adapte aux deux lignes qui controle le profil a droite, l'ouverture du menu et les commandes a 1080x680. Les trois gates Rust workspace passent (41 tests) et le binaire desktop compile. Captures natives Design inspectees a 1440x831 et 1080x680 dans docs/screenshots/m1-design.png et m1-design-compact.png. TECHNICAL.md et la checklist sont actualises.

Cette demande est traitee ; M1 reste en cours, notamment pour les captures natives Optimisation et Apprentissage et sa synthese complete. Aucun commit de cloture M1 premature ; les modifications en cours sont preservees pour le commit du ticket valide.

### user

Je voudrais pouvoir recreer un servo moteur par exemple de maniere procedurale dans le DSL et que ca serve de nouveau mot pour le langage. Reference visuelle : definition reutilisable, primitives, translations, union et soustraction.

### agent

Verification locale : Declaration ne contient actuellement que Parameter, Material, Sketch et Part ; Expr ne contient que nombres, references et chaines. CadKernel expose seulement extrude. Un constructeur servo procedural reutilisable n'est donc pas encore pris en charge ; reconnaitre son nom dans le parser seul ne suffirait pas.

Choix recommande et retenu en l'absence de l'utilisateur : conserver l'ordre des tickets et ne pas suspendre ou declarer termine M1. Preparer l'extension du plan M2/CAD apres sa cloture. La proposition et les criteres de validation sont consignes dans docs/dsl/README.md : composant parametrique utilisateur, arguments nommes et valeurs par defaut, unites, imports/namespaces, composition, instances independantes et operations geometriques reelles. Exemple de syntaxe explicitement non executable aujourd'hui ; aucune modification du compilateur ou du backend faite pour cette demande a ce stade.

Le symbole servo sera defini en .rgn plutot qu'ajoute comme mot-cle reserve. Distinguer geometrie du composant et comportement d'actionneur physique. La capture illustre la composition ; aucun editeur visuel de blocs n'est implicitement ajoute au perimetre.

Verification documentaire : syntaxe proposee distinguee de l'exemple executable existant ; limites de la implementation confirmees dans DSL/CAD. Cette capacite reste a implementer et ne doit pas etre annoncee comme livree.

### user