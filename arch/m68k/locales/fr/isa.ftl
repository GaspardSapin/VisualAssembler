# 68000 instruction documentation, French (m1-core-isa-lexer.md §4.3, §6.1).
#
# Same ids and attributes as locales/en/isa.ftl. Written in our own words (m1 §4.6), in natural
# French rather than a word-for-word translation, with one term per notion: registre de données,
# registre d’adresse, indicateur, retenue, emprunt, débordement, octet, mot, mot long, pile,
# sous-programme, étiquette, branchement, saut, cycles d’horloge.
# Style: no braces in the text, no continuation line starting with a dot, a star or a bracket,
# typographic apostrophes (’) only.
#
# Vocabulary used below: « modes de données » are every addressing mode except An; a
# « modifiable » operand can be written (no PC-relative and no immediate operand); « modes de
# contrôle » are (An), d16(An), d8(An,Xn), xxx.W, xxx.L, d16(PC) and d8(PC,Xn).

## Instructions

isa-m68k-abcd = Additionne deux octets en BCD condensé et l’indicateur X.
    .details = Syntaxe : ABCD Dy,Dx ou ABCD -(Ay),-(Ax), uniquement sur un octet. Chaque octet
        contient deux chiffres décimaux (BCD condensé) ; la somme source + destination + X est
        calculée en décimal puis rangée dans la destination. La forme mémoire progresse vers les
        adresses basses : une suite d’ABCD additionne ainsi de longs nombres décimaux en
        commençant par leur dernier octet. X et C reçoivent la retenue décimale.
    .flags = Z est mis à 0 si le résultat n’est pas nul et reste inchangé sinon. Après une suite
        d’ABCD, il indique donc si le nombre entier est nul, à condition d’avoir mis Z à 1 (et
        X à 0) avant la première. N et V sont indéfinis.

isa-m68k-add = Ajoute la source à la destination.
    .details = Syntaxe : ADD.B/W/L <ea>,Dn ou ADD.B/W/L Dn,<ea> ; sans suffixe, la taille est le
        mot. L’un des deux opérandes doit être un registre de données : la source de la
        première forme accepte tous les modes (An seulement en mot ou mot long), la destination
        de la seconde est un opérande mémoire modifiable. A68k accepte aussi ADD avec un
        registre d’adresse en destination (ADDA) ou une source immédiate (ADDI) et choisit alors
        ce codage. X, N, Z, V et C dépendent tous du résultat. Une addition dans un registre est
        plus rapide qu’une addition en mémoire.

isa-m68k-adda = Ajoute la source à un registre d’adresse.
    .details = Syntaxe : ADDA.W/L <ea>,An ; la source accepte tous les modes d’adressage. Une
        source sur un mot est d’abord étendue à 32 bits avec son signe, et le registre d’adresse
        change toujours en entier. Aucun indicateur n’est modifié, ce qui permet de faire avancer
        un pointeur entre un test et le branchement qui l’exploite. A68k choisit aussi ADDA
        quand on écrit ADD avec un registre d’adresse en destination.

isa-m68k-addi = Ajoute une valeur immédiate à la destination.
    .details = Syntaxe : ADDI.B/W/L #<data>,<ea> ; la destination est un registre de données ou
        un opérande mémoire modifiable. La constante est stockée dans les mots qui suivent le
        code opération, ce qui coûte des lectures mémoire supplémentaires. X, N, Z, V et C
        dépendent du résultat. Pour une constante de 1 à 8, ADDQ est plus court et plus rapide.

isa-m68k-addq = Ajoute une constante de 1 à 8 à la destination.
    .details = Syntaxe : ADDQ.B/W/L #<1-8>,<ea>. La constante est logée dans le code opération
        lui-même : elle ne demande aucun mot d’extension et s’exécute vite. La destination peut
        être n’importe quel opérande modifiable, y compris un registre d’adresse en mot ou en
        mot long. Sur un registre d’adresse, les 32 bits changent et aucun indicateur n’est
        modifié ; sinon X, N, Z, V et C dépendent du résultat.

isa-m68k-addx = Ajoute la source et l’indicateur X à la destination.
    .details = Syntaxe : ADDX.B/W/L Dy,Dx ou ADDX.B/W/L -(Ay),-(Ax). Elle sert aux additions sur
        plus de 32 bits : on additionne les parties basses avec ADD, puis chaque partie plus
        haute avec ADDX, qui ajoute la retenue conservée dans X. La forme mémoire prédécrémente
        les deux pointeurs et parcourt donc les nombres en mémoire depuis leur poids faible. X,
        N, V et C dépendent du résultat.
    .flags = Z est mis à 0 si le résultat n’est pas nul et reste inchangé sinon. Après une suite
        d’ADDX, il indique donc si le résultat complet est nul, à condition d’avoir mis Z à 1
        avant la suite, par exemple avec MOVE #4,CCR.

isa-m68k-and = ET logique bit à bit de la source dans la destination.
    .details = Syntaxe : AND.B/W/L <ea>,Dn ou AND.B/W/L Dn,<ea>. Un bit du résultat vaut 1
        seulement si les deux opérandes ont un 1 à cette position : c’est ainsi qu’un masque
        efface des bits. La source de la première forme accepte tous les modes de données ; la
        destination de la seconde est un opérande mémoire modifiable. A68k accepte aussi une
        source immédiate et produit alors ANDI. N et Z dépendent du résultat, V et C sont mis à
        0 et X ne change pas.

isa-m68k-andi = ET logique bit à bit d’une valeur immédiate dans la destination, CCR ou SR.
    .details = Syntaxe : ANDI.B/W/L #<data>,<ea> avec un registre de données ou un opérande
        mémoire modifiable, ANDI #<data>,CCR pour effacer certains indicateurs, ou
        ANDI #<data>,SR pour effacer certains bits de tout le registre d’état. ANDI vers SR est
        privilégiée : en mode utilisateur, elle déclenche une exception de violation de
        privilège. Dans la forme ordinaire, N et Z dépendent du résultat, V et C sont mis à 0
        et X ne change pas.
    .flags = Avec CCR ou SR en destination, chaque indicateur subit un ET avec le bit
        correspondant de la valeur immédiate : un bit à 0 l’efface, un bit à 1 le laisse tel
        quel.

isa-m68k-asl = Décalage arithmétique vers la gauche.
    .details = Syntaxe : ASL.B/W/L Dx,Dy (nombre de positions dans Dx, modulo 64),
        ASL.B/W/L #<1-8>,Dy, ou ASL <ea> pour décaler d’un bit un mot en mémoire modifiable. Des
        zéros entrent par la droite et chaque décalage d’une position multiplie la valeur par 2.
        X et C reçoivent le dernier bit sorti, N et Z décrivent le résultat, et V passe à 1 si le
        bit de signe a changé à un moment quelconque, ce qui signale un débordement signé. Avec
        un décalage de 0, C est mis à 0 et X ne change pas. Les formes sur registre coûtent 2
        cycles d’horloge de plus par position.

isa-m68k-asr = Décalage arithmétique vers la droite.
    .details = Syntaxe : ASR.B/W/L Dx,Dy (nombre de positions dans Dx, modulo 64),
        ASR.B/W/L #<1-8>,Dy, ou ASR <ea> pour décaler d’un bit un mot en mémoire modifiable. Le
        bit de signe est recopié dans les positions libérées : chaque décalage d’une position
        divise donc une valeur signée par 2, en arrondissant vers moins l’infini. X et C
        reçoivent le dernier bit sorti, N et Z décrivent le résultat, et V est mis à 0. Avec un
        décalage de 0, C est mis à 0 et X ne change pas. Les formes sur registre coûtent 2 cycles
        d’horloge de plus par position.

isa-m68k-bcc = Se branche à une étiquette si la condition est vraie.
    .details = Syntaxe : Bcc <label>, où cc vaut HI, LS, CC (ou HS), CS (ou LO), NE, EQ, VC, VS,
        PL, MI, GE, LT, GT ou LE. La condition porte sur les indicateurs laissés par une
        instruction précédente, en général CMP ou TST : si elle est vraie, l’exécution continue
        à l’étiquette, sinon à l’instruction suivante. La taille courte S code un déplacement
        sur 8 bits (environ 128 octets dans chaque sens) et la taille mot W un déplacement sur
        16 bits (environ 32 Ko) ; A68k lit le suffixe B comme S et L comme W, et l’assembleur
        choisit lui-même sans suffixe. Aucun indicateur n’est modifié. Un branchement pris coûte
        10 cycles d’horloge ; non pris, il en coûte 8 en taille courte et 12 en taille mot.

isa-m68k-bchg = Teste un bit, puis l’inverse.
    .details = Syntaxe : BCHG Dn,<ea> ou BCHG #<number>,<ea> ; le premier opérande donne le
        numéro du bit. Sur un registre de données, l’instruction travaille sur 32 bits (numéro
        modulo 32) ; sur un opérande mémoire modifiable, elle travaille sur un octet (numéro
        modulo 8). Z passe à 1 si le bit valait 0 avant l’inversion et à 0 s’il valait 1 ; les
        autres indicateurs ne changent pas. Sur un registre, la durée dépend du numéro du bit :
        les tables en donnent le maximum.

isa-m68k-bclr = Teste un bit, puis le met à 0.
    .details = Syntaxe : BCLR Dn,<ea> ou BCLR #<number>,<ea> ; le premier opérande donne le
        numéro du bit. Sur un registre de données, l’instruction travaille sur 32 bits (numéro
        modulo 32) ; sur un opérande mémoire modifiable, elle travaille sur un octet (numéro
        modulo 8). Z passe à 1 si le bit valait 0 avant d’être effacé et à 0 s’il valait 1 ; les
        autres indicateurs ne changent pas. Sur un registre, la durée dépend du numéro du bit :
        les tables en donnent le maximum.

isa-m68k-bra = Se branche à une étiquette sans condition.
    .details = Syntaxe : BRA <label>, en taille courte S (déplacement sur 8 bits) ou en taille mot
        W (déplacement sur 16 bits) ; A68k lit le suffixe B comme S et L comme W. L’exécution
        continue toujours à l’étiquette, dont la distance est relative au compteur ordinal : le
        code fonctionne donc quelle que soit l’adresse où il est chargé. Aucun indicateur n’est
        modifié. Les deux tailles prennent 10 cycles d’horloge ; pour une cible à plus de 32 Ko
        environ, il faut JMP.

isa-m68k-bset = Teste un bit, puis le met à 1.
    .details = Syntaxe : BSET Dn,<ea> ou BSET #<number>,<ea> ; le premier opérande donne le
        numéro du bit. Sur un registre de données, l’instruction travaille sur 32 bits (numéro
        modulo 32) ; sur un opérande mémoire modifiable, elle travaille sur un octet (numéro
        modulo 8). Z passe à 1 si le bit valait 0 avant d’être mis à 1, et à 0 s’il valait déjà
        1 ; les autres indicateurs ne changent pas. Sur un registre, la durée dépend du numéro
        du bit : les tables en donnent le maximum.

isa-m68k-bsr = Appelle un sous-programme situé à une étiquette proche.
    .details = Syntaxe : BSR <label>, en taille courte S ou en taille mot W, comme BRA. L’adresse
        de l’instruction suivante est empilée sous forme de mot long (A7 diminue de 4), puis
        l’exécution continue à l’étiquette ; RTS y reviendra ensuite. La cible est relative au
        compteur ordinal et doit se trouver à moins de 32 Ko environ ; au-delà, utilisez JSR.
        Aucun indicateur n’est modifié, et l’appel prend 18 cycles d’horloge.

isa-m68k-btst = Teste un bit.
    .details = Syntaxe : BTST Dn,<ea> ou BTST #<number>,<ea>. Z passe à 1 si le bit choisi vaut 0
        et à 0 s’il vaut 1 ; ni l’opérande ni les autres indicateurs ne changent, et un BEQ ou
        un BNE suit en général. Un registre de données est testé sur 32 bits (numéro modulo 32)
        et un opérande mémoire sur un octet (numéro modulo 8). Contrairement aux autres
        instructions sur les bits, BTST peut aussi lire un opérande relatif au PC, et même un
        opérande immédiat quand le numéro du bit est dans un registre.

isa-m68k-chk = Compare un registre à une borne et déclenche une exception en cas de dépassement.
    .details = Syntaxe : CHK <ea>,Dn, uniquement en taille mot sur le 68000 ; la source contient
        la borne supérieure et accepte tous les modes de données. Le mot bas de Dn est comparé,
        en tant que nombre signé, à 0 et à la borne. S’il est négatif ou supérieur à la borne,
        le processeur déclenche l’exception CHK (vecteur 6) ; sinon, l’exécution continue
        simplement. C’est une protection peu coûteuse pour un indice de tableau, et le cas avec
        exception est bien plus lent que le cas normal.
    .flags = N passe à 1 si Dn était négatif et à 0 si Dn dépassait la borne ; sans exception, il
        est indéfini. Z, V et C sont indéfinis et X ne change pas.

isa-m68k-clr = Met la destination à zéro.
    .details = Syntaxe : CLR.B/W/L <ea>, avec un registre de données ou un opérande mémoire
        modifiable ; pour un registre d’adresse, utilisez plutôt SUBA.L An,An. N, V et C sont
        mis à 0, Z est mis à 1 et X ne change pas. Sur le 68000, CLR lit sa destination en
        mémoire avant de l’écrire, ce qui compte pour certains registres matériels.
        MOVEQ #0,Dn efface un registre de données entier plus vite que CLR.L Dn.

isa-m68k-cmp = Compare un registre de données à la source.
    .details = Syntaxe : CMP.B/W/L <ea>,Dn ; la source accepte tous les modes (An seulement en
        mot ou mot long). Le processeur soustrait la source de Dn et ne garde que les
        indicateurs, pour qu’un branchement conditionnel puisse suivre : après CMP src,Dn, BGT
        se branche si Dn est plus grand que src en nombres signés, et BHI s’il est plus grand en
        nombres non signés. N, Z, V et C dépendent de la soustraction et X ne change pas. A68k
        accepte aussi CMP pour CMPA, CMPI et CMPM et choisit le codage d’après les opérandes.

isa-m68k-cmpa = Compare un registre d’adresse à la source.
    .details = Syntaxe : CMPA.W/L <ea>,An ; la source accepte tous les modes. Une source sur un
        mot est étendue à 32 bits avec son signe, et la comparaison porte toujours sur le
        registre d’adresse entier. N, Z, V et C sont positionnés comme par CMP et X ne change
        pas ; pour des adresses, ce sont les conditions non signées (HI, LS, CC, CS) qui ont un
        sens.

isa-m68k-cmpi = Compare la destination à une valeur immédiate.
    .details = Syntaxe : CMPI.B/W/L #<data>,<ea> ; sur le 68000, la destination est un registre
        de données ou un opérande mémoire modifiable. La valeur immédiate est soustraite de la
        destination et seuls les indicateurs sont conservés : N, Z, V et C sont positionnés et
        X ne change pas. Pour comparer un registre d’adresse à une constante, utilisez CMPA.

isa-m68k-cmpm = Compare deux opérandes en mémoire et fait avancer les deux pointeurs.
    .details = Syntaxe : CMPM.B/W/L (Ay)+,(Ax)+. La source est soustraite de la destination et
        seuls les indicateurs sont conservés, puis les deux registres d’adresse passent aux
        données suivantes. Elle est faite pour les boucles qui comparent deux chaînes ou deux
        tampons, souvent fermées par DBNE. N, Z, V et C sont positionnés et X ne change pas.

isa-m68k-dbcc = Contrôle de boucle : décrémente un compteur et se branche tant que la condition est fausse et que le compteur n’est pas épuisé.
    .details = Syntaxe : DBcc Dn,<label>, où cc est l’une des 16 conditions ; DBF s’écrit aussi
        DBRA. Si la condition est vraie, la boucle s’arrête et l’exécution passe à la suite.
        Sinon, le mot bas de Dn est décrémenté, et le processeur se branche à l’étiquette sauf
        si le compteur vient d’atteindre -1 ; une boucle DBRA dont le compteur part de n
        s’exécute donc n + 1 fois. Seul le mot bas de Dn change, et aucun indicateur n’est
        modifié. L’étiquette doit se trouver à moins de 32 Ko environ, et chaque tour de boucle
        (branchement pris) coûte 10 cycles d’horloge.

isa-m68k-divs = Division signée d’un registre de 32 bits par une source de 16 bits.
    .details = Syntaxe : DIVS <ea>,Dn, en taille mot ; la source accepte tous les modes de
        données. La valeur 32 bits de Dn est divisée par le mot source signé : le quotient va
        dans le mot bas de Dn et le reste, du signe du dividende, dans le mot haut. Si le
        quotient ne tient pas sur 16 bits, V passe à 1 et Dn reste inchangé ; un diviseur nul
        déclenche l’exception de division par zéro (vecteur 5). N et Z décrivent le quotient
        (ils sont indéfinis après un débordement), C est toujours mis à 0 et X ne change pas.
        C’est l’une des instructions les plus lentes, et les tables en donnent le pire cas.

isa-m68k-divu = Division non signée d’un registre de 32 bits par une source de 16 bits.
    .details = Syntaxe : DIVU <ea>,Dn, en taille mot ; la source accepte tous les modes de
        données. La valeur 32 bits de Dn est divisée par le mot source non signé : le quotient
        va dans le mot bas de Dn et le reste dans le mot haut. Si le quotient ne tient pas sur
        16 bits, V passe à 1 et Dn reste inchangé ; un diviseur nul déclenche l’exception de
        division par zéro (vecteur 5). N et Z décrivent le quotient (ils sont indéfinis après un
        débordement), C est toujours mis à 0 et X ne change pas. C’est l’une des instructions
        les plus lentes, et les tables en donnent le pire cas.

isa-m68k-eor = OU exclusif bit à bit d’un registre de données dans la destination.
    .details = Syntaxe : EOR.B/W/L Dn,<ea> ; la source est obligatoirement un registre de données
        et la destination un registre de données ou un opérande mémoire modifiable. Un bit du
        résultat vaut 1 là où les deux opérandes diffèrent : un EOR avec un masque inverse donc
        les bits choisis. A68k accepte aussi une source immédiate et produit alors EORI. N et Z
        dépendent du résultat, V et C sont mis à 0 et X ne change pas.

isa-m68k-eori = OU exclusif bit à bit d’une valeur immédiate dans la destination, CCR ou SR.
    .details = Syntaxe : EORI.B/W/L #<data>,<ea> avec un registre de données ou un opérande
        mémoire modifiable, EORI #<data>,CCR pour inverser certains indicateurs, ou
        EORI #<data>,SR pour inverser certains bits de tout le registre d’état. EORI vers SR est
        privilégiée : en mode utilisateur, elle déclenche une exception de violation de
        privilège. Dans la forme ordinaire, N et Z dépendent du résultat, V et C sont mis à 0
        et X ne change pas.
    .flags = Avec CCR ou SR en destination, chaque indicateur dont le bit vaut 1 dans la valeur
        immédiate est inversé ; les autres gardent leur valeur.

isa-m68k-exg = Échange le contenu de deux registres.
    .details = Syntaxe : EXG Rx,Ry, avec deux registres de données, deux registres d’adresse, ou
        un de chaque. Les 32 bits sont échangés : la seule taille est donc le mot long. Aucun
        indicateur n’est modifié, et l’instruction prend 6 cycles d’horloge.

isa-m68k-ext = Étend le signe d’un registre de données.
    .details = Syntaxe : EXT.W Dn recopie le bit 7 dans les bits 8 à 15 (d’octet à mot), et
        EXT.L Dn recopie le bit 15 dans les bits 16 à 31 (de mot à mot long) ; sans suffixe, la
        taille est le mot. Elle prépare un octet ou un mot signé pour un calcul sur une taille
        plus grande : EXT.W suivie d’EXT.L transforme un octet en mot long. N et Z dépendent du
        résultat, V et C sont mis à 0 et X ne change pas.

isa-m68k-illegal = Déclenche toujours l’exception d’instruction illégale.
    .details = Syntaxe : ILLEGAL, sans opérande ni taille. Son code opération ($4AFC) est
        réservé pour ne jamais correspondre à une instruction valide : le processeur déclenche
        l’exception d’instruction illégale (vecteur 4) et passe en mode superviseur. Les
        programmes s’en servent pour s’arrêter volontairement, et les débogueurs comme point
        d’arrêt. Les indicateurs ne changent pas.

isa-m68k-jmp = Saute à une adresse.
    .details = Syntaxe : JMP <ea>, avec un mode de contrôle. L’exécution continue à l’adresse
        effective elle-même, et non à une valeur lue en mémoire : JMP (A0) va à l’adresse
        contenue dans A0. Contrairement à BRA, elle atteint tout l’espace d’adressage, et avec
        un registre d’index elle permet des tables de sauts. Aucun indicateur n’est modifié ; la
        durée dépend du mode d’adressage.

isa-m68k-jsr = Appelle un sous-programme situé à n’importe quelle adresse.
    .details = Syntaxe : JSR <ea>, avec un mode de contrôle, comme JMP. L’adresse de
        l’instruction suivante est empilée sous forme de mot long, puis l’exécution continue à
        l’adresse effective ; RTS y reviendra ensuite. Elle sert pour les sous-programmes
        lointains ou les appels à travers un registre, comme JSR (A0). Aucun indicateur n’est
        modifié ; la durée dépend du mode d’adressage.

isa-m68k-lea = Charge une adresse effective dans un registre d’adresse.
    .details = Syntaxe : LEA <ea>,An, en mot long, avec un mode de contrôle. L’adresse de
        l’opérande est calculée et rangée dans An, sans aucune lecture en mémoire :
        LEA 8(A0),A1 donne à A1 la valeur A0 + 8. C’est la façon habituelle de faire pointer un
        registre sur un tableau ou une chaîne, y compris avec un opérande relatif au PC comme
        LEA texte(PC),A0. Aucun indicateur n’est modifié.

isa-m68k-link = Crée le cadre de pile d’un sous-programme.
    .details = Syntaxe : LINK An,#<displacement>, avec un déplacement signé sur 16 bits,
        généralement négatif. Le processeur empile An, copie le pointeur de pile dans An, puis
        ajoute le déplacement au pointeur de pile, ce qui réserve la place des variables
        locales. Dans le sous-programme, les arguments se trouvent alors à des déplacements
        positifs par rapport à An et les variables locales à des déplacements négatifs ; UNLK
        supprime le cadre. Aucun indicateur n’est modifié, et l’instruction prend 16 cycles
        d’horloge.

isa-m68k-lsl = Décalage logique vers la gauche.
    .details = Syntaxe : LSL.B/W/L Dx,Dy (nombre de positions dans Dx, modulo 64),
        LSL.B/W/L #<1-8>,Dy, ou LSL <ea> pour décaler d’un bit un mot en mémoire modifiable. Des
        zéros entrent par la droite, et chaque décalage d’une position multiplie une valeur non
        signée par 2. X et C reçoivent le dernier bit sorti, N et Z décrivent le résultat, et V
        est mis à 0. Avec un décalage de 0, C est mis à 0 et X ne change pas. Les formes sur
        registre coûtent 2 cycles d’horloge de plus par position.

isa-m68k-lsr = Décalage logique vers la droite.
    .details = Syntaxe : LSR.B/W/L Dx,Dy (nombre de positions dans Dx, modulo 64),
        LSR.B/W/L #<1-8>,Dy, ou LSR <ea> pour décaler d’un bit un mot en mémoire modifiable. Des
        zéros entrent par la gauche, et chaque décalage d’une position divise une valeur non
        signée par 2. X et C reçoivent le dernier bit sorti, N et Z décrivent le résultat, et V
        est mis à 0. Avec un décalage de 0, C est mis à 0 et X ne change pas. Les formes sur
        registre coûtent 2 cycles d’horloge de plus par position.

isa-m68k-move = Copie la source dans la destination.
    .details = Syntaxe : MOVE.B/W/L <ea>,<ea> ; sans suffixe, la taille est le mot. La source
        accepte tous les modes (An seulement en mot ou mot long) et la destination est un
        registre de données ou un opérande mémoire modifiable ; A68k transforme une copie vers
        un registre d’adresse en MOVEA. D’autres formes chargent les indicateurs ou tout le
        registre d’état depuis la source (MOVE vers CCR, MOVE vers SR), copient SR ailleurs, ou
        transfèrent USP depuis ou vers un registre d’adresse ; MOVE vers SR et les deux formes
        de MOVE USP sont privilégiées. Dans la forme ordinaire, N et Z dépendent de la valeur copiée, V et C
        sont mis à 0 et X ne change pas. Les copies de mémoire à mémoire sont permises, et la
        durée dépend des deux modes d’adressage.

isa-m68k-movea = Copie la source dans un registre d’adresse.
    .details = Syntaxe : MOVEA.W/L <ea>,An ; la source accepte tous les modes. Un mot est étendu
        à 32 bits avec son signe : le registre change donc toujours en entier. Contrairement à
        MOVE, aucun indicateur n’est modifié, si bien que charger un pointeur ne perturbe pas un
        test en attente. A68k choisit aussi MOVEA quand on écrit MOVE avec un registre
        d’adresse en destination.

isa-m68k-movem = Transfère plusieurs registres vers la mémoire ou depuis la mémoire.
    .details = Syntaxe : MOVEM.W/L <list>,<ea> pour ranger, ou MOVEM.W/L <ea>,<list> pour
        charger, où une liste comme D0-D3/A0/A2 désigne les registres. Le rangement accepte -(An)
        ou un mode de contrôle autre que relatif au PC, le chargement (An)+ ou n’importe quel
        mode de contrôle : MOVEM.L D0-D7/A0-A6,-(SP) sauvegarde des registres et
        MOVEM.L (SP)+,D0-D7/A0-A6 les restaure. En mémoire, les registres apparaissent toujours
        dans l’ordre D0 à A7 depuis l’adresse la plus basse, et les mots chargés dans des
        registres sont étendus à 32 bits avec leur signe. Aucun indicateur n’est modifié. La
        durée croît avec le nombre de registres : 4 cycles d’horloge chacun en taille mot et 8 en
        mot long.

isa-m68k-movep = Transfère des données entre un registre de données et un octet sur deux en mémoire.
    .details = Syntaxe : MOVEP.W/L Dx,d16(Ay) ou MOVEP.W/L d16(Ay),Dx. Les octets du registre,
        poids fort en premier, vont vers les adresses d16(Ay), d16+2(Ay), et ainsi de suite, ou
        en proviennent, en sautant un octet à chaque fois. Cela convient aux périphériques 8 bits
        branchés sur une seule moitié du bus de données de 16 bits. Un mot transfère 2 octets et
        un mot long 4. Aucun indicateur n’est modifié.

isa-m68k-moveq = Charge une petite constante signée dans un registre de données.
    .details = Syntaxe : MOVEQ #<data>,Dn, avec une constante de -128 à 127 ; la taille est
        toujours le mot long. La constante de 8 bits est stockée dans le code opération et
        étendue à 32 bits avec son signe : MOVEQ #-1,D0 donne donc $FFFFFFFF à D0. C’est la
        façon la plus rapide de charger une petite valeur, en 4 cycles d’horloge. N et Z
        dépendent de la valeur, V et C sont mis à 0 et X ne change pas.

isa-m68k-muls = Multiplication signée de deux valeurs de 16 bits en un résultat de 32 bits.
    .details = Syntaxe : MULS <ea>,Dn, en taille mot ; la source accepte tous les modes de
        données. Le mot bas de Dn est multiplié par le mot source, tous deux pris comme des
        nombres signés, et le produit complet sur 32 bits remplace Dn : aucun débordement n’est
        possible. N et Z dépendent du produit, V et C sont mis à 0 et X ne change pas. La durée
        dépend des bits de la source : 38 cycles d’horloge plus 2 par paire de bits voisins
        différents, 70 au maximum.

isa-m68k-mulu = Multiplication non signée de deux valeurs de 16 bits en un résultat de 32 bits.
    .details = Syntaxe : MULU <ea>,Dn, en taille mot ; la source accepte tous les modes de
        données. Le mot bas de Dn est multiplié par le mot source, tous deux pris comme des
        nombres non signés, et le produit complet sur 32 bits remplace Dn : aucun débordement
        n’est possible. N et Z dépendent du produit, V et C sont mis à 0 et X ne change pas. La
        durée dépend de la source : 38 cycles d’horloge plus 2 par bit à 1, 70 au maximum.

isa-m68k-nbcd = Calcule l’opposé d’un octet en BCD condensé, en tenant compte de l’indicateur X.
    .details = Syntaxe : NBCD <ea>, uniquement sur un octet, avec un registre de données ou un
        opérande mémoire modifiable. Le processeur calcule 0 - destination - X en décimal, ce qui
        donne le complément à dix d’un nombre en BCD condensé, ou son complément à neuf quand X
        vaut 1. X et C reçoivent l’emprunt décimal.
    .flags = Z est mis à 0 si le résultat n’est pas nul et reste inchangé sinon, comme pour ABCD
        et SBCD. N et V sont indéfinis.

isa-m68k-neg = Calcule l’opposé de la destination (complément à deux).
    .details = Syntaxe : NEG.B/W/L <ea>, avec un registre de données ou un opérande mémoire
        modifiable. La destination est remplacée par 0 moins elle-même, ce qui change le signe
        d’un nombre signé. X et C passent à 1 sauf si l’opérande était nul, V passe à 1 quand
        l’opérande est la valeur la plus négative (comme $80 sur un octet, qui reste $80), et N
        et Z décrivent le résultat.

isa-m68k-negx = Calcule l’opposé de la destination en tenant compte de l’indicateur X.
    .details = Syntaxe : NEGX.B/W/L <ea>, avec un registre de données ou un opérande mémoire
        modifiable. Le processeur calcule 0 - destination - X : un nombre rangé en plusieurs
        parties s’inverse donc avec NEG sur sa partie basse, puis NEGX sur chaque partie plus
        haute. X, N, V et C dépendent du résultat.
    .flags = Z est mis à 0 si le résultat n’est pas nul et reste inchangé sinon : après une
        suite, il indique donc si le résultat complet est nul.

isa-m68k-nop = Ne fait rien.
    .details = Syntaxe : NOP, sans opérande ni taille. Le processeur passe simplement à
        l’instruction suivante, ce qui prend 4 cycles d’horloge et 2 octets. Elle réserve de la
        place pour une correction ultérieure ou ajoute un délai court et précis. Aucun
        indicateur n’est modifié.

isa-m68k-not = Inverse chaque bit de la destination (complément à un).
    .details = Syntaxe : NOT.B/W/L <ea>, avec un registre de données ou un opérande mémoire
        modifiable. Chaque bit à 0 passe à 1 et chaque bit à 1 passe à 0. N et Z dépendent du
        résultat, V et C sont mis à 0 et X ne change pas.

isa-m68k-or = OU logique bit à bit de la source dans la destination.
    .details = Syntaxe : OR.B/W/L <ea>,Dn ou OR.B/W/L Dn,<ea>. Un bit du résultat vaut 1 dès
        qu’un des opérandes a un 1 à cette position : c’est ainsi qu’un masque met des bits à 1.
        La source de la première forme accepte tous les modes de données ; la destination de la
        seconde est un opérande mémoire modifiable. A68k accepte aussi une source immédiate et
        produit alors ORI. N et Z dépendent du résultat, V et C sont mis à 0 et X ne change pas.

isa-m68k-ori = OU logique bit à bit d’une valeur immédiate dans la destination, CCR ou SR.
    .details = Syntaxe : ORI.B/W/L #<data>,<ea> avec un registre de données ou un opérande
        mémoire modifiable, ORI #<data>,CCR pour mettre certains indicateurs à 1, ou
        ORI #<data>,SR pour mettre à 1 certains bits de tout le registre d’état, comme le masque
        d’interruption. ORI vers SR est privilégiée : en mode utilisateur, elle déclenche une
        exception de violation de privilège. Dans la forme ordinaire, N et Z dépendent du
        résultat, V et C sont mis à 0 et X ne change pas.
    .flags = Avec CCR ou SR en destination, chaque indicateur dont le bit vaut 1 dans la valeur
        immédiate est mis à 1 ; les autres gardent leur valeur.

isa-m68k-pea = Empile une adresse effective.
    .details = Syntaxe : PEA <ea>, avec un mode de contrôle. L’adresse de l’opérande, et non son
        contenu, est calculée et empilée sous forme de mot long, et A7 diminue de 4. C’est une
        façon compacte de passer un pointeur à un sous-programme, par exemple PEA tampon(PC).
        Aucun indicateur n’est modifié.

isa-m68k-reset = Réinitialise les périphériques externes (privilégiée).
    .details = Syntaxe : RESET, sans opérande. Le processeur active sa sortie RESET pendant 124
        cycles d’horloge pour ramener les périphériques à leur état initial ; ses propres
        registres ne changent pas et l’exécution continue à l’instruction suivante. Elle est
        privilégiée : en mode utilisateur, elle déclenche une exception de violation de
        privilège. Aucun indicateur n’est modifié.

isa-m68k-rol = Rotation vers la gauche, sans l’indicateur X.
    .details = Syntaxe : ROL.B/W/L Dx,Dy (nombre de positions dans Dx, modulo 64),
        ROL.B/W/L #<1-8>,Dy, ou ROL <ea> pour faire tourner d’un bit un mot en mémoire
        modifiable. Le bit qui sort à gauche revient à droite : aucun bit n’est perdu. C reçoit
        le dernier bit sorti, N et Z décrivent le résultat, V est mis à 0 et X ne change pas ;
        avec une rotation de 0, C est mis à 0. Les formes sur registre coûtent 2 cycles
        d’horloge de plus par position.

isa-m68k-ror = Rotation vers la droite, sans l’indicateur X.
    .details = Syntaxe : ROR.B/W/L Dx,Dy (nombre de positions dans Dx, modulo 64),
        ROR.B/W/L #<1-8>,Dy, ou ROR <ea> pour faire tourner d’un bit un mot en mémoire
        modifiable. Le bit qui sort à droite revient à gauche : aucun bit n’est perdu. C reçoit
        le dernier bit sorti, N et Z décrivent le résultat, V est mis à 0 et X ne change pas ;
        avec une rotation de 0, C est mis à 0. Les formes sur registre coûtent 2 cycles
        d’horloge de plus par position.

isa-m68k-roxl = Rotation vers la gauche à travers l’indicateur X.
    .details = Syntaxe : ROXL.B/W/L Dx,Dy (nombre de positions dans Dx, modulo 64),
        ROXL.B/W/L #<1-8>,Dy, ou ROXL <ea> pour un bit sur un mot en mémoire modifiable ; A68k
        accepte aussi l’écriture ROLX. X joue le rôle d’un bit supplémentaire : il entre à
        droite, tandis que le bit qui sort à gauche va dans X et C. Pour décaler un nombre en
        plusieurs parties, on applique LSL à la partie basse, puis ROXL à chaque partie plus
        haute. N et Z décrivent le résultat et V est mis à 0 ; avec une rotation de 0, C prend
        la valeur de X et X ne change pas.

isa-m68k-roxr = Rotation vers la droite à travers l’indicateur X.
    .details = Syntaxe : ROXR.B/W/L Dx,Dy (nombre de positions dans Dx, modulo 64),
        ROXR.B/W/L #<1-8>,Dy, ou ROXR <ea> pour un bit sur un mot en mémoire modifiable ; A68k
        accepte aussi l’écriture RORX. X joue le rôle d’un bit supplémentaire : il entre à
        gauche, tandis que le bit qui sort à droite va dans X et C. Pour décaler un nombre en
        plusieurs parties, on applique LSR ou ASR à la partie haute, puis ROXR à chaque partie
        plus basse. N et Z décrivent le résultat et V est mis à 0 ; avec une rotation de 0, C
        prend la valeur de X et X ne change pas.

isa-m68k-rte = Termine un gestionnaire d’exception (privilégiée).
    .details = Syntaxe : RTE, sans opérande. Le processeur dépile le registre d’état, puis le
        compteur ordinal, depuis la pile superviseur, ce qui rétablit le mode, le masque
        d’interruption et les indicateurs sauvegardés au début de l’exception. Elle est
        privilégiée, car elle peut faire repasser le processeur en mode utilisateur. Elle prend
        20 cycles d’horloge.

isa-m68k-rtr = Revient d’un sous-programme en restaurant les indicateurs.
    .details = Syntaxe : RTR, sans opérande. Le processeur dépile un mot dont l’octet bas devient
        le registre des codes condition, puis dépile l’adresse de retour dans le compteur
        ordinal. Elle va de pair avec un sous-programme qui commence par MOVE SR,-(SP), afin que
        l’appelant retrouve ses indicateurs. Elle prend 20 cycles d’horloge.

isa-m68k-rts = Revient d’un sous-programme.
    .details = Syntaxe : RTS, sans opérande. Le processeur dépile l’adresse de retour, un mot
        long empilé par BSR ou JSR, et continue à cette adresse ; A7 augmente de 4. Le pointeur
        de pile doit être revenu là où il était juste après l’appel, sinon le processeur repart
        à une mauvaise adresse. Aucun indicateur n’est modifié ; elle prend 16 cycles
        d’horloge.

isa-m68k-sbcd = Soustrait des octets en BCD condensé, en tenant compte de l’indicateur X.
    .details = Syntaxe : SBCD Dy,Dx ou SBCD -(Ay),-(Ax), uniquement sur un octet. Le processeur
        calcule destination - source - X en décimal et range le résultat dans la destination.
        Comme pour ABCD, la forme mémoire progresse vers les adresses basses : une suite de SBCD
        soustrait ainsi de longs nombres décimaux en commençant par leur dernier octet. X et C
        reçoivent l’emprunt décimal.
    .flags = Z est mis à 0 si le résultat n’est pas nul et reste inchangé sinon. Après une suite
        de SBCD, il indique donc si le nombre entier est nul, à condition d’avoir mis Z à 1 (et
        X à 0) avant la première. N et V sont indéfinis.

isa-m68k-scc = Met un octet à 1 partout si la condition est vraie, à zéro sinon.
    .details = Syntaxe : Scc <ea>, sur un octet, avec un registre de données ou un opérande
        mémoire modifiable ; cc est l’une des 16 conditions, y compris T (toujours) et F
        (jamais). L’octet devient $FF si la condition est vraie et $00 si elle est fausse, ce qui
        conserve le résultat d’un test sous forme de booléen. Aucun indicateur n’est modifié.
        Sur un registre de données, une condition vraie coûte 2 cycles d’horloge de plus qu’une
        condition fausse.

isa-m68k-stop = Charge le registre d’état et attend une interruption (privilégiée).
    .details = Syntaxe : STOP #<data>, avec une valeur immédiate sur 16 bits. La valeur est
        copiée dans le registre d’état, puis le processeur cesse d’exécuter des instructions
        jusqu’à une interruption de niveau supérieur au nouveau masque, une exception de trace
        ou une réinitialisation. Elle permet d’attendre un événement sans boucle d’attente
        active. Elle est privilégiée : en mode utilisateur, elle déclenche une exception de
        violation de privilège.

isa-m68k-sub = Soustrait la source de la destination.
    .details = Syntaxe : SUB.B/W/L <ea>,Dn ou SUB.B/W/L Dn,<ea> ; la destination devient
        destination moins source. L’un des deux opérandes doit être un registre de données : la
        source de la première forme accepte tous les modes (An seulement en mot ou mot long), la
        destination de la seconde est un opérande mémoire modifiable. A68k accepte aussi SUB
        avec un registre d’adresse en destination (SUBA) ou une source immédiate (SUBI). X, N,
        Z, V et C dépendent du résultat ; C et X signalent un emprunt non signé.

isa-m68k-suba = Soustrait la source d’un registre d’adresse.
    .details = Syntaxe : SUBA.W/L <ea>,An ; la source accepte tous les modes. Une source sur un
        mot est d’abord étendue à 32 bits avec son signe, et le registre d’adresse change
        toujours en entier. Aucun indicateur n’est modifié. SUBA.L An,An est un moyen rapide de
        mettre un registre d’adresse à zéro.

isa-m68k-subi = Soustrait une valeur immédiate de la destination.
    .details = Syntaxe : SUBI.B/W/L #<data>,<ea> ; la destination est un registre de données ou
        un opérande mémoire modifiable. La constante est stockée dans les mots qui suivent le
        code opération. X, N, Z, V et C dépendent du résultat. Pour une constante de 1 à 8, SUBQ
        est plus court et plus rapide.

isa-m68k-subq = Soustrait une constante de 1 à 8 de la destination.
    .details = Syntaxe : SUBQ.B/W/L #<1-8>,<ea>. La constante est logée dans le code opération
        lui-même : elle ne demande aucun mot d’extension et s’exécute vite. La destination peut
        être n’importe quel opérande modifiable, y compris un registre d’adresse en mot ou en
        mot long. Sur un registre d’adresse, les 32 bits changent et aucun indicateur n’est
        modifié ; sinon X, N, Z, V et C dépendent du résultat.

isa-m68k-subx = Soustrait la source et l’indicateur X de la destination.
    .details = Syntaxe : SUBX.B/W/L Dy,Dx ou SUBX.B/W/L -(Ay),-(Ax). La destination devient
        destination - source - X, ce qui enchaîne des soustractions sur plus de 32 bits : SUB
        sur les parties basses, puis SUBX sur chaque partie plus haute. La forme mémoire
        prédécrémente les deux pointeurs et commence donc par le poids faible. X, N, V et C
        dépendent du résultat.
    .flags = Z est mis à 0 si le résultat n’est pas nul et reste inchangé sinon. Après une suite
        de SUBX, il indique donc si le résultat complet est nul, à condition d’avoir mis Z à 1
        avant la suite.

isa-m68k-swap = Échange les deux moitiés d’un registre de données.
    .details = Syntaxe : SWAP Dn, en taille mot. Les bits 31 à 16 et 15 à 0 échangent leurs
        places, ce qui rend le mot haut accessible, par exemple le reste après une division. N
        et Z dépendent du résultat complet sur 32 bits, V et C sont mis à 0 et X ne change pas.
        Elle prend 4 cycles d’horloge.

isa-m68k-tas = Teste un octet et met son bit 7 à 1 en une seule opération indivisible.
    .details = Syntaxe : TAS <ea>, sur un octet, avec un registre de données ou un opérande
        mémoire modifiable. Le processeur lit l’octet, positionne N et Z d’après son ancienne
        valeur, puis le réécrit avec le bit 7 à 1, en un seul cycle de bus
        lecture-modification-écriture qu’aucun autre maître du bus ne peut interrompre. Elle
        sert ainsi de verrou pour des données partagées entre processeurs. V et C sont mis à 0
        et X ne change pas.

isa-m68k-trap = Appelle le système par l’un des 16 vecteurs de trappe.
    .details = Syntaxe : TRAP #<vector>, avec un numéro de 0 à 15. Le processeur passe en mode
        superviseur, sauvegarde le compteur ordinal et le registre d’état sur la pile
        superviseur, et continue au gestionnaire dont l’adresse se trouve dans le vecteur
        32 + n. Les simulateurs et les systèmes d’exploitation s’en servent pour leurs
        services ; EASy68K, par exemple, fournit ses entrées et sorties par TRAP #15. Les
        indicateurs ne changent pas, et l’instruction prend 34 cycles d’horloge.

isa-m68k-trapv = Déclenche une exception si l’indicateur de débordement est à 1.
    .details = Syntaxe : TRAPV, sans opérande. Si V vaut 1, le processeur déclenche l’exception
        TRAPV (vecteur 7) ; sinon il passe à l’instruction suivante, aussi vite qu’avec un NOP.
        Placée après une opération signée, elle intercepte un débordement. Aucun indicateur
        n’est modifié.

isa-m68k-tst = Compare l’opérande à zéro.
    .details = Syntaxe : TST.B/W/L <ea> ; sur le 68000, l’opérande est un registre de données ou
        un opérande mémoire modifiable. L’opérande ne change pas : N et Z décrivent sa valeur, V
        et C sont mis à 0 et X ne change pas, pour qu’un branchement conditionnel comme BEQ, BMI
        ou BPL puisse suivre.

isa-m68k-unlk = Supprime un cadre de pile créé par LINK.
    .details = Syntaxe : UNLK An, avec le registre du LINK correspondant. Le pointeur de pile
        reçoit la valeur de An, ce qui libère les variables locales, puis l’ancienne valeur de
        An est dépilée dans An. Elle précède en général immédiatement RTS. Aucun indicateur n’est
        modifié, et l’instruction prend 12 cycles d’horloge.

## Conditions (cc in Bcc, DBcc and Scc; after CMP src,dst they compare dst with src)

isa-m68k-cond-t = toujours vraie
isa-m68k-cond-f = toujours fausse
isa-m68k-cond-hi = supérieur : dst est plus grand que src en nombres non signés
isa-m68k-cond-ls = inférieur ou égal : dst est plus petit que src ou égal, en nombres non signés
isa-m68k-cond-cc = retenue à 0, aussi écrite HS (supérieur ou égal) : dst est plus grand que src ou égal, en nombres non signés
isa-m68k-cond-cs = retenue à 1, aussi écrite LO (inférieur) : dst est plus petit que src en nombres non signés
isa-m68k-cond-ne = différent : dst et src diffèrent, ou le résultat n’est pas nul
isa-m68k-cond-eq = égal : dst et src sont égaux, ou le résultat est nul
isa-m68k-cond-vc = pas de débordement : aucun débordement signé ne s’est produit
isa-m68k-cond-vs = débordement : un débordement signé s’est produit
isa-m68k-cond-pl = positif : le résultat est positif ou nul
isa-m68k-cond-mi = négatif : le résultat est négatif
isa-m68k-cond-ge = supérieur ou égal : dst est plus grand que src ou égal, en nombres signés
isa-m68k-cond-lt = inférieur : dst est plus petit que src en nombres signés
isa-m68k-cond-gt = supérieur strict : dst est plus grand que src en nombres signés
isa-m68k-cond-le = inférieur ou égal : dst est plus petit que src ou égal, en nombres signés

## Timing outcomes

isa-m68k-outcome-taken = branchement pris
isa-m68k-outcome-not-taken = branchement non pris
isa-m68k-outcome-true = condition vraie
isa-m68k-outcome-false = condition fausse
isa-m68k-outcome-not-expired = condition fausse, compteur non épuisé : la boucle continue
isa-m68k-outcome-expired = condition fausse, compteur épuisé : la boucle se termine
isa-m68k-outcome-trap = exception déclenchée
isa-m68k-outcome-no-trap = pas d’exception

## Timing units (n in the per-unit costs)

isa-m68k-unit-shift-count = positions de décalage ou de rotation
isa-m68k-unit-register-count = registres transférés
isa-m68k-unit-one-bits = bits à 1 dans le mot source
isa-m68k-unit-bit-pairs = paires de bits voisins différents dans le mot source, complété par un 0 à droite
