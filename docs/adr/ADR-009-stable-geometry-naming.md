# ADR-009 — Nommage geometrique stable

- Statut : accepte pour le MVP
- Date : 2026-09-22

## Contexte

Un kernel peut renumeroter faces, aretes et sommets apres une modification parametrique. Persister `face[13]` casserait trous, assemblages, charges et selections. Aucun algorithme ne garantit une identite parfaite lorsque la topologie se scinde, fusionne ou disparait ; les ambiguïtes doivent donc etre visibles.

## Options considerees

- indices du kernel : simple et fragile, rejete.
- signature uniquement geometrique : utile en repli, instable avec symetries et tolerances.
- provenance de feature uniquement : robuste pour creation directe, insuffisante apres booleens complexes.
- combinaison provenance + role + requete + signature : davantage de metadonnees, meilleure explicabilite.

## Decision (cible normative)

Une `GeometryRef` persistante appartient a RoboGen et contient :

- l'`EntityId` de la feature productrice et, si utile, de l'operande source ;
- le type d'entite (`Face`, `Edge`, `Vertex`) ;
- un role semantique emis par l'operation (`Extrude.CapStart`, `Hole.Wall`, etc.) ;
- une chaine de provenance a travers conservation, scission et fusion ;
- un `GeometrySelector` declaratif (planarite, normale, rayon, adjacence, proximite a un repere nomme) ;
- une signature quantifiee de repli (classe de surface/courbe, aire/longueur, centroid, orientation, voisinage), jamais utilisee comme identite absolue.

Pendant un rebuild, l'adaptateur CAD emet une table d'evolution. La resolution procede par role/provenance, puis filtre par selecteur, puis classe par signature avec tolerances liees au document. Un resultat unique est accepte. Zero resultat donne `MissingReference`; plusieurs candidats non separables donnent `AmbiguousReference` avec candidats pour reparation utilisateur. Aucun premier candidat arbitraire n'est choisi.

Les references temporaires de picking sont converties en `GeometryRef` avant d'entrer dans une commande. Les IDs natifs du kernel restent limites a la revision courante.

## Etat implemente au 2026-09-22

[robogen-cad](../../crates/robogen-cad/src/lib.rs) definit `FaceRef::FeatureOutput { feature: FeatureId, role: FaceRole }` et `FaceRef::Semantic(String)`, avec les roles `StartCap`, `EndCap` et `Side(u32)`. Cette enum serialisable n'est pas un resolver et ne prouve pas la stabilite d'une reference apres rebuild ; le numero de cote n'offre pas a lui seul une garantie de provenance.

Le backend d'extrusion retourne un `Mesh` de sommets/triangles, sans table d'evolution topologique. Il n'existe pas de chaine complete `GeometryRef`/`GeometrySelector`, de matching role/provenance/signature ou de diagnostics `MissingReference`/`AmbiguousReference` implementes. La conversion du picking en reference persistante n'est pas raccordee. M3 doit etablir et tester la provenance et la selection de la tranche CAD (responsables CAD, RENDER et orchestration) ; cela ne vaut pas validation du nommage general apres scission, fusion ou booleens. La cible ci-dessus est conservee, sans migration ni nouveau schema decide ici.

## Consequences

- les editions courantes de dimensions conservent davantage de references ;
- les echecs sont explicables et reparables ;
- chaque operation CAD doit decrire la provenance de ses sorties ;
- stockage et tests supplementaires ;
- les changements topologiques reels peuvent encore exiger une intervention utilisateur.

## Strategie de remplacement

Le schema `GeometryRef` est versionne. Un algorithme de matching plus avance peut remplacer le resolver sans changer le DSL tant qu'il preserve roles et diagnostics. Lors d'un changement de kernel, reconstruire les tables de provenance et executer le corpus de mutations parametriques (variation, apparition/disparition, symetrie, booleen).

Si un kernel fournit un historique topologique fiable, l'adaptateur le convertit vers la table RoboGen au lieu de persister ses identifiants.

