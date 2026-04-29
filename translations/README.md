This directory contains instructions that you may follow if you are willing to help translating SongRec, including its GUI as well as its command line specification.

## How to translate the interface?

1. Connect using your Github or Gitlab account at **https://weblate.fossplant.re/accounts/login/** (you can also register with an e-mail)
2. Use the Weblate interface at https://weblate.fossplant.re/projects/songrec/songrec/

You Github account should appear and be listed in the [contributors](https://github.com/marin-m/SongRec/graphs/contributors) if you use an e-mail you also use here.

Alternatively and if you are easy with using git, you can also [clone the repository](https://docs.github.com/en/repositories/creating-and-managing-repositories/cloning-a-repository), edit the `.po` files in the `translations/locale/` folder using a tool such as [`poedit`](https://poedit.net/download) and [submit a pull request](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/creating-a-pull-request).

## How to rebuild SongRec while taking in account your translation?

1. Download a `.po` file for your translation using the Weblate interface, for this, go to your your language and select "Files > Download translation". You should obtain a `.po` files
1. Download the SongRec source code, either cloning the repository (using for example the `git clone git@github.com:marin-m/SongRec.git` command in your terminal, or directly [downloading the Zipball](https://github.com/marin-m/SongRec/archive/refs/heads/main.zip) from Github and extracting it to your hard disk).
2. Save your `.po` file under the `translations/locale/fr_FR/LC_MESSAGES/songrec.po` file of the repository, replacing `fr_FR` with the locale code of your language, and creating the intermediary directories as needed.
3. Run the `./translations/update_po_files.sh` script that should generate a binary `.mo` file from the text `.po` file that you coped at the previous steps. Before that, you should install the dependencies of the concerned script (through the `sudo apt install gettext` command), and edit the script to include the locale code of your language within the lines starting with `for locale in` [...].
4. Follow the [compilation instructions](https://github.com/marin-m/SongRec#compilation), until the `cargo run` command which should launch SongRec while taking in account your translation.

## How to make a good translation?

* Don't force yourself to make an exact translation, when using more free terms will sound better or more natural.
* If you don't understand what does a string to translate mean exactly, look at the source code.
* If looking at the source code is not helpful because you don't understand Rust, you may want to ask about it on the [translation Github issue](https://github.com/marin-m/SongRec/issues/23)
* Generally, try to look at the interface of the program and think whether the string you are willing to translate will fit well in the interface after having been translated. Also just launching the command with the `-h` flags should display all what is needed for translating the command line, except for a few rare or unlikely errors.
* You may also want to check the source/ask about it/check the interface when you have a doubt about the grammatical meaning of something, for example if you don't know whether something is a noun or a verb.

### Examples of tricky strings to translate

* `The data-URI Shazam fingerprint to recognize.`: Data-URI is a data format (you may precise that this is a format, etc.).
* `Invalid sample rate in decoded Shazam packet`: "Sample rate", like "Frequency band", is a technical signal processing term that is used in unlikely errors. You may want to find an equivalent translation for your langage, if there has been academic or didactic work using it at all.
* The presence in `_` in a string (in `_Open` or `_Cancel`) means that there is an `Alt-` shortcut on the letter which is present after the underscore. This is a standard thing for GTK+-based programs.
* `Song recognized`: This is the text from the GNOME notification when a song is recognized. I translated it to "Song identified" in French.
* `Recognition results`: This is the text for the title from the bottom-left frame of the GUI. I also translated it to "Identified song" in French because it sounded more natural.
* `Recognize songs`: This is the text for the title from the top-left frame of the GUI.
* For the command-line help, I used the indicative present time rather than infinitive (in terms of French grammar) to match the English present, because it sounded better.
