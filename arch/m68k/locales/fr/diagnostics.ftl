# Diagnostics of the 68000 lexer and assembler, French (m1-core-isa-lexer.md §5.4, §5.7,
# §6.1; m2-assembler.md §7).
#
# Same ids, attributes and variables as locales/en/diagnostics.ftl: $text and $cause for the
# lexer, and for the assembler the variables listed in arch/m68k/src/asm/codes.rs.
# Written in our own words, in natural French; typographic quotes and apostrophes (’) only;
# a regular space before : ; ? and inside « ».

## Lexer (m1-core-isa-lexer.md §5.4, §5.7)

diag-lex-bad-label = « { $text } » n’est pas une étiquette valide
    .help = Une étiquette commence par une lettre, « _ » ou « . », suivie de lettres, de chiffres, de « _ », de « . » ou de « @ ». Une étiquette locale s’écrit avec des chiffres suivis de « $ » (comme 1$) ou avec une barre oblique inverse suivie d’un nom.

diag-lex-label-junk = texte inattendu « { $text } » après l’étiquette
    .help = Séparez l’étiquette de l’instruction par un espace ou une tabulation, par exemple « boucle: nop ».

diag-lex-bad-operation = « { $text } » n’est ni une instruction, ni une directive, ni un nom de macro
    .help = Le champ d’opération contient un nom comme MOVE, DC ou un nom de macro, éventuellement suivi d’une taille comme .W.

diag-lex-bad-size-suffix = « { $text } » n’est pas un suffixe de taille valide
    .help = Utilisez .B (octet), .W (mot, 16 bits), .L (mot long, 32 bits) ou .S (branchement court).

diag-lex-bad-number = « { $text } » n’est pas un nombre valide
    .help = Écrivez 255 en décimal, $FF en hexadécimal, %11111111 en binaire ou @377 en octal ; les notations 0xFF et 0FFh ne sont pas acceptées.

diag-lex-unterminated-string = la chaîne « { $text } » n’est pas fermée
    .help = Terminez la chaîne par le guillemet qui l’ouvre. Pour placer ce guillemet dans la chaîne, doublez-le.

diag-lex-bad-backslash = « { $text } » n’est pas une séquence valide avec une barre oblique inverse
    .help = Hors des chaînes et des commentaires, la barre oblique inverse introduit un paramètre de macro (\0 à \9, ou \@) ou une étiquette locale (\nom).

diag-lex-unexpected-char = caractère inattendu « { $text } »
    .help = Ce caractère n’a pas de sens ici : supprimez-le, ou placez-le dans un commentaire après « ; ».

diag-lex-non-ascii = caractère non ASCII « { $text } » hors d’un commentaire ou d’une chaîne
    .help = Les lettres accentuées et les autres caractères non ASCII ne sont acceptés que dans les commentaires et les chaînes.

diag-lex-line-too-long = cette ligne dépasse 64 Kio et n’est pas analysée
    .help = Découpez-la en plusieurs lignes.

diag-lex-operands-in-comment = a68k ignorera « { $text } » : les opérandes s’arrêtent au premier espace.
    .help = { $cause ->
        [after-comma] Supprimez les espaces après les virgules.
        [inside-expression] Supprimez les espaces dans l’expression.
       *[operand-list] Collez les opérandes, ou commencez le commentaire par « ; ».
    }

## Assembler (m2-assembler.md §7.2)

diag-asm-unknown-operation = instruction ou macro inconnue « { $name } »
    .note = { $count ->
        [0] Les noms de macro distinguent majuscules et minuscules et doivent être définis avant leur premier appel.
       *[other] Vouliez-vous dire { $suggestions } ?
    }

diag-asm-not-a68k-directive = a68k 2.71 ne connaît pas la directive { $name }
    .help = { $name ->
        [ELSE] Fermez le bloc par ENDC et ouvrez un second bloc avec le test IF contraire.
        [REPT] Écrivez les lignes une à une, ou définissez une macro et appelez-la à chaque répétition.
       *[other] Les optimisations se désactivent avec l’option sans optimisation de l’assembleur, pas dans le fichier source.
    }

diag-asm-unsupported-directive = { $name } est une directive d’a68k que VisualAssembler ne prend pas en charge
    .help = { $name ->
        [EQUR] Écrivez le registre lui-même (D0 à D7, A0 à A7) : VisualAssembler ne donne pas d’autre nom aux registres.
        [REG] Écrivez la liste de registres elle-même, par exemple D0-D3/A0 dans MOVEM.
        [RORG] Placez plutôt le code avec ORG.
       *[other] Supprimez cette ligne : VisualAssembler n’a pas de modèle d’adressage proche ou lointain.
    }

diag-asm-not-in-s-format = a68k refuse { $name } quand il produit des S-records
    .help = Placez le code et les données avec ORG : un programme chargé depuis un S-record n’a ni sections ni symboles externes.

diag-asm-bad-operand = opérande illisible « { $text } »
    .help = Écrivez-le sous l’une des formes D0, A1, (A0), (A0)+, -(A0), 8(A0), 4(A0,D1.W), boucle(PC), $1000, ($1000).W, #5 ou D0-D3/A0 ; le déplacement se place devant la parenthèse : 8(A0), et non (8,A0).

diag-asm-bad-expression = expression invalide « { $text } »
    .help = Vérifiez que chaque parenthèse est fermée et qu’aucun opérateur ne manque ni n’est doublé : deux signes de suite, comme --2, sont refusés.

diag-asm-operand-count = { $mnemonic } { $expected ->
        [0] ne prend aucun opérande
        [one] prend un opérande
       *[other] prend { $expected } opérandes
    }, mais la ligne en donne { $found }
    .help = Vérifiez les opérandes : ils sont séparés par des virgules, sans espace, car le premier espace termine le champ des opérandes.

diag-asm-mode-not-allowed = { $mnemonic } n’accepte pas le mode { $mode } ici
    .note = Modes acceptés ici : { $allowed }.

diag-asm-size-not-allowed = { $mnemonic } n’existe pas en .{ $size }
    .note = { $allowed ->
        [none] { $mnemonic } ne prend pas de suffixe de taille.
       *[other] Tailles acceptées : { $allowed }.
    }

diag-asm-no-matching-form = ces opérandes ne correspondent à aucune forme de { $mnemonic }
    .note = Formes acceptées : { $forms }.

diag-asm-register-size-suffix = une taille après un registre (« { $text } ») n’est permise que pour l’index de d8(An,Xn) ou d8(PC,Xn)
    .help = Retirez la taille placée après le registre : la taille de l’opération s’écrit après l’instruction, comme dans MOVE.W D0,D1.

diag-asm-missing-displacement = a68k exige un déplacement devant (An,Xn)
    .help = Écrivez 0 devant la parenthèse, par exemple 0(A0,D0).

diag-asm-absolute-size-syntax = a68k ne lit pas « { $text } » comme une adresse avec taille
    .help = Mettez l’adresse entre parenthèses avant la taille, par exemple ($400).W.

diag-asm-value-out-of-range = { $value } ne tient pas ici (permis : de { $min } à { $max })
    .help = Changez la valeur, ou choisissez une taille plus grande ou une instruction qui l’accepte.

diag-asm-branch-out-of-range = la cible est à { $distance } octets, trop loin pour un branchement .{ $size }
    .help = Retirez la taille et laissez l’assembleur choisir ; pour une cible à plus de 32 Kio, utilisez JMP ou JSR.

diag-asm-short-branch-zero = un branchement court ne peut pas viser l’instruction suivante (son déplacement vaudrait 0)
    .help = Retirez « .s » : l’assembleur écrit alors un branchement sur un mot.

diag-asm-odd-branch-target = la cible { $target } est à une adresse impaire : le 68000 s’arrêtera sur une erreur d’adresse
    .help = Placez l’étiquette sur la ligne de l’instruction qu’elle désigne, ou ajoutez EVEN avant l’étiquette.

diag-asm-undefined-symbol = « { $name } » n’est pas défini
    .help = Définissez-le par une étiquette en colonne 1 ou avec EQU, ou vérifiez son orthographe.
    .note = { $count ->
        [0] Les symboles distinguent majuscules et minuscules.
       *[other] Vouliez-vous dire { $suggestions } ?
    }

diag-asm-undefined-local-label = l’étiquette locale « { $name } » n’est pas définie dans ce bloc
    .note = Une étiquette locale n’est visible que jusqu’à la prochaine étiquette ordinaire, EQU, SET ou « = ».

diag-asm-duplicate-symbol = « { $name } » est déjà défini
    .help = Renommez l’une des deux définitions ; seul un symbole défini par SET peut recevoir une nouvelle valeur, à nouveau par SET.

diag-asm-reserved-name = « { $name } » est une instruction ou une directive et ne peut pas nommer une macro
    .help = Choisissez un autre nom pour la macro, par exemple en lui ajoutant un préfixe.

diag-asm-label-like-keyword = « { $name } » en colonne 1 est une étiquette, pas une instruction
    .help = Indentez la ligne avec un espace ou une tabulation pour en faire une instruction.

diag-asm-label-required = { $name } exige un nom en colonne 1
    .help = Écrivez le nom tout au début de la ligne, puis { $name }.

diag-asm-value-not-known = { $name } exige une valeur connue à cette ligne, mais « { $symbol } » est défini plus bas
    .help = Déplacez la définition de « { $symbol } » au-dessus de cette ligne.

diag-asm-division-by-zero = division par zéro
    .help = Vérifiez la valeur placée après « / » : ici, elle vaut 0.

diag-asm-negative-count = { $name } n’accepte pas de nombre négatif ({ $count })

diag-asm-address-overflow = le programme dépasse { $address }, la fin de l’espace d’adressage du 68000
    .help = Le 68000 adresse 16 Mio, de $000000 à $FFFFFF : vérifiez les valeurs d’ORG et les tailles données à DS et DCB.

diag-asm-overlap = ces octets recouvrent des octets déjà placés en { $address }
    .help = Vérifiez les lignes ORG : deux parties du programme utilisent les mêmes adresses.

diag-asm-string-in-expression = une chaîne entre guillemets ne peut pas entrer dans un calcul (« { $text } »)
    .help = Écrivez le code du caractère, par exemple $42.

diag-asm-string-too-long = une valeur .{ $size } contient au plus { $max } caractères, cette chaîne en a { $length }
    .help = Rangez un texte plus long avec DC.B, qui accepte des chaînes de toute longueur.

diag-asm-text-after-end = les lignes après END sont ignorées
    .help = Placez ces lignes avant END, ou supprimez-les.

diag-asm-unbalanced-conditional = { $kind ->
        [close] { $name } n’a pas d’IF correspondant
       *[open] ce bloc { $name } n’est jamais fermé par ENDC
    }
    .help = { $kind ->
        [close] Supprimez ce { $name }, ou ajoutez la ligne IF qui ouvre son bloc.
       *[open] Ajoutez ENDC après la dernière ligne du bloc.
    }

diag-asm-unbalanced-macro = { $kind ->
        [close] { $name } est hors de toute macro
       *[open] la macro « { $name } » n’est jamais fermée par ENDM
    }
    .help = { $kind ->
        [close] ENDM et MEXIT se placent entre une ligne MACRO et son ENDM : supprimez ce { $name }, ou placez-le dans la macro.
       *[open] Ajoutez ENDM après la dernière ligne de la macro.
    }

diag-asm-macro-param-outside = « { $text } » est un paramètre de macro, mais cette ligne n’est pas dans une macro
    .help = \0 à \9 et \@ ne sont remplacés que dans les lignes du corps d’une macro : supprimez ce paramètre, ou placez la ligne dans une macro.

diag-asm-macro-too-deep = les macros s’appellent sur plus de { $limit } niveaux
    .help = Cherchez une macro qui s’appelle elle-même, directement ou par une autre macro.

diag-asm-expansion-limit = le programme dépasse { $limit } lignes une fois les macros développées ; l’assemblage s’arrête
    .help = Cherchez une macro qui s’appelle elle-même sans fin.

diag-asm-include-too-deep = les fichiers INCLUDE sont imbriqués sur plus de { $limit } niveaux
    .help = Incluez les fichiers depuis le fichier principal plutôt que les uns depuis les autres.

diag-asm-include-cycle = « { $path } » s’inclut lui-même
    .help = Supprimez la ligne INCLUDE qui relit ce fichier, directement ou par un autre fichier.

diag-asm-include-not-found = fichier « { $path } » introuvable
    .note = Les chemins partent du dossier du fichier principal, puis de chaque dossier d’inclusion.

diag-asm-include-refused = le chemin « { $path } » est refusé : { $reason ->
        [absolute] il est absolu
        [drive] il désigne un lecteur, un partage réseau ou un périphérique
        [colon] il contient « : »
        [device] il désigne un périphérique Windows
        [escapes] il sort de son dossier
       *[other] il est vide ou invalide
    }
    .help = Utilisez un chemin relatif dans le dossier du fichier principal, ou ajoutez un dossier d’inclusion.

diag-asm-include-failed = impossible d’utiliser « { $path } » : { $reason ->
        [too-large] il dépasse 16 Mio
        [not-utf8] ce n’est pas du texte UTF-8
        [too-many-files] le programme utilise déjà trop de fichiers
        [unsupported] cet outil ne lit pas les fichiers binaires
       *[other] il est illisible
    }
    .help = { $reason ->
        [too-large] Découpez-le en plusieurs fichiers plus petits.
        [not-utf8] Enregistrez-le de nouveau en texte UTF-8 depuis votre éditeur.
        [too-many-files] Regroupez les fichiers inclus en un plus petit nombre de fichiers.
        [unsupported] Supprimez la ligne INCBIN : les fichiers binaires ne peuvent pas être lus ici.
       *[other] Vérifiez que le fichier existe toujours et que vous avez le droit de le lire.
    }

diag-asm-too-many-diagnostics = seuls les { $limit } premiers problèmes sont signalés
    .help = Corrigez d’abord ces problèmes, puis assemblez de nouveau pour voir les suivants.

diag-asm-operands-in-comment = { $kind ->
        [data] a68k ignorera « { $text } » : { $bytes ->
            [one] seul { $bytes } octet sera produit
           *[other] seuls { $bytes } octets seront produits
        }
       *[instruction] a68k ignorera « { $text } » : seul « { $kept } » est assemblé
    }
    .help = { $cause ->
        [after-comma] Supprimez les espaces après les virgules.
        [inside-expression] Supprimez les espaces dans l’expression.
       *[operand-list] Collez les opérandes, ou commencez le commentaire par « ; ».
    }

diag-asm-movem-single = a68k assemble ce MOVEM d’un seul registre en MOVE.{ $size } : { $effect ->
        [sign] les indicateurs changent et le registre garde son mot haut au lieu d’être étendu par le signe
       *[flags] les indicateurs N et Z changent, V et C sont mis à 0
    }
    .help = Écrivez MOVE vous-même, ou désactivez les optimisations pour garder MOVEM.

diag-asm-phase-error = la valeur de « { $name } » a changé après le placement du code
    .help = Cela ne devrait pas arriver : signalez-le aux développeurs de VisualAssembler, avec le fichier source.

## Related notes of assembler diagnostics (no attribute)

diag-asm-related-first-definition = première définition de « { $name } »

diag-asm-related-scope-start = le bloc des étiquettes locales commence ici

diag-asm-related-included-from = inclus depuis ici

diag-asm-related-in-macro = ligne de la macro « { $name } »

diag-asm-related-other-bytes = les autres octets viennent d’ici
