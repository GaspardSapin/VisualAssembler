# 68000 assembler directives, French (m2-assembler.md §4.5–§4.8, §7.1).
#
# Same ids and attributes as locales/en/directives.ftl. Written in our own words, in natural
# French rather than a word-for-word translation, with the vocabulary of isa.ftl (étiquette,
# compteur d’emplacement, octet, mot, mot long, bloc conditionnel, éditeur de liens).
# Style: no braces in the text, no continuation line starting with a dot, a star or a bracket,
# typographic apostrophes (’) only; quoted strings are described, not shown.

isa-m68k-dir-equals = Donne une valeur constante à l’étiquette, comme EQU.
    .details = Syntaxe : nom = expression, le nom en colonne 1, par exemple taille = 64. C’est
        l’équivalent d’EQU : la valeur est fixée une fois pour toutes, et définir le nom une
        seconde fois est une erreur. L’expression peut utiliser des symboles définis plus bas
        dans le fichier. Comme une étiquette ordinaire, la ligne ouvre un nouveau bloc
        d’étiquettes locales.

isa-m68k-dir-bss = Ouvre une section de données mises à zéro ; refusée, comme par a68k -s.
    .details = Dans les fichiers objets destinés à un éditeur de liens, BSS ouvre une section de
        mémoire réservée mais non stockée. VisualAssembler produit ses programmes sous forme de
        S-records, comme a68k avec son option -s, et a68k refuse BSS dans ce mode : la ligne est
        donc refusée avec une erreur. Réservez la mémoire avec DS, à une adresse choisie avec ORG.

isa-m68k-dir-cnop = Complète par des octets nuls jusqu’à une adresse de reste choisi.
    .details = Syntaxe : CNOP décalage,alignement, par exemple CNOP 0,4 pour atteindre le multiple
        de 4 suivant. Les deux valeurs doivent être connues à cette ligne : l’alignement est une
        puissance de deux à partir de 2, et le décalage va de 0 à l’alignement moins 1. Des
        octets nuls sont ajoutés jusqu’à ce que le compteur d’emplacement, divisé par
        l’alignement, donne le décalage pour reste. Rien n’est ajouté si le compteur convient
        déjà.

isa-m68k-dir-code = Ouvre une section de code ; refusée, comme par a68k -s.
    .details = Dans les fichiers objets destinés à un éditeur de liens, CODE ouvre une section
        d’instructions. VisualAssembler produit ses programmes sous forme de S-records, comme
        a68k avec son option -s, et a68k refuse CODE dans ce mode : la ligne est donc refusée
        avec une erreur. Placez le code à une adresse choisie avec ORG.

isa-m68k-dir-cseg = Autre nom de CODE ; refusée, comme par a68k -s.
    .details = CSEG ouvre une section de code dans les fichiers objets destinés à un éditeur de
        liens, exactement comme CODE. A68k la refuse quand il produit des S-records, et
        VisualAssembler, qui produit toujours des S-records, refuse la ligne avec une erreur.
        Placez le code à une adresse choisie avec ORG.

isa-m68k-dir-data = Ouvre une section de données ; refusée, comme par a68k -s.
    .details = Dans les fichiers objets destinés à un éditeur de liens, DATA ouvre une section de
        constantes et de variables. VisualAssembler produit ses programmes sous forme de
        S-records, comme a68k avec son option -s, et a68k refuse DATA dans ce mode : la ligne est
        donc refusée avec une erreur. Placez les données à une adresse choisie avec ORG, puis
        écrivez-les avec DC ou réservez-les avec DS.

isa-m68k-dir-dc = Range des constantes : octets, mots ou mots longs.
    .details = Syntaxe : DC.B, DC.W ou DC.L suivi de valeurs séparées par des virgules, sans
        espace ; sans suffixe, la taille est le mot. Chaque valeur est une expression ; en DC.B,
        une valeur formée d’une seule chaîne entre guillemets range ses caractères en octets.
        DC.W et DC.L alignent d’abord le compteur d’emplacement sur une adresse paire. Un octet
        accepte de -128 à 255 et un mot de -32768 à 65535 ; une valeur hors de ces bornes est
        une erreur. Un espace termine le champ des opérandes : DC.B 1, 2 ne range donc que le 1.

isa-m68k-dir-dcb = Range une même valeur un nombre donné de fois.
    .details = Syntaxe : DCB.B, DCB.W ou DCB.L nombre,valeur ; sans suffixe, la taille est le mot.
        Le nombre doit être connu à cette ligne et valoir 0 ou plus. DCB.W et DCB.L alignent
        d’abord sur une adresse paire. Par exemple, DCB.W 4,-1 range quatre mots valant $FFFF.

isa-m68k-dir-ds = Réserve de la mémoire remplie de zéros.
    .details = Syntaxe : DS.B, DS.W ou DS.L nombre ; sans suffixe, la taille est le mot. La
        directive réserve autant d’octets, de mots ou de mots longs, que VisualAssembler remplit
        de zéros dans le programme et dans le S-record, comme a68k. Le nombre doit être connu à
        cette ligne et valoir 0 ou plus. DS.W et DS.L alignent d’abord sur une adresse paire :
        DS.W 0 sert donc uniquement à aligner.

isa-m68k-dir-dseg = Autre nom de DATA ; refusée, comme par a68k -s.
    .details = DSEG ouvre une section de données dans les fichiers objets destinés à un éditeur
        de liens, exactement comme DATA. A68k la refuse quand il produit des S-records, et
        VisualAssembler, qui produit toujours des S-records, refuse la ligne avec une erreur.
        Placez les données à une adresse choisie avec ORG.

isa-m68k-dir-end = Termine le programme, éventuellement avec son adresse de départ.
    .details = Syntaxe : END, ou END expression pour donner le point d’entrée, que le dernier
        enregistrement du S-record contient. L’assemblage s’arrête à cette ligne : les lignes
        suivantes sont ignorées, dans ce fichier comme dans les fichiers qui l’ont inclus, et un
        texte après END autre qu’un commentaire donne un avertissement. END est facultatif ;
        sans lui, le programme se termine à sa dernière ligne et n’a pas de point d’entrée.

isa-m68k-dir-endc = Ferme un bloc conditionnel ouvert par une directive IF.
    .details = Chaque IFEQ, IFNE, IFGT, IFGE, IFLT, IFLE, IFD, IFND, IFC ou IFNC demande son ENDC.
        Les blocs peuvent s’imbriquer, et chaque ENDC ferme le bloc ouvert le plus intérieur. Un
        ENDC sans bloc ouvert, ou un bloc encore ouvert à la fin du programme, est une erreur.
        A68k 2.71 ne connaît pas ELSE : pour choisir entre deux cas, écrivez un second bloc avec le
        test contraire.

isa-m68k-dir-endif = Autre nom d’ENDC : ferme un bloc conditionnel.
    .details = ENDIF et ENDC sont la même directive, et chacune ferme le bloc le plus intérieur
        ouvert par une directive IF. Les blocs peuvent s’imbriquer. Un ENDIF sans bloc ouvert
        est une erreur.

isa-m68k-dir-endm = Termine le corps d’une macro.
    .details = Les lignes entre nom MACRO et ENDM forment le corps de la macro ; elles ne sont
        assemblées qu’aux endroits où la macro est appelée. Un ENDM hors d’une macro, ou une
        macro sans ENDM, est une erreur. Pour quitter un développement avant la fin, utilisez
        MEXIT.

isa-m68k-dir-equ = Donne une valeur constante à l’étiquette.
    .details = Syntaxe : nom EQU expression, le nom en colonne 1, par exemple taille EQU 64. La
        valeur est fixée une fois pour toutes : définir le nom une seconde fois est une erreur,
        alors qu’un nom défini par SET peut changer. L’expression peut utiliser des symboles
        définis plus bas dans le fichier. Les symboles distinguent majuscules et minuscules :
        Taille et TAILLE sont deux noms différents. Comme une étiquette ordinaire, EQU ouvre un
        nouveau bloc d’étiquettes locales.

isa-m68k-dir-equr = Donne un nom à un registre ; non prise en charge par VisualAssembler.
    .details = Dans a68k, nom EQUR D3 permet d’écrire nom à la place de D3. VisualAssembler ne
        prend pas en charge les noms de registres et refuse la ligne avec une erreur, car il
        reconnaît les registres ligne par ligne, sans lire le reste du programme. Écrivez le
        registre lui-même, avec un commentaire qui dit ce qu’il contient.

isa-m68k-dir-even = Passe à l’adresse paire suivante.
    .details = Si le compteur d’emplacement est impair, EVEN ajoute un octet nul ; sinon, elle ne
        fait rien. Les instructions, ainsi que DC, DS et DCB en taille mot ou mot long, sont
        alignées automatiquement : EVEN sert surtout avant une étiquette seule sur sa ligne qui
        suit des octets. Le 68000 ne lit et n’écrit des mots et des mots longs qu’aux adresses
        paires.

isa-m68k-dir-far = Directive de modèle d’adressage d’a68k ; non prise en charge.
    .details = Dans a68k, NEAR et FAR activent et désactivent un modèle où les données sont
        atteintes par rapport à un registre d’adresse. VisualAssembler n’a pas ce modèle et
        refuse la ligne avec une erreur. Supprimez la ligne : VisualAssembler choisit lui-même
        un adressage absolu ou relatif au PC pour chaque opérande.

isa-m68k-dir-idnt = Nomme le module du programme ; acceptée sans effet.
    .details = Syntaxe : IDNT nom. Dans les fichiers objets, ce nom identifie le module pour un
        éditeur de liens. VisualAssembler accepte la ligne et l’ignore : le S-record qu’il écrit
        n’a pas de nom de module, comme avec a68k -s.

isa-m68k-dir-ifc = Assemble un bloc si deux chaînes sont égales.
    .details = Syntaxe : IFC suivi de deux chaînes entre guillemets séparées par une virgule,
        puis le bloc, puis ENDC. Les chaînes sont comparées octet par octet : majuscules et
        minuscules diffèrent donc. On l’utilise surtout dans les macros, par exemple pour savoir
        si l’argument \1 est vide en le comparant à une chaîne vide. Le test est fait une seule
        fois, à la lecture de la ligne ; les lignes d’un bloc faux sont sautées et ne donnent
        aucune erreur.

isa-m68k-dir-ifd = Assemble un bloc si un symbole est défini plus haut.
    .details = Syntaxe : IFD nom, puis le bloc, puis ENDC. Le bloc est assemblé quand le nom est
        défini sur une ligne précédente, comme étiquette ou par EQU, SET ou =, et sauté sinon.
        Une définition plus bas ne compte pas, car le test est fait une seule fois, à la lecture
        de la ligne. IFND est le test contraire.

isa-m68k-dir-ifeq = Assemble un bloc si une valeur est nulle.
    .details = Syntaxe : IFEQ expression, puis le bloc, puis ENDC. L’expression doit être connue
        à cette ligne : un symbole défini plus bas est refusé. Les lignes d’un bloc faux sont
        sautées et ne donnent aucune erreur : une constante comme DEBUG EQU 0 permet ainsi
        d’activer ou de désactiver des parties d’un programme. Les blocs peuvent s’imbriquer.

isa-m68k-dir-ifge = Assemble un bloc si une valeur est positive ou nulle.
    .details = Syntaxe : IFGE expression, puis le bloc, puis ENDC. L’expression doit être connue à
        cette ligne, et sa valeur est comparée à zéro. Les lignes d’un bloc faux sont sautées et
        ne donnent aucune erreur. IFLT est le test contraire.

isa-m68k-dir-ifgt = Assemble un bloc si une valeur est strictement positive.
    .details = Syntaxe : IFGT expression, puis le bloc, puis ENDC. L’expression doit être connue à
        cette ligne, et sa valeur est comparée à zéro. Les lignes d’un bloc faux sont sautées et
        ne donnent aucune erreur. IFLE est le test contraire.

isa-m68k-dir-ifle = Assemble un bloc si une valeur est négative ou nulle.
    .details = Syntaxe : IFLE expression, puis le bloc, puis ENDC. L’expression doit être connue à
        cette ligne, et sa valeur est comparée à zéro. Les lignes d’un bloc faux sont sautées et
        ne donnent aucune erreur. IFGT est le test contraire.

isa-m68k-dir-iflt = Assemble un bloc si une valeur est strictement négative.
    .details = Syntaxe : IFLT expression, puis le bloc, puis ENDC. L’expression doit être connue à
        cette ligne, et sa valeur est comparée à zéro. Les lignes d’un bloc faux sont sautées et
        ne donnent aucune erreur. IFGE est le test contraire.

isa-m68k-dir-ifnc = Assemble un bloc si deux chaînes diffèrent.
    .details = Syntaxe : IFNC suivi de deux chaînes entre guillemets séparées par une virgule,
        puis le bloc, puis ENDC. Les chaînes sont comparées octet par octet : majuscules et
        minuscules diffèrent donc. Dans une macro, IFNC permet de vérifier qu’un argument a été
        donné, en le comparant à une chaîne vide. IFC est le test contraire.

isa-m68k-dir-ifnd = Assemble un bloc si un symbole n’est pas défini plus haut.
    .details = Syntaxe : IFND nom, puis le bloc, puis ENDC. Le bloc est assemblé quand le nom
        n’est défini sur aucune ligne précédente ; une définition plus bas ne compte pas. Elle
        peut protéger un fichier de définitions : IFND suivi d’un nom, puis ce nom EQU 1 et les
        définitions, puis ENDC, pour qu’un fichier inclus deux fois ne définisse ses symboles
        qu’une fois. IFD est le test contraire.

isa-m68k-dir-ifne = Assemble un bloc si une valeur n’est pas nulle.
    .details = Syntaxe : IFNE expression, puis le bloc, puis ENDC. L’expression doit être connue à
        cette ligne, et sa valeur est comparée à zéro. Les lignes d’un bloc faux sont sautées et
        ne donnent aucune erreur. IFEQ est le test contraire.

isa-m68k-dir-incbin = Insère à cet endroit les octets d’un fichier.
    .details = Syntaxe : INCBIN chemin, le chemin écrit tel quel ou entre guillemets. Les octets
        du fichier sont copiés au compteur d’emplacement, sans alignement ; une instruction
        placée après eux est alignée automatiquement. Le chemin est cherché comme pour INCLUDE,
        et les mêmes chemins sont refusés. Le fichier peut contenir au plus 16 Mio.

isa-m68k-dir-include = Assemble à cet endroit les lignes d’un autre fichier source.
    .details = Syntaxe : INCLUDE chemin, le chemin écrit tel quel ou entre guillemets. Le chemin
        est cherché dans le dossier du fichier principal, puis dans chaque dossier d’inclusion ;
        a68k cherche plutôt dans son dossier de travail, alors gardez ensemble les fichiers d’un
        programme. Par sécurité, un chemin absolu, un lecteur, un partage réseau, un nom de
        périphérique ou un chemin qui sort de son dossier est refusé. Un fichier inclus peut en
        inclure d’autres, jusqu’à 16 niveaux, et un fichier qui s’inclut lui-même est une
        erreur. Un problème dans un fichier inclus est signalé dans ce fichier, avec une note
        sur la ligne INCLUDE.

isa-m68k-dir-list = Rétablit le listing ; acceptée sans effet.
    .details = Dans a68k, LIST rétablit le listing après NOLIST. VisualAssembler accepte la ligne
        et l’ignore, car son listing montre toujours toutes les lignes.

isa-m68k-dir-macro = Commence la définition d’une macro.
    .details = Syntaxe : nom MACRO, le nom en colonne 1, puis les lignes du corps, puis ENDM. Un
        appel écrit le nom à la place d’une opération, suivi d’arguments séparés par des
        virgules : dans le corps, \1 à \9 les représentent, \0 le suffixe de taille de l’appel
        (W s’il n’y en a pas) et \@ un numéro propre à chaque appel, qui rend les étiquettes
        uniques. NARG donne le nombre d’arguments, et un argument contenant des espaces ou des
        virgules s’écrit entre < et >. Les noms de macro distinguent majuscules et minuscules,
        doivent être définis avant leur premier appel et ne peuvent pas être le nom d’une
        instruction ou d’une directive.

isa-m68k-dir-mexit = Quitte le développement en cours d’une macro.
    .details = MEXIT arrête le développement de la macro appelée : le reste de son corps est sauté
        pour cet appel. On la place en général dans un bloc conditionnel, pour terminer la macro
        plus tôt selon un argument. Un MEXIT hors d’une macro est une erreur.

isa-m68k-dir-near = Directive de modèle d’adressage d’a68k ; non prise en charge.
    .details = Dans a68k, NEAR et FAR activent et désactivent un modèle où les données sont
        atteintes par rapport à un registre d’adresse. VisualAssembler n’a pas ce modèle et
        refuse la ligne avec une erreur. Supprimez la ligne : VisualAssembler choisit lui-même
        un adressage absolu ou relatif au PC pour chaque opérande.

isa-m68k-dir-nol = Autre nom de NOLIST ; acceptée sans effet.
    .details = Dans a68k, NOL suspend le listing jusqu’à LIST, exactement comme NOLIST.
        VisualAssembler accepte la ligne et l’ignore, car son listing montre toujours toutes les
        lignes.

isa-m68k-dir-nolist = Suspend le listing ; acceptée sans effet.
    .details = Dans a68k, NOLIST suspend le listing jusqu’à LIST. VisualAssembler accepte la
        ligne et l’ignore, car son listing montre toujours toutes les lignes.

isa-m68k-dir-org = Fixe l’adresse du code et des données qui suivent.
    .details = Syntaxe : ORG adresse, par exemple ORG $1000. L’adresse doit être connue à cette
        ligne et rester inférieure à $1000000, la fin de l’espace d’adressage du 68000 ; sans
        ORG, le programme commence à l’adresse 0. Un ORG qui laisse un trou ouvre un nouveau
        segment du programme. Placer des octets sur des octets déjà placés, par exemple avec un
        ORG qui revient sur une adresse utilisée, est une erreur.

isa-m68k-dir-page = Commence une nouvelle page d’un listing imprimé ; acceptée sans effet.
    .details = VisualAssembler accepte PAGE et l’ignore, car son listing est un texte simple, sans
        pages. Elle ne change rien au programme.

isa-m68k-dir-public = Autre nom de XDEF ; refusée, comme par a68k -s.
    .details = PUBLIC rend des symboles visibles pour les autres fichiers liés avec celui-ci,
        exactement comme XDEF. A68k la refuse quand il produit des S-records, et VisualAssembler, qui
        produit toujours des S-records, refuse la ligne avec une erreur. Supprimez la ligne :
        chaque symbole est déjà visible dans tout le programme, fichiers inclus compris.

isa-m68k-dir-reg = Donne un nom à une liste de registres ; non prise en charge par VisualAssembler.
    .details = Dans a68k, nom REG D0-D3/A0 permet à MOVEM d’utiliser nom à la place de la liste.
        VisualAssembler ne prend pas en charge les noms de registres et refuse la ligne avec une
        erreur, car il reconnaît les registres ligne par ligne, sans lire le reste du programme.
        Écrivez la liste elle-même dans chaque MOVEM.

isa-m68k-dir-rorg = Fixe une origine relogeable ; non prise en charge par VisualAssembler.
    .details = Dans a68k, RORG place le compteur d’emplacement par rapport au début d’une section
        relogeable, dans les programmes destinés à un éditeur de liens. VisualAssembler ne la
        prend pas en charge et refuse la ligne avec une erreur. Placez le code à une adresse
        avec ORG.

isa-m68k-dir-section = Ouvre une section nommée ; refusée, comme par a68k -s.
    .details = Dans les fichiers objets destinés à un éditeur de liens, SECTION ouvre une section
        nommée de code ou de données. VisualAssembler produit ses programmes sous forme de
        S-records, comme a68k avec son option -s, et a68k refuse SECTION dans ce mode : la ligne
        est donc refusée avec une erreur. Placez chaque partie du programme à une adresse
        choisie avec ORG.

isa-m68k-dir-set = Donne à l’étiquette une valeur qui peut changer ensuite.
    .details = Syntaxe : nom SET expression, le nom en colonne 1. Contrairement à EQU, le nom peut
        recevoir une nouvelle valeur par SET, et chaque utilisation voit la dernière valeur
        donnée au-dessus d’elle. L’expression doit être connue à cette ligne. Un nom défini par
        SET ne peut pas être aussi défini par EQU ou comme étiquette. Comme une étiquette
        ordinaire, SET ouvre un nouveau bloc d’étiquettes locales.

isa-m68k-dir-spc = Ajoute des lignes vides à un listing imprimé ; acceptée sans effet.
    .details = Syntaxe : SPC nombre. VisualAssembler accepte la ligne et l’ignore, car son
        listing n’a pas de mise en page. Elle ne change rien au programme.

isa-m68k-dir-title = Fixe le titre d’un listing imprimé ; acceptée sans effet.
    .details = Syntaxe : TITLE texte. VisualAssembler accepte la ligne et l’ignore : son listing
        commence par sa propre ligne d’en-tête. Elle ne change rien au programme.

isa-m68k-dir-ttl = Autre nom de TITLE ; acceptée sans effet.
    .details = Syntaxe : TTL texte, exactement comme TITLE. VisualAssembler accepte la ligne et
        l’ignore : son listing commence par sa propre ligne d’en-tête.

isa-m68k-dir-xdef = Rend des symboles visibles pour les autres fichiers ; refusée, comme par a68k -s.
    .details = Dans les fichiers objets, XDEF rend des symboles de ce fichier visibles pour les
        autres fichiers liés avec lui. Un programme produit sous forme de S-record forme un seul bloc,
        sans éditeur de liens, et a68k refuse XDEF quand il produit des S-records : la ligne est
        donc refusée avec une erreur. Supprimez la ligne : chaque symbole est déjà visible dans
        tout le programme, fichiers inclus compris.

isa-m68k-dir-xref = Utilise des symboles d’un autre fichier ; refusée, comme par a68k -s.
    .details = Dans les fichiers objets, XREF nomme des symboles qu’un éditeur de liens prend
        dans un autre fichier. Un programme produit sous forme de S-record forme un seul bloc,
        sans éditeur de liens, et a68k refuse XREF quand il produit des S-records : la ligne est
        donc refusée avec une erreur. Apportez plutôt les définitions dans le programme, par
        exemple avec INCLUDE.
