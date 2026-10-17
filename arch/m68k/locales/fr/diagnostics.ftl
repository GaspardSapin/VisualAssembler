# Diagnostics of the 68000 lexer, French (m1-core-isa-lexer.md §5.4, §5.7, §6.1).
#
# Same ids, attributes and variables as locales/en/diagnostics.ftl: $text everywhere, and $cause
# (after-comma, inside-expression or operand-list) for diag-lex-operands-in-comment.
# Written in our own words, in natural French; typographic quotes and apostrophes (’) only.

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
