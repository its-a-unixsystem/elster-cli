# Kurzanleitung

[🇬🇧 English](getting-started.md) | 🇩🇪 Deutsch

Bei Abweichungen gilt die englische Fassung.

## KIECLI installieren

KIECLI läuft auf Linux x86_64. Das Release-Archiv setzt glibc 2.25 oder neuer voraus. Installieren Sie `libgcc_s.so.1` aus Ihrer Distribution (`libgcc-s1` unter Debian und Ubuntu, `libgcc` unter Fedora und Arch). Die Bibliothek ist nicht im Archiv enthalten.

Laden Sie das Archiv `kiecli-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz` und `SHA256SUMS` aus demselben [GitHub Release](https://github.com/its-a-unixsystem/kiecli/releases). Prüfen Sie im Download-Verzeichnis die Prüfsumme oder die Build-Attestierung. Setzen Sie `archive` auf den Namen der heruntergeladenen Datei:

```sh
archive=kiecli-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz
read -r digest _ < SHA256SUMS
printf '%s  %s\n' "$digest" "$archive" | sha256sum --check --strict
```

Alternativ prüfen Sie die Build-Attestierung mit der GitHub CLI: `gh attestation verify "$archive" --repo its-a-unixsystem/kiecli`.

Nach erfolgreicher Prüfung entpacken Sie das Archiv und installieren die ausführbare Datei in einem Verzeichnis auf Ihrem `PATH`:

```sh
tar -xzf "$archive"
install -D -m 755 kiecli "$HOME/.local/bin/kiecli"
```

Mit [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) können Sie `cargo binstall kiecli` verwenden. Wenn Sie KIECLI selbst gegen die glibc Ihres Systems bauen möchten, nutzen Sie `cargo install --locked kiecli` oder in einem Checkout `cargo install --locked --path .`.

## ERiC installieren

Lesen Sie `lizenz.pdf` im [offiziellen ERiC-Paket](https://download.elster.de/download/eric/eric_44/ERiC-44.3.6.0-Linux-x86_64.jar) und die [Datenschutzhinweise der Steuerverwaltung](https://www.elster.de/eportal/helpGlobal?themaGlobal=datenschutz_public#ds_steuerverw). Wenn Sie die Bedingungen akzeptieren, führen Sie diese Befehle aus:

```sh
kiecli eric install --accept-eric-terms
kiecli eric status
```

Die Statusausgabe sollte `"test_load": true` enthalten. Für diese Schritte benötigen Sie weder ein Zertifikat noch eine Hersteller-ID.

Der Installer prüft die hinterlegte SHA-256-Prüfsumme und lädt ERiC und Otto zur Probe, bevor er die neue Version aktiviert. Ältere Installationen bleiben unter `versions/` erhalten. Schlägt die Installation fehl, ändert sich die aktive Version nicht. Für einen anderen Patchstand einer unterstützten Hauptversion geben Sie `--version` und eine vertrauenswürdige Prüfsumme mit `--sha256` an.

Wenn Sie das offizielle Paket selbst entpacken, lesen Sie die Lizenz- und Datenschutzhinweise und legen Sie neben `lizenz.pdf` die Datei `.kiecli-eric-terms-accepted` an. Geben Sie dieses Verzeichnis mit `--eric-dir` oder `KIECLI_ERIC_DIR` an.

## Zugang für einen Steuerpflichtigen einrichten

Für Postfach und Kontoabfrage brauchen Sie Ihre eigene Hersteller-ID. [Registrieren Sie sich für den ELSTER-Entwicklerbereich](https://www.elster.de/elsterweb/registrierung-entwickler/form). Sobald Sie Zugang haben, [beantragen Sie die Hersteller-ID](https://www.elster.de/elsterweb/entwickler/antrag-hersteller-id/form). Geben Sie den Produktnamen und die Schnittstellen ERiC und Sonstige an. Rechnen Sie mit einigen Tagen für den Zugang und einer Woche oder mehr für die Hersteller-ID. ELSTER kann Produktnamen mit „Elster“ ablehnen.

Hinterlegen Sie das `.pfx`-Zertifikat und eine PIN-Quelle, die ohne Rückfrage funktioniert. Beispiel für `$XDG_CONFIG_HOME/kiecli/config.toml` (Standard: `~/.config/kiecli/config.toml`):

```toml
[profile.firma]
certificate = "~/elster/firma.pfx"
pin_file = "~/elster/firma.pin"
```

Tragen Sie `hersteller_id` mit Ihrer fünfstelligen Hersteller-ID oberhalb der Profiltabelle ein oder setzen Sie beim Aufruf `KIECLI_HERSTELLER_ID`. Schränken Sie die Leserechte der PIN-Datei auf Ihr Benutzerkonto ein. Weitere PIN-Quellen und die Rangfolge der Einstellungen stehen in der [Konfigurationsreferenz](../.scratch/kiecli/spec.md#4-configuration-and-secrets).

## Zertifikat prüfen

```sh
kiecli zertifikat status --profile firma
```

KIECLI öffnet das Zertifikat lokal und sendet nichts an ELSTER. Prüfen Sie `abgelaufen` und `testzertifikat` in der Ausgabe.

## ELSTER-Testsystem ausprobieren

Legen Sie für ein offizielles ELSTER-Testzertifikat und dessen PIN-Quelle ein separates Profil an. Verwenden Sie Ihre eigene Hersteller-ID. Der folgende Aufruf fragt nur Anzahlen ab:

```sh
kiecli postfach status --profile test --test --von "$(date -u -d yesterday +%F)"
```

Prüfen Sie, ob die Antwort `"test": true` enthält. Viele Hersteller teilen sich das Testpostfach. Halten Sie den Zeitraum klein und bestätigen Sie keine fremden Nachrichten. Ohne `--test` gehen Anfragen an das Produktivsystem. `kiecli skill` beschreibt die Abläufe für Postfach und Kontoabfrage.
