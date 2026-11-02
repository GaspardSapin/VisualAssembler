# Problems found in the configuration files, French (m4-editor-shell.md §4.2, §4.6).
#
# Same ids, attributes and variables as locales/en/config.ftl: one diag-<code> message per
# entry of CONFIG_CODES. Written in our own words, in natural French, with the
# m1-core-isa-lexer.md §12.2 typography (regular spaces before ":", ";" and the closing quote).
#
# The selectors on $expected use the ids listed in locales/en/config.ftl. $error and $detail
# stay English (an I/O error kind, the TOML parser's text). TOML examples keep the straight
# double quotes of TOML.

diag-cfg-unreadable = impossible de lire ce fichier de configuration ({ $error }) ; les valeurs par défaut s’appliquent
    .help = Vérifiez que le chemin désigne un fichier ordinaire que votre compte a le droit de lire, puis relancez VisualAssembler ou revenez dans sa fenêtre.

diag-cfg-too-large = ce fichier de configuration fait { $bytes } octets, au-delà de la limite de 1 Mio ; il est ignoré
    .help = Un fichier de configuration ne compte que quelques lignes. Vérifiez qu’il s’agit du bon fichier, ou repartez d’un fichier vide.

diag-cfg-not-utf8 = ce fichier de configuration n’est pas du texte UTF-8 (octet invalide à la position { $offset }) ; il est ignoré
    .help = Enregistrez de nouveau le fichier avec l’encodage UTF-8 ; la plupart des éditeurs de texte le proposent dans leur boîte « Enregistrer sous ».

diag-cfg-syntax = ce fichier n’est pas du TOML valide ({ $detail }) ; le fichier entier est ignoré
    .help = Corrigez l’endroit indiqué. Chaque réglage s’écrit clé = valeur sous l’en-tête de sa [table], et les textes se placent entre guillemets droits, par exemple theme = "dark".

diag-cfg-schema-version = la valeur { $found } de schema_version n’est pas prise en charge ; le fichier entier est ignoré
    .help = Cette version de VisualAssembler lit schema_version = 1. Écrivez 1, ou supprimez la ligne, ce qui revient au même.

diag-cfg-unknown-key = clé inconnue « { $key } » ; elle est ignorée
    .help = Vérifiez son orthographe et l’en-tête [table] sous lequel elle est écrite. Les clés s’écrivent en minuscules, avec « _ » entre les mots.

diag-cfg-wrong-type = { $key } doit être { $expected ->
        [boolean] true ou false
        [integer] un nombre entier
        [number] un nombre
        [string] un texte entre guillemets droits
        [array] une liste entre crochets
        [table] une table
       *[other] une valeur de type { $expected }
    } ; cette valeur est ignorée
    .help = { $expected ->
        [boolean] Écrivez true ou false, sans guillemets.
        [integer] Écrivez le nombre sans guillemets et sans partie décimale, par exemple 300.
        [number] Écrivez le nombre sans guillemets, avec un point pour les décimales, par exemple 14 ou 14.5.
        [string] Placez le texte entre guillemets droits, par exemple "dark".
        [array] Placez les valeurs entre crochets, séparées par des virgules, par exemple [".git", "build"].
        [table] Écrivez-la sous sa propre ligne d’en-tête [table].
       *[other] Écrivez une valeur du type attendu.
    }

diag-cfg-out-of-range = { $key } doit être compris entre { $min } et { $max } ; cette valeur est ignorée
    .help = Choisissez une valeur de { $min } à { $max }, bornes comprises.

diag-cfg-invalid-value = « { $value } » n’est pas une valeur valide pour { $key } ; elle est ignorée
    .help = { $expected ->
        [font-file] Indiquez le chemin d’un fichier de police .ttf ou .otf, absolu ou relatif au dossier des réglages, ou laissez le texte vide pour garder la police intégrée.
        [file-name] Indiquez un seul nom de fichier ou de dossier, non vide et sans « / » ni « \ », par exemple « .git » ou « build ».
       *[other] Les valeurs acceptées sont : { $expected }. Écrivez l’une d’elles en minuscules, entre guillemets droits.
    }

diag-cfg-unknown-command = commande inconnue « { $command } » ; ce raccourci est ignoré
    .help = Vérifiez l’orthographe de l’identifiant de la commande, par exemple file.save ou palette.commands. Pour retirer un raccourci par défaut, écrivez son identifiant après un « - », par exemple "-edit.select-next-occurrence".

diag-cfg-bad-key = « { $key } » n’est pas une combinaison de touches utilisable ; ce raccourci est ignoré
    .help = Écrivez les modificateurs Ctrl, Shift et Alt, puis une seule touche, reliés par « + », par exemple Ctrl+Shift+P ou Alt+F5. Les flèches, Home, End, Page Up, Page Down, Backspace, Delete, Enter, Tab, Escape et les raccourcis copier, couper et coller restent à l’éditeur et ne peuvent pas être attribués.

diag-cfg-bad-path = « { $value } » ne convient pas pour { $key } : le chemin doit rester dans le dossier du projet ; il est ignoré
    .help = Écrivez un chemin relatif au dossier du projet, sans « .. », sans lecteur et sans « / » au début, par exemple « src/main.asm » ou « inc ».
