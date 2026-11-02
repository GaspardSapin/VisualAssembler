# Problems found in the configuration files, English (m4-editor-shell.md §4.2, §4.6).
#
# One message diag-<code> per entry of CONFIG_CODES (the cfg-* codes), receiving the arguments
# of the §4.2 table, with a .help where the fix is not obvious. Same ids, attributes and
# variables in locales/fr/config.ftl. Written in our own words.
#
# $expected of diag-cfg-wrong-type is a type id: boolean, integer, number, string, array or
# table. $expected of diag-cfg-invalid-value is the list of accepted values (such as
# "auto, en, fr"), or font-file or file-name for the two path rules. $error (an I/O error kind)
# and $detail (the TOML parser's text) are English in every language. Typographic quotes and
# apostrophes only, except in TOML examples, which keep the straight double quotes of TOML.

diag-cfg-unreadable = this configuration file cannot be read ({ $error }); the default values apply
    .help = Check that the path names a regular file that your account may read, then restart VisualAssembler or come back to its window.

diag-cfg-too-large = this configuration file holds { $bytes } bytes, more than the 1 MiB limit; it is ignored
    .help = A configuration file needs only a few lines. Check that this is the right file, or start again from an empty one.

diag-cfg-not-utf8 = this configuration file is not UTF-8 text (invalid byte at offset { $offset }); it is ignored
    .help = Save the file again with the UTF-8 encoding; most text editors offer it in their “Save as” dialog.

diag-cfg-syntax = this file is not valid TOML ({ $detail }); the whole file is ignored
    .help = Fix the place shown. Each setting is written key = value under the [table] header it belongs to, and text values go between double quotes, such as theme = "dark".

diag-cfg-schema-version = schema_version { $found } is not supported; the whole file is ignored
    .help = This version of VisualAssembler reads schema_version = 1. Write 1, or remove the line, which means 1.

diag-cfg-unknown-key = unknown key “{ $key }”; it is ignored
    .help = Check its spelling and the [table] header it is written under. Keys are lowercase, with “_” between words.

diag-cfg-wrong-type = { $key } must be { $expected ->
        [boolean] true or false
        [integer] a whole number
        [number] a number
        [string] a text between double quotes
        [array] a list between square brackets
        [table] a table
       *[other] a value of type { $expected }
    }; this value is ignored
    .help = { $expected ->
        [boolean] Write true or false, without quotes.
        [integer] Write the number without quotes and without a decimal point, such as 300.
        [number] Write the number without quotes, such as 14 or 14.5.
        [string] Put the text between double quotes, such as "dark".
        [array] Put the values between square brackets, separated by commas, such as [".git", "build"].
        [table] Write it under its own [table] header line.
       *[other] Write a value of the expected type.
    }

diag-cfg-out-of-range = { $key } must be between { $min } and { $max }; this value is ignored
    .help = Choose a value from { $min } to { $max }, both included.

diag-cfg-invalid-value = “{ $value }” is not a valid value for { $key }; it is ignored
    .help = { $expected ->
        [font-file] Give the path of a .ttf or .otf font file, absolute or relative to the settings folder, or leave the text empty to keep the built-in font.
        [file-name] Give a single file or folder name, not empty and without “/” or “\”, such as “.git” or “build”.
       *[other] The accepted values are: { $expected }. Write one of them in lowercase, between double quotes.
    }

diag-cfg-unknown-command = unknown command “{ $command }”; this binding is ignored
    .help = Check the spelling of the command id, such as file.save or palette.commands. To remove a default binding, write its id after a “-”, such as "-edit.select-next-occurrence".

diag-cfg-bad-key = “{ $key }” is not a key combination that can be bound; this binding is ignored
    .help = Write the modifiers Ctrl, Shift and Alt, then one key, joined by “+”, such as Ctrl+Shift+P or Alt+F5. The arrows, Home, End, Page Up, Page Down, Backspace, Delete, Enter, Tab, Escape and the copy, cut and paste shortcuts stay with the editor and cannot be bound.

diag-cfg-bad-path = “{ $value }” cannot be used for { $key }: the path must stay inside the project folder; it is ignored
    .help = Write a path relative to the project folder, without “..”, without a drive and without a leading “/”, such as “src/main.asm” or “inc”.
