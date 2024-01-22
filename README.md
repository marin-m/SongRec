# SongRec

SongRec is an open-source Shazam client for Linux, written in Rust.

![Screenshot](packaging/Screenshot.png?raw=true)

Features:

* Recognize audio from an arbitrary audio file.
* Recognize audio from the microphone.
* Usage from both GUI and command line (for the file recognition part).
* Provide an history of the recognized songs on the GUI, exportable to CSV.
* Continuous song detection from the microphone, with the ability to choose your input device.
* Ability to recognize songs from your speakers rather than your microphone (on compatible PulseAudio setups).
* Generate a lure from a song that, when played, will fool Shazam into thinking that it is the concerned song.

A (command-line only) Python version, which I made before rewriting in Rust for performance, is also available for demonstration purposes. It supports file recognition only.

## How it works

For useful information about how audio fingerprinting works, you may want to read [this article](http://coding-geek.com/how-shazam-works/). To be put simply, Shazam generates a spectrogram (a time/frequency 2D graph of the sound, with amplitude at intersections) of the sound, and maps out the frequency peaks from it (which should match key points of the harmonics of voice or of certains instruments).

Shazam also downsamples the sound at 16 KHz before processing, and cuts the sound in four bands of 250-520 Hz, 520-1450 Hz, 1450-3500 Hz, 3500-5500 Hz (so that if a band is too much scrambled by noise, recognition from other bands may apply). The frequency peaks are then sent to the servers, which subsequently look up the strongest peaks in a database, in order look for the simultaneous presence of neighboring peaks both in the associated reference fingerprints and in the fingerprint we sent.

Hence, the Shazam fingerprinting algorithm, as implemented by the client, is fairly simple, as much of the processing is done server-side. The general functionment of Shazam has been documented in public [research papers](https://www.ee.columbia.edu/~dpwe/papers/Wang03-shazam.pdf) and patents.

## Installation

Here are a few ways to install and run the application:

Using pacman (Arch Linux):

```bash
sudo pacman -S songrec
songrec
```

Using apt with PPA (Ubuntu, supported 18.04, 20.04, 22.04, 23.04, 23.10):

```bash
wget -qO- 'http://keyserver.ubuntu.com/pks/lookup?op=get&search=0x6888550b2fc77d09' | sudo tee /etc/apt/trusted.gpg.d/songrec.asc
sudo apt-add-repository ppa:marin-m/songrec -y -u
sudo apt install songrec -y
songrec
```

Using Flatpak (all distributions) (NOTE: with Flatpak, the GUI should work fine but some of the CLI features may not be usable due to filesystem sandboxing restrictions):

```bash
sudo apt install flatpak -y
flatpak remote-add --user flathub https://flathub.org/repo/flathub.flatpakrepo --if-not-exists
flatpak install --user flathub com.github.marinm.songrec -y
flatpak run com.github.marinm.songrec
```

Using Cargo (all distributions, dependencies given for Ubuntu/Debian, if your `rustc` version is not recent enough please refer to the instructions below):

```bash
sudo apt install cargo rustc -y
echo 'export PATH="$HOME/.cargo/bin:$PATH"' | tee -a ~/.profile ~/.bashrc
source ~/.bashrc

sudo apt install build-essential libasound2-dev libgtk-3-dev libssl-dev -y
cargo install songrec --no-default-features -F gui,ffmpeg,pulse,mpris
songrec
```

Note: It is not mandatory, but if you want to be able to recognize more formats than WAV, OGG, FLAC and MP3, you should ensure that you have the `ffmpeg` package installed.

Note: You may remove dependencies over GTK+, Pulseaudio/PipeWire's libpulse or DBus MPRIS through editing the `-F` flag passed to `cargo`.

## Compilation

(**WARNING**: Remind to compile the code in "--release" mode for correct performance.)

### Installing Rust

First, you need to [install the Rust compiler and package manager](https://www.rust-lang.org/tools/install). It has been observed to work with `rustc` since version 1.43.0.

You can either install Rust from the repositories, for example using Ubuntu/Debian:

```bash
sudo apt install rustc cargo
```

Or under Fedora Linux:

```bash
sudo dnf install rustc cargo
```

Or, using any distribution:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # Type "1"
# Login and reconnect to add Rust to the $PATH, or run:
source $HOME/.cargo/env

# If you already installed Rust, then update it:
rustup update
```

### Install dependent libraries (nothing exotic)

Debian:

```bash
sudo apt install build-essential libasound2-dev libpulse-dev libgtk-3-dev libssl-dev
```

Void Linux (libressl):

```shell
sudo xbps-install base-devel alsa-lib-devel pulseaudio-devel gtk+3-devel libressl-devel
```

Void Linux (openssl):

```shell
sudo xbps-install base-devel alsa-lib-devel pulseaudio-devel gtk+3-devel openssl-devel
```

Fedora Linux:

```shell
sudo dnf groupinstall "Development Tools"
sudo dnf install alsa-lib-devel pulseaudio-libs-devel openssl-devel dbus-devel pkgconf-pkg-config glib gtk3-devel
```

### Compiling the project

This will compile and run the projet:

```bash
# For the stable release:
cargo install songrec --no-default-features -F gui,ffmpeg,pulse,mpris
songrec

# For the Github tree:
git clone https://github.com/marin-m/songrec
cd songrec
cargo run --release --no-default-features -F gui,ffmpeg,pulse,mpris
```

For the latter, you will then find the project's binary (that you will be able to move or execute directly) at `target/release/songrec`.

Note: You may remove dependencies over GTK+, Pulseaudio/PipeWire's libpulse or DBus MPRIS through editing the `-F` flag passed to `cargo`.

## Sample usage

Passing no arguments or using the `gui` subcommand will launch the GUI, and try to recognize audio real-time as soon as the application is launched:

```
./songrec
./songrec gui
```

Using the `gui-norecording` subcommand will launch the GUI without recognizing audio as soon as the software is started (you will need to click the "Turn on microphone recognition" button to do so):

```
./songrec gui-norecording
```

The GUI allows you to recognize songs either from your microphone, speakers (on compatible PulseAudio setups), or from an audio file. The MP3, FLAC, WAV and OGG formats should be accepted for audio files if FFMpeg is not installed, and any audio or video formats supported by FFMpeg should be accepted if FFMpeg is installed.

The following commands allow to recognize sound from your microphone or from a file using the command line (`listen` runs while the microphone is usable while `recognize` recognizes only one song), use the `-h` flag in order to see all the available options:

```
./songrec listen -h
./songrec recognize -h
```

By default, only the artist and track name of the concerned song are displayed to the standard output, and other information may be displayed to the error output. The `--csv` and `--json` options allow to display more programmatically usable information to the standard output.

The above decribes the newer CLI interface of SongRec, but an older interface, operating only on audio files or raw audio fingerprints, is also available and described below.

The following subcommand will try to recognize audio from the middle of an audio file, and print the JSON response from Shazam servers:

```
./songrec audio-file-to-recognized-song sound_file.mp3
```

The following subcommands will do the same with an intermediary step, manipulating data-URI audio fingerprints as used by Shazam internally:

```
./songrec audio-file-to-fingerprint sound_file.mp3
./songrec fingerprint-to-recognized-song 'data:audio/vnd.shazam.sig;base64,...'
```

The following will produce back hearable tones from a given fingerprint, that should be able to fool Shazam into thinking that this is the original song (either to the default audio output device, or to a .WAV file):

```
./songrec fingerprint-to-lure 'data:audio/vnd.shazam.sig;base64,...'
./songrec fingerprint-to-lure 'data:audio/vnd.shazam.sig;base64,...' /tmp/output.wav
```

When using the application, you may notice that certain information will be saved to `~/.local/share/songrec` and `~/.config/songrec` (or an equivalent directory depending on your operating system), including the CSV-format list of the last recognized songs and the last selected microphone input device (so that it is chosen back when restarting the app). You may want to delete these directories in case of persistent issues.

## Privacy

SongRec collects no data and contacts no other servers than Shazam's. SongRec does not upload raw audio data anywhere: only fingerprints of the audio are uploaded, which means sequences of frequency peaks encoded in the form of "(frequency, amplitude, time)" tuples.

This does not suffice to represent anything hearable alone (use the "Play a Shazam lure" button to see how much this is different from full sound); that means that no actually hearable sound (e.g voice fragments) is sent to servers, only metadata derived on the characteristics of the sound that may only suffice to recognize a song already known by Shazam is being sent.

## Legal

This software is released under the [GNU GPL v3](https://www.gnu.org/licenses/gpl-3.0.html) license. It was created with the intent of providing interoperability between the remote Shazam services and Linux-based deskop systems.

Please note that in certain countries located outside of the European Union, especially the United States, software patents may apply.

