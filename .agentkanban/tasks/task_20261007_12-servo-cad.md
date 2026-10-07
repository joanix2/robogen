---
title: "Servomoteur parametrique - CAD executable"
lane: todo
created: 2026-10-07
updated: 2026-10-07
description: "Reproduire le servomoteur des captures, reparer les booleens et valider edition, apparences et STL."
---

## Conversation

### user

Creer un ticket avec un plan et implementer les etapes proposees pour reproduire le servomoteur des captures dans RoboGen.

### agent

## Plan autorise

1. Reparer la projection CAD : les tests d'union de cubes et de soustraction echouent sur les volumes. Hypothese locale initiale : incoherence des plans ou de la triangulation apres transformation dans l'adaptateur csgrs. Discriminant : tests CAD existants et contrat des plans transformes.
2. Ajouter un exemple executable dans examples/servo avec servo_main(jeu), servo(), boitier 11.8 x 22.5 x 22.7 mm, pattes, bossages, axe et ouvertures d'apres les captures. Le jeu dilate seulement le boitier comme sur la reference ; les valeurs ne constituent pas une fiche constructeur.
3. Conserver la source .rgn et le SemanticModel comme verite. Verifier composition, instances independantes, diagnostics, modifications de jeu et undo/redo jusqu'au mesh et STL.
4. Porter des apparences depuis la source jusqu'a l'apercu pour distinguer boitier bleu, axe clair et connecteur noir. Documenter le contrat avant modification des DTO. Le STL ne porte pas de couleurs.
5. Rendre l'exemple accessible depuis le desktop et verifier le parcours natif, la camera, l'edition et l'export. Mesurer le temps de construction ; tout travail long doit etre deplace hors UI avec garde de revision et annulation cooperative.
6. Documenter les limites, executer les tests cibles puis fmt, Clippy et tests workspace. Capturer une preuve native si l'environnement graphique le permet. Ne pas declarer M1/M3 complets par ce ticket.

## Criteres d'acceptation

- Union et difference donnent les volumes attendus et des maillages fermes.
- Le servomoteur complet compile et produit des perçages reels, avec bounds verifies et aucun resultat invente.
- Une modification de jeu et un undo/redo mettent a jour la geometrie et l'export ; une erreur ne permet pas un export obsolete.
- Les trois apparences sont derivees de la source et visibles dans l'apercu.
- Le logiciel permet de charger le modele sans modifier son code ; un STL en millimetres est exportable.
- Tests unitaires et integration, gates Rust et verification native documentes, avec limites explicites.

## Perimetre

Tranche supplementaire DSL/CAD/UI autorisee par la demande du 2026-10-07, dans la continuite d'ADR-010. Pas de physique, PPO, IA, editeur de blocs, import de bibliotheques, STEP ou remplacement du viewport par un pipeline wgpu complet. Aucun changement de lane. Les changements preexistants sont preserves.

TODO : [todo_20261007_12-servo-cad.md](todo_20261007_12-servo-cad.md).

### user

### agent

## Implementation du 2026-10-07

Le code de la tranche est implemente. La cloture du ticket reste suspendue a
une verification interactive native manuelle ; aucun changement de lane ni
commit de completion n'a ete effectue.

- Correction confirmee du backend : la translation generale csgrs inversait
	certains plans. Polygon::translate conserve les plans. La conformite des
	aretes corrige les jonctions en T des surfaces decoupees.
- Exemple [servo](../../examples/servo/main.rgn) executable avec servo_main(jeu),
	servo(), boitier, pattes, bossages, axe, connecteur et ouvertures des captures.
	Le jeu ne dilate que le boitier. Resultat : 1437 sommets, 2870 triangles,
	environ 0.8 s en debug local.
- Constructeur color type, trois apparences conservees dans les booleens,
	projection jusqu'au rendu ; STL sans couleurs, coordonnees en millimetres.
- Menu Exemples -> Servomoteur et argument .rgn au lancement desktop.
	Compilation hors UI avec un worker et une demande en attente remplacable,
	progression par piece, annulation cooperative et rejet des revisions obsoletes.
	Toute edition reste dans SourceDocument et son historique ; export bloque
	pendant les calculs, les erreurs et apres annulation.
- Cadrage adapte aux bounds et CAD Z vertical. Triangles tries en profondeur
	puis dessines directement, sans contours a mitres. La premiere capture
	native a permis de detecter et corriger des traits parasites invisibles aux
	seuls tests de maillage.

## Preuves

- Tests CAD : volumes signes, fermeture orientee, translation et annulation.
- Tests DSL : couleurs entieres 0..255, erreurs d'unites, composants imbriques
	et instances independantes.
- Integration : bounds du servo, trois couleurs, fermeture, perçages ouverts
	dans l'epaisseur des pattes, jeu 0 -> 0.2 mm, IDs, undo/redo et STL.
- Tests projet/UI : revisions successives, annulation, historique, chargement
	asynchrone du servo colore, export et garde contre les sources invalides.
- `cargo fmt --check` : passe.
- `cargo clippy --workspace --all-targets -- -D warnings` : passe.
- `cargo test --workspace` : 69 tests passes, aucun echec.
- `cargo check -p robogen-desktop`, build desktop/CLI et check CLI du servo : passes.
- [Capture native](../../docs/screenshots/servo-native.png) 1440x831 et
	[capture compacte](../../docs/screenshots/servo-compact.png) 1080x680 inspectees.
	La seconde est une vraie taille de client, forcee temporairement via X11
	override-redirect car le WM ignorait les demandes ordinaires.

## Restant et limites

La session X11 n'a pas accepte les clics synthetiques. Le parcours natif
manuel Exemples -> modifier JEU -> orbiter -> annuler/retablir -> exporter
reste a confirmer ; les operations correspondantes sont testees en code.
La fenetre de test est fermee apres capture pour ne pas laisser son mode
override-redirect actif. Relancer normalement avec
`cargo run -p robogen-desktop -- examples/servo/main.rgn`.

Les cylindres restent facettes (32 segments), les appels booleens individuels
ne sont pas interruptibles, et l'apercu est une projection CPU via egui.
Pas de STEP, B-Rep analytique, physique, editeur a blocs ou imports. La forme
des captures n'est pas une certification constructeur. Le connecteur passe
sous la fixation avant : le controle du trou concerne la patte, pas un rayon
libre traversant toute la hauteur du moteur.

Des metadonnees Cargo perimees ont ete nettoyees uniquement pour les crates
DSL et domaine. Les diagnostics Serde restant dans l'editeur ne sont pas
reproduits par les gates Cargo. Rustfmt a egalement normalise le fichier du
parseur DSL preexistant, sans modification de son comportement.

### user

Modelise aussi la carte de la photo fournie (encombrement 104 x 90 x 37 mm).

### agent

## Iteration 2 - Carte embarquee

Ajout autorise au ticket actif. Plan : modele .rgn avec PCB, connecteurs
creux, module, dissipateur a ailettes, ventilateur statique et broches ; entree
dans Exemples ; tests de compilation, couleurs, encombrement et STL ; capture
native. Hypothese : les constructeurs existants suffisent sans changement de
backend. Premier discriminant : compilation CLI du modele complet.

Seul l'encombrement 104 x 90 x 37 mm est cote sur la photo. Implantation,
percages, connecteurs et pales sont approximatifs ; modele visuel non certifie,
sans identification certaine de la reference, ni simulation electronique ou
thermique. Les constantes du materiau DisplayOnly ne sont pas des proprietes
mesurees. Les corps sont separes, pas fusionnes en un solide imprimable unique.

### user

### agent

## Resultat de l'iteration 2

Modele ajoute dans [examples/compute_board/main.rgn](../../examples/compute_board/main.rgn),
accessible par Exemples -> Carte embarquee (104 x 90 mm). Sept parties colorees :
PCB principal, module, dissipateur a douze ailettes, ventilateur statique,
connecteurs creux, broches et petits composants. Encombrement controle de
104 x 90 x 37 mm ; details non cotes explicitement approximatifs.

La premiere validation de fermeture a expose un defaut de conversion precoce
en f32 pour les petits trous a environ 100 mm de l'origine. L'adaptateur CAD
conserve maintenant les coordonnees f64 du backend pour raccorder les aretes,
sans agrandir la tolerance de 1 nm, sans changer les DTO et sans relever les
budgets. Un test unitaire de PCB a quatre trous verifie volume et fermeture ;
la regression servomoteur reste verte.

Validations :

- compilation CLI : 3707 sommets, 7206 triangles ;
- integration : sept parties fermees, couleurs, STL en mm et hauteur 37 -> 42 mm ;
- UI : chargement, cadrage, couleurs, export et retour au modele precedent par undo ;
- `cargo fmt --check` et Clippy workspace avec `-D warnings` : passes ;
- `cargo test --workspace --quiet` : 72 tests passes ;
- build desktop/CLI : passe ;
- [capture native 1440x831](../../docs/screenshots/compute-board-native.png) inspectee,
  sans forcer la taille du client. RoboGen reste ouvert sur la carte.

Les approximations, les constantes physiques DisplayOnly et le STL multicoque
sont documentes dans le README de l'exemple. Pas de modele electronique,
thermique ou de ventilateur anime. Cette iteration est implementee ; la
verification interactive manuelle du servomoteur signalee en iteration 1 reste
ouverte. Aucun changement de lane ni commit de cloture du ticket global.

### user

### agent

Notification ulterieure : le terminal natif 12d5a22a s'est termine. La mention
precedente "RoboGen reste ouvert" ne decrit donc plus l'etat courant. La fin
du journal ne rapporte pas d'erreur fatale, seulement les avertissements de
presentation Vulkan deja observes. La cause de fermeture n'est pas etablie ;
le message shell sur un ancien chemin Snap ne prouve pas un crash de RoboGen.
Le modele et les preuves restent enregistres. Relancement possible depuis la
racine : `./target/debug/robogen-desktop examples/compute_board/main.rgn`.

### user

Corriger l'affichage : la carte presente des faces qui se masquent et des
triangles parasites, capture fournie.

### agent

Iteration 3 autorisee : remplacer la visibilite par tri de profondeur moyenne
par un tampon de profondeur par pixel dans l'apercu CPU existant. Hypothese :
les grands triangles et les faces croisees ne peuvent pas etre ordonnes par
leur centre. Test discriminant : deux triangles superposes dont l'ordre de
visibilite change selon le pixel, independent de l'ordre d'entree. Puis
raccorder l'image au viewport, conserver camera/grille/couleurs, mettre en
cache les vues immobiles et verifier la carte en natif. Aucun changement CAD.
La precedente validation visuelle etait insuffisante et ne prouvait pas la
correction des occultations.

### user

### agent

Iteration 3 implementee : tampon de profondeur inverse par pixel et clipping
proche dans robogen-render ; composition texture et cache par camera, mesh et
taille dans tous les apercus robogen-ui. Aucun changement de geometrie CAD.
Calcul barycentrique incremental pour limiter le cout par pixel. Resolution
bornee a 2048 par axe ; rendu CPU synchrone lors des changements de vue,
sans antialiasing ni mesure de cadence continue. ADR-001 et TECHNICAL alignes.

Verification :
- tests de triangles croises independants de l'ordre, clipping et faces invalides ;
- cache reutilise a vue constante et invalide par camera, couleur/mesh et taille ;
- integration carte : moyeu devant PCB a 600x400 et 320x240, image non vide ;
- fmt, Clippy workspace `-D warnings`, 76 tests workspace et build desktop passes ;
- aucun diagnostic editeur sur les trois fichiers Rust modifies ;
- [capture native corrigee 1440x831](../../docs/screenshots/compute-board-depth.png)
	inspectee apres rotation de camera XTest : ventilateur, ailettes et connecteurs
	devant le PCB, sans les triangles parasites precedents. Pas de taille forcee.

Contrairement aux essais precedents, les evenements XTest ont permis la rotation
native ici. Cela ne valide pas le parcours complet edition/export du servo,
qui reste ouvert. Aucun changement de lane ni commit de cloture global.

### user

Separer la partie droite en deux parties : Taxonomie (M1), contenant les
instances, et Bibliotheque (M2), contenant les fonctions generatrices, par
exemple servo pour generer des servomoteurs.

### agent

Distinction retenue pour la prochaine iteration : M2 designe ici les
definitions reutilisables, M1 les instances concretes du projet, et non les
jalons du backlog qui portent aussi ces noms.

Proposition : deux sections empilees et redimensionnables a droite, Taxonomie
en haut et Bibliotheque en bas, avec recherches et selections independantes.
La Bibliotheque expose les composants generateurs et leurs parametres ; la
Taxonomie expose les instances du document et leur lien vers la definition.
Exemple existant : `component servo(jeu: Length = JEU) -> Solid` appartient a
la Bibliotheque, `part Servo { ... body = servo(); }` a la Taxonomie. Plusieurs
instances peuvent utiliser le meme generateur avec des arguments distincts.
Les deux vues doivent deriver du document .rgn, pas d'un catalogue fictif.
Une future action d'instanciation devra passer par une commande annulable.

Constat local : `taxonomy_panel` est actuellement a gauche et melange les
deux titres ; `assistant_panel` occupe la droite. La distinction ne se reduit
donc pas a renommer les titres. Verification prevue : un generateur servo et
deux instances doivent produire une definition en Bibliotheque et deux entrees
distinctes en Taxonomie, avec navigation vers la meme definition.

Point a confirmer : les deux sections de droite remplacent-elles l'assistant
IA, ou faut-il conserver celui-ci ailleurs ? Aucune modification de code a ce
stade ; le workflow du depot demande `todo implement` pour lancer cette
nouvelle iteration apres clarification de la disposition.

### user

Modifier les settings pour avoir acces au micro.

### agent

Reglages utilisateur VS Code modifies hors depot : `dictation.enabled` et
`dictation.showButton` explicitement a true. JSON valide, cles booleennes
confirmees dans la version installee, aucun diagnostic sur le fichier.
Ces options etaient deja actives par defaut : cette modification ne prouve
pas la resolution d'un blocage de permission ou de peripherique. `pactl`
est absent ; l'etat muet du micro systeme n'a pas pu etre verifie. Aucun
enregistrement lance ni permission systeme contournee. Tester avec le bouton
micro de la zone de chat et accepter l'autorisation si elle est demandee.
La separation Taxonomie/Bibliotheque n'a pas ete implementee dans ce tour.

### user

Could not access the microphone for speech-to-text: Could not start audio source

### agent

Diagnostic sans enregistrement : PipeWire, pipewire-pulse et WirePlumber
actifs ; socket Pulse present. `wpctl` identifie SF-558 Mono comme source par
defaut, volume 1.00 non muet. `arecord -l` voit le micro USB avec un
sous-peripherique disponible. Aucun evenement dans les journaux audio des
20 dernieres minutes. VS Code renderer.log confirme a 19:28:42 un echec
`[chat-stt] microphone acquisition failed Could not start audio source`.
Cause precise non determinee ; pas de preuve d'un refus de permission.
Snap code revision 268 utilise le confinement classic.

Prochaine verification utilisateur : menu du micro de dictee, selection
explicite SF-558 Mono (commande interne
`workbench.action.chat.selectSpeechToTextMicrophone`), puis nouvel essai.
Si echec persistant, fermer completement VS Code apres sauvegarde puis le
relancer. Aucun service audio redemarre, aucun parametre supplementaire
modifie, aucune capture vocale effectuee pendant le diagnostic.

Le terminal natif RoboGen 9701e14e est termine ; ne pas reutiliser cet ID.
Le message de chemin Snap 241 absent ne prouve pas la cause de l'echec micro.

### user

L'erreur Could not start audio source persiste.

### agent

Incompatibilite de chargement reproduite sans capture : le Snap code 268
n'embarque pas libpulse. Le processus audio Electron charge libasound mais
pas libpulse et aucun client VS Code n'apparait dans PipeWire. Le paquet
systeme libpulse0 est installe et intact (dpkg -V sans ecart).
Le RPATH de code inclut /snap/core20/current/lib/x86_64-linux-gnu.
Test ELECTRON_RUN_AS_NODE avec process.dlopen de la libpulse systeme :
`/snap/core20/current/lib/x86_64-linux-gnu/libc.so.6: version GLIBC_2.34 not found
(required by /lib/x86_64-linux-gnu/libpulse.so.0)`.

Cette incompatibilite est une cause probable de l'echec de capture, et non
un simple reglage de permission. Recommandation : installer le paquet officiel
VS Code .deb, lancer /usr/bin/code apres fermeture du Snap et tester le micro.
Ne pas desinstaller le Snap ni effacer les preferences avant validation.
Aucune installation systeme ni capture audio effectuee ; correction complete
a confirmer apres changement de distribution de l'editeur.

### user

Precision sur la separation : a droite, en haut la Taxonomie avec les
instances ; en dessous une deuxieme section Bibliotheque avec les fonctions
qui servent a generer ces instances.

### agent

Disposition retenue : une colonne droite divisee verticalement en deux
sections, Taxonomie (M1, instances du projet) en haut, Bibliotheque (M2,
fonctions generatrices reutilisables) en bas. Par exemple servo_gauche et
servo_droit en haut referencent servo(jeu) en bas. Ce ne sont pas deux onglets
alternatifs : les deux sections restent visibles simultanement.

Pour preserver l'assistant IA, proposition de le deplacer dans la colonne
gauche liberee ; ce placement reste a confirmer. Aucun code modifie dans
ce tour de clarification. `todo implement` autorisera la preparation de la
checklist et l'implementation suivant le workflow du depot.

### user

Modeliser la camera du plan Raspberry Pi Camera module, Gert van Loo,
21 mai 2013. Confirmation recue via le dialogue : `todo implement`.

### agent

Iteration 4 autorisee : generateur raspberry_pi_camera et instance Camera,
exemple chargeable dans le menu, tests de geometrie/parametres/STL/UI et
capture native. Hypothese : les primitives existantes suffisent ; premier
discriminant = compilation CLI du modele, sans changement du moteur CAD.

Cotes retenues : PCB 23.9 x 25 x 0.95, trous diametre 2 aux centres
(9.35, 2), (21.85, 2), (9.35, 23), (21.85, 23), base optique 8 x 8 a
X=5.1, hauteur optique 5.2 au-dessus du PCB, connecteur sous PCB 2.8,
nappe largeur 16.2. Le deport 1.27 est interprete comme le plan median de
la nappe sous le PCB. Diametres optiques, repartition des hauteurs, empreinte
connecteur et longueur de nappe seront signales approximatifs. Pas de
simulation optique/electronique. La separation des panneaux est hors de
cette iteration et reste non implementee.

### user

### agent

Iteration 4 implementee : exemple examples/raspberry_pi_camera avec generateur
raspberry_pi_camera(hole_diameter), cinq sous-composants, instance Camera et
manifest projet. Entree Exemples -> Camera Raspberry Pi (2013), chargement
asynchrone existant, preset camera et historique conserves. Aucun changement
du backend CAD ni du renderer.

Preuves :
- CLI : 1773 sommets, 3558 triangles ; export /tmp/raspberry-pi-camera.stl reussi ;
- maillage ferme et oriente, six couleurs, quatre trous traversants aux centres
	du plan et rayons verifies pour diametres 2 et 3 mm ;
- bounds STL en mm : X=-5..23.9, Y=0..25, Z=-2.8..6.15 ;
- UI : compilation asynchrone, export et undo, objectif visible et image non
	vide a 600x400 et 320x240 ;
- fmt, Clippy workspace -D warnings, 78 tests workspace et build desktop passes ;
- aucun diagnostic editeur dans les deux fichiers Rust modifies ;
- [capture native 1440x831](../../docs/screenshots/camera-native.png) inspectee,
	sans taille forcee : les quatre trous, le PCB, l'objectif et la nappe sont
	visibles, sans defaut d'occultation observe.

Le README de l'exemple distingue les cotes des approximations : diametres et
etages optiques, empreinte connecteur, queue du capteur rectangulaire et nappe
droite de 5 mm. Materiau DisplayOnly non physique ; pas de simulation optique,
pas de certification constructeur. Documentation generale et TECHNICAL mises
a jour. La verification native prouve le rendu, pas le parcours manuel complet.
La separation Taxonomie/Bibliotheque et le parcours servo manuel restent ouverts ;
aucun changement de lane ni commit de cloture du ticket global.

### user
 
### agent

Iteration 5 : confirmation `todo implement` recue pour taxonomie generee par
le DSL en haut a droite, bibliotheque de fonctions en bas, assistant a gauche.
Les fonctions servomoteur, camera et Jetson Nano Super doivent ajouter des
instances au document, pas remplacer le projet par un exemple. Le modele de
carte reste approximatif et non certifie sous ce libelle fourni par l'utilisateur.
Taxonomie issue des parts/features compilees, avec signalement des resultats
obsoletes. Insertion via commande SourceDocument annulable, source autonome.
La carte a sept parts doit devenir un generateur multicorps : ajout explicite
compound au contrat ADR-010, sans assimilation a union ni changement de budgets
booleens. Tests et verifications natives avant declaration de completion.

### user

Correction : il s'agit de la partie gauche, pas droite. Garder le chat a sa
place initiale a droite ; remplacer la colonne gauche par Taxonomie au-dessus
de Bibliotheque.

### agent

Correction appliquee dans l'iteration 5 en cours : les deux sections sont
maintenant a gauche, assistant restaure a droite. Aucun changement du contrat
DSL ni de l'ordre vertical. Test de disposition mis a jour et passe a
1440x831 et 1080x680 ; les 17 tests UI, fmt et Clippy passent. Verification
native en cours sur la version reconstruite.

### user

### agent

Iteration 5 terminee avec la disposition corrigee : Taxonomie en haut a
gauche, Bibliotheque en dessous, chat conserve a droite. Les sections ont
deux recherches et defilements independants, un separateur ajustable et une
largeur redimensionnable. La taxonomie derive des parts/features compilees,
avec IDs et provenance, pas d'un catalogue fictif. Resultats precedents
signales pendant un calcul ou une erreur ; insertion/export alors bloques.

Les fonctions servo, raspberry_pi_camera et jetson_nano_super s'inserent
dans le document via une commande annulable. Les definitions et aides sont
copiees une fois avec prefixes distincts, les collisions sont diagnostiquees,
les instances recoivent des noms uniques et une position hors des bounds X
existants. Les definitions du document sont reutilisees, avec le materiau
d'une instance existante. Arguments et placement sont editables en DSL.
La source generee compile independamment de l'UI. Liens et double-clic ouvrent
les declarations source. L'ancienne injection arbitraire set_taxonomy est
supprimee ; aucun type tiers ne traverse les nouvelles projections.

Compound est explicite, borne et annulable, sans fusion implicite ; les
operandes booleens multicorps sont refuses. Le generateur de carte conserve
exactement les positions des sommets et le nombre de triangles du modele
precedent. Le libelle Jetson Nano Super est celui demande, pas une certification
de reference constructeur ; la reserve reste visible et documentee.

Preuves :
- 85 tests workspace passes ; fmt, Clippy -D warnings, builds CLI et desktop passes ;
- apres correction gauche/droite : 17 tests UI, fmt, Clippy et build desktop repasses ;
- tests de disposition 1440x831 et 1080x680, conflits sans mutation, materiaux,
	instances multiples, source autonome, undo/redo, selection et etats invalides ;
- aucun diagnostic editeur sur les fichiers Rust modifies ;
- trois clics XTest natifs sur les boutons + : support conserve, servo_1,
	raspberry_pi_camera_1 et jetson_nano_super_1 ajoutes et visibles en 3D ;
- [capture finale 1440x680](../../docs/screenshots/taxonomy-library-left-native.png)
	inspectee : sections a gauche, chat a droite, quatre instances compilees.

README, documentation DSL, ADR-010 et TECHNICAL actualises. Aucun changement
de lane ni commit de cloture global : le parcours manuel servo initial reste
ouvert. Le test natif de cette iteration couvre les ajouts, pas un parcours
manuel complet d'edition/export. La fenetre de verification de l'ancienne
disposition a ete fermee ; la version corrigee a ete lancee ensuite.

### user

Demande si RoboGen optimise la topologie comme Fusion 360 : definir des
volumes grossiers puis reduire matiere et temps d'impression en conservant
la solidite et l'imprimabilite. Imprimante actuelle : FlashForge Finder.

### agent

C'est un objectif prevu, pas une fonction actuellement operationnelle.
Verification : robogen-topopt ne contient que le contrat et
UnavailableTopologyOptimizer, qui retourne Unavailable. ADR-008 prevoit au
jalon M7 FEM elastique lineaire statique et SIMP, objectif compliance/volume.
La reduction de compliance vise la rigidite ; elle ne certifie ni resistance
a la rupture, ni fatigue, ni tenue des couches d'une piece imprimee.

Parcours vise : enveloppe de conception, zones preservees (fixations et
interfaces), volumes interdits, charges/appuis, materiau et limites de
deformation, puis optimisation et revalidation du maillage reconstruit.
Pour la Finder, ajouter un profil de fabrication correspondant a sa version,
au filament, a la buse, aux couches, aux parois et a l'orientation. Les
surplombs, supports et epaisseurs minimales doivent etre des contraintes,
pas une garantie implicite du resultat topologique. Moins de volume ne
garantit pas moins de temps : comparer les parcours dans le trancheur.

Les constantes DisplayOnly des exemples actuels ne conviennent pas a une
validation mecanique. Une calibration sur eprouvettes imprimees et des essais
de la piece sous charge sont necessaires avant d'affirmer sa solidite.
Premiere cible proposee : support de camera ou de servomoteur, compare a une
piece de reference avec le meme profil d'impression. Preciser la version de
Finder, le filament utilise et la premiere piece/les efforts a supporter.
Aucune implementation FEM/SIMP lancee, aucune lane ni TODO M7 modifiee.

### user

Capture des reglages du trancheur pour la Finder.

### agent

Parametres observes : machine Finder, buse 0.4 mm, materiau Flashforge-PLA,
filament 1.75 mm, profil Standard, temperature extrudeuse configuree 220 C,
controle de temperature coche et commande Extrudeur Droit. Ce sont les
reglages affiches, pas une mesure de temperature ni une recommandation de
temperature pour tous les PLA. La generation exacte de Finder n'est pas
identifiable avec cette capture seule.

Pour le futur profil de fabrication, demander hauteur de couche, nombre de
parois/largeur de ligne, remplissage (motif et pourcentage), puis orientation
et charges de la piece cible. Aucune resistance mecanique ni epaisseur
structurelle minimale ne peut etre deduite du seul diametre de buse. Ne pas
assimiler le remplissage du trancheur a une densite SIMP sans calibration.
Parametres notes pour la conception, aucun profil executable ni solveur
d'optimisation ajoute a ce stade.

### user

Creer un ticket pour cette tache, la laisser de cote pour le moment et rendre
tout cela parametrable.

### agent

Ticket cree en attente :
[Optimisation topologique et profils d'impression parametrables](task_20261007_13-topopt-print-profile.md),
avec sa [checklist](todo_20261007_13-topopt-print-profile.md).
Le perimetre comprend les profils machine/materiau/fabrication, les charges,
les zones preservees/exclues et les reglages du solveur. Les valeurs de la
Finder deviennent un prereglage modifiable, jamais des constantes imposees.
Les champs inconnus restent explicites et seront precises a la reprise.
Aucun code modifie, aucune implementation lancee et aucune lane existante
deplacee. Reprise conditionnee aux prerequisites M7 et a une autorisation.

### user

Modeliser un pack Li-ion 1S OEM jaune marque 3.7 V / 3000 mAh / 15C,
enveloppe nominale 18 x 68 mm, deux fils et connecteur rouge deux broches.
La cellule nue 18.3 x 64.9 mm est une reference distincte, pas un composant
interne dimensionne pour ce pack. Sources fournies : eBay 298604746866,
comparaison VICMILE via FindThisBest et cellule Samsung via MAXPOWER.
Autorisation confirmee par selection explicite : todo implement.

### agent

Iteration 6 : modele parametrique, tests geometriques et export, insertion
dans la bibliotheque et menu Exemples, documentation et verification native.
Hypothese locale : cylinder/translate/color/compound et difference existants
suffisent ; tester compilation et bornes avant integration UI.
Fils droits de 20 mm et connecteur 6 x 4 x 10 mm sont des estimations editables.
Ni identification VICMILE/Samsung, ni masse, ni tenue electrique certifiees.
Pas de texture de marquage imprime dans le maillage actuel.

### user

### agent

Iteration 6 realisee : examples/li_ion_battery, pack nominal jaune 18 x 68 mm,
deux fils et connecteur rouge estimes, huit parametres de longueur. Ensemble
par defaut 18 x 18 x 98 mm, 370 sommets et 716 triangles. Ajout a la
bibliotheque via LibraryGenerator::Battery et au menu Exemples, insertion
annulable. Documentation des sources fournies et approximations dans le
README exemple, README principal et TECHNICAL. Marquage imprime non reproduit.

Validation : 88 tests workspace, fmt --check, Clippy all-targets -D warnings,
builds desktop/CLI, controle CLI et export /tmp/li-ion-battery.stl passes.
Tests de dimensions/variations, couleurs, fermeture, STL, insertion/undo et
visibilite passes. Capture docs/screenshots/battery-native.png inspectee :
client natif 1440x831, pack jaune, fils et connecteur visibles. La fenetre de
verification s'est terminee apres la capture. Aucun commit ni changement de
lane ; le parcours manuel servo de l'iteration 1 reste ouvert.

### user

Le DSL doit pouvoir etre projete non seulement en vue 3D et code source,
mais aussi sous forme de schema mecanique ou electronique.

### agent

Direction proposee, sans implementation lancee : un meme modele semantique
alimente des vues synchronisees, pas des documents independants a maintenir.
Verification locale : SemanticModel contient parametres, materiaux, sketches
et parts ; il ne decrit pas encore les broches et reseaux electriques.

Distinguer trois projections : plan mecanique (vues orthogonales, coupes,
cotes et annotations), schema cinematique (corps, liaisons, axes et mouvements)
et schema electrique (symboles, broches, connecteurs et reseaux). La geometrie
peut alimenter un plan mais ne suffit ni a definir des liaisons ni a deduire
un cablage. Tolerances et cotes fonctionnelles doivent etre explicites.

Faire evoluer les composants vers des definitions avec representations CAD,
interfaces mecaniques et ports electriques optionnels, lies par les memes
identifiants. Une batterie aurait son enveloppe 3D et un symbole a deux bornes ;
les connexions et proprietes electriques doivent etre declarees, pas devinees
depuis les cylindres colores actuels. Meme principe pour servo/camera/carte,
sans inventer leurs brochages ni compatibilites.

Les mutations de domaine depuis chaque vue deviennent des commandes annulables
qui mettent a jour la source et invalident les projections. Le placement des
symboles, parcours graphiques des fils et annotations de feuille relevent
d'une mise en page persistante distincte de la connectivite. Selection croisee
et diagnostics pointent les memes entites et leur source. Le schema electrique
n'implique pas automatiquement simulation de circuit ni conception de PCB.

Proposition d'ordre : premiere tranche plan mecanique 2D du modele existant,
puis tranche schema electrique avec bornes/reseaux explicitement declares ;
schema cinematique sur les contrats d'assemblage/robotique. Une ADR et un plan
dedies seront necessaires avant modification des frontieres ou schemas.
Discussion seulement : aucun changement du DSL, aucun nouvel onglet factice,
aucune extension du perimetre TopOpt differe.

### user