# Mini bipède — disposition des composants

Première implantation paramétrique de **17 composants existants** : dix servos
pour les jambes, quatre pour les bras, une carte embarquée, une batterie et une
caméra. Les modèles du commerce gardent leurs dimensions et leurs limites de
provenance ; aucune mise à l'échelle pour les faire rentrer dans le robot.

Ouvrir **Exemples > Mini bipède — disposition**, ou lancer :

```sh
cargo run -p robogen-desktop -- examples/mini_biped/main.rgn
cargo run -p robogen-cli -- check examples/mini_biped/main.rgn
cargo run -p robogen-cli -- export-stl examples/mini_biped/main.rgn /tmp/mini-biped-layout.stl
```

Le STL est une disposition multicorps des composants, en millimètres, sans
couleurs. Ce n'est pas un ensemble de supports à imprimer. Le modèle est
volontairement sans structure porteuse : les supports doivent être générés par
optimisation topologique à partir de contraintes, pas dessinés arbitrairement.

## Implantation et paramètres

X vers la gauche du robot, Y vers l'arrière, Z vers le haut ; avant = -Y.
La pose initiale garde les enveloppes des 17 composants séparées. Dimensions
hors tout : environ **190 × 69 × 362,1 mm**, de Z=5,4 à 367,5 mm. Aucun pied
structurel n'est encore généré et Z=0 n'est pas un contact physique calculé.

| Paramètre | Valeur initiale | Effet |
|---|---:|---|
| `LEG_SPACING` | 110 mm | Écartement des axes de jambes |
| `SHIN_LENGTH` | 65 mm | Distance verticale cheville de tangage–genou |
| `THIGH_LENGTH` | 65 mm | Distance verticale genou–hanche de tangage |
| `ANKLE_ROLL_Z` | 16 mm | Hauteur du repère de cheville de roulis |
| `ANKLE_PITCH_Z` | 50 mm | Hauteur du repère de cheville de tangage |
| `SHOULDER_SPACING` | 190 mm | Écartement des axes de bras |
| `UPPER_ARM_LENGTH` | 65 mm | Distance verticale épaule–coude |
| `BATTERY_REAR_OFFSET` | 40 mm | Position arrière du pack |

Les hauteurs du genou, des hanches, du torse, des épaules, des coudes et de la
caméra sont des équations dérivées de ces paramètres dans `main.rgn`. Elles
sont éditables dans Code DSL et bénéficient d'annuler/rétablir. La compilation
native s'effectue hors du thread UI ; l'export attend un résultat valide.
Les angles de `rotate` orientent les boîtiers entiers : ils n'actionnent pas
les arbres des servos et ne représentent pas des angles de liaison simulée.

Chaque jambe prévoit hanche en roulis/tangage, genou en tangage et cheville en
tangage/roulis. Chaque bras prévoit épaule et coude en tangage. Ces 14 axes
sont une hypothèse d'architecture ; aucune quantité de servos réellement
possédés, course, puissance ou aptitude à marcher n'est déduite du modèle.
Les repères de pose des servos sont au centre de leur face d'arbre de sortie.
La carte est verticale ; la caméra regarde vers -Y ; le pack est horizontal
près du bassin, avec les fils et le connecteur rectilignes du modèle initial.

## Supports par contraintes : suite prévue

Le [ticket 15](../../.agentkanban/tasks/task_20261007_15-biped-constraints-topopt.md)
porte le DSL déclaratif des interfaces, liaisons, domaines de conception,
zones préservées/exclues, charges, matériaux et objectifs. Les supports
(bassin, cuisses, tibias, pieds, torse et bras) seront des artefacts générés
et recalculables. Les volumes de débattement, passages de câbles et accès aux
vis devront être exclus ; les interfaces de sortie et de fixation sont
distinctes et doivent être mesurées, notamment les palonniers non modélisés.

Les équations actuelles définissent uniquement la disposition. Il n'existe
pas encore de solveur de contraintes d'assemblage ni de `OptimizationSpec`
exécutable. Le backend TopOpt est explicitement indisponible : M7 doit valider
FEM/SIMP, M8 les charges robotiques, et le ticket 13 les profils d'impression.
Aucun support optimisé ou domaine de conception n'est représenté par une
pièce factice dans cette scène.

La séparation des enveloppes est testée pour cette pose et ces dimensions,
pas pour tous les mouvements ou toutes les éditions possibles. Masses,
centres de masse, couples des servos, efforts de marche et propriétés du PLA
imprimé restent à renseigner avant le calcul mécanique. Les matériaux des
modèles visuels ne sont pas des données mécaniques validées.

## Provenance et vérification

Les définitions sont copiées par le mécanisme de bibliothèque depuis les
exemples [servo](../servo/README.md), [carte](../compute_board/README.md),
[caméra](../raspberry_pi_camera/README.md) et [batterie](../li_ion_battery/README.md),
car les imports DSL restent indisponibles. Un test compare ces copies aux
sources de bibliothèque pour détecter une divergence.

Les tests couvrent unités et rotations, orientation et fermeture des maillages,
conservation des couleurs, séparation des enveloppes initiales, dimensions
commerciales, déplacement paramétrique avec IDs stables, export STL,
chargement asynchrone, visibilité et historique UI. Ils ne prouvent ni marche,
ni résistance, ni compatibilité électrique.
