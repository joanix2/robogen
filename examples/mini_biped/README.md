# Mini bipède — composants et préparation des supports

Première implantation paramétrique de **17 composants existants** : dix servos
pour les jambes, quatre pour les bras, une carte embarquée, une batterie et une
caméra. Les modèles du commerce gardent leurs dimensions et leurs limites de
provenance ; aucune mise à l'échelle pour les faire rentrer dans le robot.

Ouvrir **Exemples > Mini bipède — préparation des supports**, ou lancer :

```sh
cargo run -p robogen-desktop -- examples/mini_biped/main.rgn
cargo run -p robogen-cli -- check examples/mini_biped/main.rgn
```

La scène comporte maintenant **17 composants et 11 préparations de supports**.
L'export global est bloqué tant que les supports ne sont pas calculés : leurs
domaines ne sont jamais des solides à imprimer. Les supports doivent être générés
par optimisation topologique à partir de contraintes, pas dessinés arbitrairement.

Conception affiche les domaines en bleu, contacts conservés en vert et exclusions
en rouge. Optimisation sélectionne chaque support indépendamment, avec la caméra
sélectionnée initialement. Le calcul reste désactivé pour ces préparations :
matériau mécanique, charges/appuis et limites restent explicitement manquants,
sauf pour **LeftShinSupport**, qui dispose désormais des hypothèses ci-dessous.

## Tibia gauche : hypothèses provisoires PLA

Le 2026-10-08, le PLA et l'utilisation d'estimations ont été autorisés par
l'utilisateur. `left_shin_PLA_estimated` utilise ces valeurs dans
[main.rgn](main.rgn), sans changer les composants achetés. Ce n'est pas une
qualification mécanique : les dix autres supports restent incomplets.

| Entrée | Valeur provisoire | Hypothèse à remplacer |
|---|---:|---|
| Matériau `PLA_Estimated` | 1 240 kg/m³ ; E = 2 GPa ; ν = 0,35 | PLA imprimé assimilé à un solide homogène isotrope ; pas de calibration par éprouvette |
| Résistance nominale | 25 MPa | Valeur indicative, **non vérifiée par le solveur** |
| `ESTIMATED_SHIN_VERTICAL_FORCE` | 40 N | Robot supposé de 2 kg, appui sur une jambe, facteur dynamique 2 : 2 × 9,81 × 2 ≈ 40 N |
| `ESTIMATED_SHIN_LATERAL_FORCE` | 10 N | Sollicitation transversale arbitraire exploratoire, pas un effort de marche simulé |
| `ESTIMATED_SHIN_MAX_DISPLACEMENT` | 0,5 mm | Limite provisoire de déplacement nodal maximal |

Les cas indépendants appliquent respectivement **[10, 0, -40] N** et
**[0, 10, -40] N** à l'interface proximale. Ils ne couvrent pas toutes les
directions, les couples des servos, les chocs, la fatigue ou le flambement.
La masse est minimisée sous la limite de déplacement, pas à volume imposé.

Les interfaces `estimated_*_rigid_transfer` supposent un transfert rigide idéal,
sans souplesse de vis ni de palonnier. À la cheville, deux zones de 8 × 4 × 5 mm
sont déportées de 8 mm des centres des trous, de part et d'autre des pattes,
et encastrées. Au genou, une zone cylindrique de rayon 8 mm et d'épaisseur 5 mm
est placée à 4 mm de la face de sortie du servo et reçoit les forces.
Ces zones ne sont **ni des fixations réelles, ni des adaptateurs dessinés** :
elles idéalisent les transferts qui devront être remplacés par le montage mesuré.
Leurs dimensions/déports sont les paramètres `ESTIMATED_*_PAD_*` du DSL.

Les enveloppes et perçages connus restent exclus. La projection à maille cible
2,5 mm donne 28 × 24 × 36 cellules, soit 24 192, sans conflit avec ces zones
idéalisées. Cela **ne résout pas la géométrie des fixations de 2 mm**. Les anneaux
de contact exacts sont conservés dans la bibliothèque de préparations, mais ne
servent pas d'appuis à ce modèle exploratoire du tibia.

Le moteur Julia a été supprimé (ADR-014). La minimisation de masse est
indisponible, même avec des entrées complètes ; aucun calcul n'est déclenché au
chargement. Les préparations restent éditables et l'export reste interdit.
Avant fabrication : montage réel, anisotropie et essais PLA, résistance,
paramètres d'impression, débattements et étude de convergence sont à vérifier.

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

## Supports par contraintes

Le [ticket 15](../../.agentkanban/tasks/task_20261007_15-biped-constraints-topopt.md)
porte le DSL déclaratif des interfaces, liaisons, domaines de conception,
zones préservées/exclues, charges, matériaux et objectifs. Les supports
(bassin, cuisses, tibias, pieds, torse et bras) seront des artefacts générés
et recalculables. Les volumes de débattement, passages de câbles et accès aux
vis devront être exclus ; les interfaces de sortie et de fixation sont
distinctes et doivent être mesurées, notamment les palonniers non modélisés.

Les onze opérations `topology` préparent caméra, torse, bassin, cuisses, tibias,
pieds et bras. Leurs domaines sont des propositions paramétriques de conception,
pas des formes optimisées. `SUPPORT_WIDTH`, `SUPPORT_DEPTH`, `FOOT_WIDTH`,
`FOOT_LENGTH`, `FOOT_HEIGHT`, `CONTACT_RADIUS` et `CONTACT_THICKNESS` sont éditables.
Les anneaux de contact utilisent les centres et diamètres des trous déclarés ;
leur rayon extérieur et épaisseur sont des choix de conception, pas des cotes
fabricant. Les sorties de servos/palonniers ne sont pas modélisées comme interfaces.

Les exclusions couvrent les enveloppes locales des composants voisins et les
perçages connus. Les volumes de mouvement, outils et câbles flexibles, marges de
montage et séparation entre futurs supports restent à compléter. Les domaines
peuvent se chevaucher ; cette préparation ne valide pas un assemblage articulé.
Le support caméra référence les trous supérieurs de la carte comme proposition
de montage, sans présumer que son PCB peut reprendre les efforts.

Pour les dix supports encore incomplets, `material: unknown`, `load_cases: []`
et `constraints: []` rendent ces inconnues
explicites. Le matériau de présentation de la `part` n'est pas réutilisé pour le
calcul. Project/CLI/TopOpt refusent une requête incomplète avec E347. Le moteur
actuel fonctionne pour la rigidité sous budget de matière, mais ne sait pas encore
minimiser la masse sous les contraintes requises : un backend adapté reste à
valider. Aucune propriété de résistance ou de fabrication n'est revendiquée.

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

Ils vérifient aussi onze préparations distinctes, leurs trois couches, l'inclusion
des contacts dans les bornes des domaines, l'absence de résultat et le refus E347.
L'export STL testé à bas niveau porte uniquement sur les 17 composants ; l'UI et
la CLI interdisent l'export global de cette scène incomplète.

## Données recherchées le 2026-10-08

La première livraison mécanique visée est désormais **LeftShinSupport**. Le
prototype Julia exploratoire a été retiré le 2026-10-20 ; sa qualification et
ses mesures antérieures restent dans l'historique Git. Aucun moteur minimum
de masse n'est actuellement raccordé.
Les valeurs constructeur ci-dessous ne sont pas injectées
dans le DSL : une fiche fabricant ne prouve pas l'identité du composant possédé.

| Élément | Source et donnée | Statut pour ce robot |
|---|---|---|
| Servo candidat | [TowerPro SG90 digital](https://www.towerpro.com.tw/product/sg90-7/) : 9 g, 23 × 12,2 × 29 mm, couple de blocage 1,8 kgf·cm à 4,8 V (environ 0,17652 N·m) | Non identifié : largeur 12,2 mm contre 11,8 mm dans le DSL. Ce n'est pas un couple de service. |
| Palonnier et vis | La page TowerPro montre un jeu de bras, sans toutes les cotes nécessaires au montage choisi | Référence, trous, engagement, vis et course à confirmer ; aucune cannelure inventée. |
| Caméra 2013 | [Raspberry Pi, tableau matériel](https://www.raspberrypi.com/documentation/accessories/camera.html#hardware-specification) : Camera Module 1, OV5647, 3 g, environ 25 × 24 × 9 mm | Donnée constructeur nominale de la révision correspondante ; confirmer le module réel, nappe/vis et centre de masse. |
| Carte embarquée | [NVIDIA Orin Nano, implantation](https://docs.nvidia.com/jetson/orin-nano-devkit/user-guide/latest/hardware_layout.html) distingue module et carte porteuse | Ne permet pas d'identifier la photo du modèle 104 × 90 × 37 mm. Masse, centre de masse et trous restent non vérifiés. |
| Batterie | [Provenance existante](../li_ion_battery/README.md) : pack OEM 18 × 68 mm, comparaison à 53 g | Ni fabricant ni masse du pack réel établis ; pas de transfert des caractéristiques d'une cellule nue ou d'un autre pack. |
| PLA | [Flashforge HS PLA](https://www.flashforge.com/products/hs-pla) : filament 1,75 mm, buse 190–240 °C, couches 0,12–0,3 mm | HS PLA n'est pas identifié comme le « Flashforge-PLA » du profil Finder. Cette page ne fournit pas le jeu de propriétés mécaniques imprimées requis. |

Attention : les trous de la carte embarquée sont **estimés**, comme le précise
son README, contrairement aux trous cotés du dessin caméra. Les contacts de
préparation ne les transforment pas en interfaces exactes validées.

Pour le tibia, il manque toujours l'interface proximale de transmission au genou,
la confirmation du montage, les masses/centres de masse et les courses/couples
utilisables. Pour le PLA imprimé : référence/lot, orientation, parois/remplissage,
module, coefficient de Poisson, résistance admissible et conditions d'essai.
Les charges statiques seront dérivées de ces entrées et des poses retenues ; ni
charge de marche ni marge dynamique ne sera déduite d'une simple ressemblance.
