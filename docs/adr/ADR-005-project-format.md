# ADR-005 — Format de projet

- Statut : accepte pour le MVP
- Date : 2026-09-22

## Contexte

Un projet reference plusieurs sources DSL, assets et executions de calcul. Les sources doivent rester lisibles et fusionnables par Git. Les maillages et resultats d'optimisation, simulation ou entrainement peuvent etre volumineux et reconstructibles.

## Options considerees

- dossier + manifest TOML : lisible, diffable, simple a inspecter et a migrer.
- archive ZIP monolithique : facile a transporter, mauvaise edition collaborative et mises a jour non atomiques.
- SQLite : transactions et requetes, mais sources et diffs opaques.
- DSL unique : simple au debut, inapte aux assets et artefacts lourds.

## Decision (cible normative)

Un projet de travail est un dossier avec un manifest `project.rgnproj` en TOML versionne :

```text
my_robot/
  project.rgnproj
  src/*.rgn
  assets/
  generated/{meshes,simulation,optimization,training}/
```

Le manifest contient `schema_version`, UUID et nom du projet, entrees DSL ordonnees, assets et configurations de runs. Tous les chemins sont relatifs a la racine, normalises, sans echappement `..` ni symlink non autorise lors d'un import.

Chaque artefact genere a un sidecar de metadonnees avec hash de contenu des entrees, backend et version, options, unite, date et statut. Les sources DSL et assets faisant autorite ne sont jamais remplaces par un cache. L'ecriture du manifest utilise fichier temporaire, synchronisation puis renommage atomique.

Un export de partage pourra emballer le dossier en archive, mais l'archive n'est pas le format d'edition.

## Etat implemente au 2026-09-22

[ProjectManifest et load_manifest](../../crates/robogen-project/src/lib.rs) couvrent seulement `schema_version` (valeur par defaut 1), `name` et un chemin `source` unique. Il n'y a pas d'UUID de projet, de liste ordonnee de sources/assets ou de configurations de runs dans ce manifest. Le chargeur deserialise la version sans rejeter les versions inconnues.

Le controle de chemin rejette les chemins absolus et les composants autres que `Component::Normal`, dont `..`. Il est lexical : la lecture par `root.join(source)` suit les symlinks sans verifier le confinement de leur cible dans la racine. Il ne faut donc pas annoncer ce chargeur comme une frontiere sure pour les projets non fiables.

Il n'existe ni `ProjectStore`, ni writer de manifest atomique, ni migrations avec sauvegarde/rollback, ni gestion des sidecars d'artefacts decrits ci-dessus. M3 doit valider chargement/enregistrement, versions et chemins (responsables Project et ARCHITECT) ; la sauvegarde robuste et les migrations avec fixtures/rollback restent au backlog M11. Le present bilan ne modifie pas le schema persistant et ne choisit aucun nouveau stockage.

## Consequences

- projets inspectables et amicaux pour Git ;
- les resultats lourds peuvent etre ignores, nettoyes et reconstruits ;
- les migrations doivent etre versionnees et testees avec fixtures anciennes ;
- deplacer manuellement un fichier exige de mettre a jour le manifest ;
- charger un projet non fiable impose validation de chemins, tailles et formats avant allocation.

## Strategie de remplacement

`robogen-project` est le seul composant qui lit/ecrit le manifest. Une future base SQLite ou archive implemente le meme `ProjectStore`. Un export canonique dossier+TOML reste disponible pour la portabilite. Toute migration preserve les UUID et cree une sauvegarde avant mutation irreversible.

Le format ne serialise jamais les handles de kernel, GPU ou physique ; changer de backend invalide seulement les artefacts concernes.

