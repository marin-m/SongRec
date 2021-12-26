This directory contains instructions that you may follow if you are willing to help translating SongRec, including its GUI as well as its command line specification.

## How to translate the interface?

1. Install `poedit`, available in the repositories of your favorite Linux distribution
2. Download and save the [`songrec.pot`](https://github.com/marin-m/SongRec/raw/master/translations/songrec.pot) file which is the base for translating the interface
3. Import it into `poedit` and translate (see below for advice)
4. Save it as a `.po` file (`.pot` files being templates for `.po` files, which are the translations) bearing the name of your langage, and submit it as an attachment to a comment in the following Github issue: https://github.com/marin-m/SongRec/issues/23

## How to rebuild SongRec while taking in account your translation?

1. Download the SongRec source code, either cloning the repository (using for example the `git clone git@github.com:marin-m/SongRec.git` command in your terminal, or directly [downloading the Zipball](https://github.com/marin-m/SongRec/archive/refs/heads/master.zip) from Github and extracting it to your hard disk).
2. Save your `.po` file under the `translations/fr_FR/LC_MESSAGES/songrec.po` file of the repository, replacing `fr_FR` with the locale code of your language (type `echo $LANG` in a terminal in order to know about it, dismissing the final `.UTF8`), and creating the intermediary directories as needed.
3. Run the `./translations/update_po_files.sh` script that should generate a binary `.mo` file from the text `.po` file that you coped at the previous steps. Before that, you should install the dependencies of the concerned script (through the `sudo apt install intltool gettext` command), and edit the script to include the locale code of your language within the lines starting with `for locale in` [...].
4. Follow the [compilation instructions](https://github.com/marin-m/SongRec#compilation), until the `cargo run` command which should launch SongRec while taking in account your translation.

## How to make a good translation?

* Don't force yourself to make an exact translation, when using more free terms will sound better or more natural.
* If you don't understand what does a string to translate mean exactly, look at the source code.
* If looking at the source code is not helpful because you don't understand Rust, you may want to ask about it on the [translation Github issue](https://github.com/marin-m/SongRec/issues/23)
* Generally, try to look at the interface of the program and think whether the string you are willing to translate will fit well in the interface after having been translated. Also just launching the command with the `-h` flags should display all what is needed for translating the command line, except for a few rare or unlikely errors.
* You may also want to check the source/ask about it/check the interface when you have a doubt about the grammatical meaning of something, for example if you don't know whether something is a noun or a verb.

### Examples of tricky strings to translate

* `Note: Could not parse TSV output from`: TSV is a data format, literally meaning "tab-separated values". This string is displayed just before a Linux command, displayed between quotes, that invokes a PulseAudio-related utility (you may precise that this is a command, etc.).
* `Application::new failed`: This is the name of a function from the code (you may use something like "did not work").
* `The data-URI Shazam fingerprint to recognize.`: Data-URI is a data format (you may precise that this is a format, etc.).
* `Invalid sample rate in decoded Shazam packet`: "Sample rate", like "Frequency band", is a technical signal processing term that is used in unlikely errors. You may want to find an equivalent translation for your langage, if there has been academic or didactic work using it at all.
* `Recognize from my speakers instead of microphone`: Don't make it extremely long, otherwise the interface may horizontally overflow (I limited to 62 characters for French).
* `Failed to get default input config`: Unlikely error message. It's about obtaining the configuration regarding the default microphone of the system, when obtained with an audio system such as ALSA or PulseAudio.
* The presence in `_` in a string (in `_Open` or `_Cancel`) means that there is an `Alt-` shortcut on the letter which is present after the underscore. This is a standard thing for GTK+-based programs.
* `Song recognized`: This is the text from the GNOME notification when a song is recognized. I translated it to "Song identified" in French.
* `Recognition results`: This is the text for the title from the bottom-left frame of the GUI. I also translated it to "Identified song" in French because it sounded more natural.
* `Recognize songs`: This is the text for the title from the top-left frame of the GUI.
* For the command-line help, I used the indicative present time rather than infinitive (in terms of French grammar) to match the English present, because it sounded better.
