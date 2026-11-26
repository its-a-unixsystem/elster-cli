# KIECLI

> KIECLI is an independent open-source project. It is not affiliated with, endorsed by or supported by ELSTER, the Bayerisches Landesamt für Steuern or any tax authority. ELSTER® is a registered trademark of the Freistaat Bayern.

[🇬🇧 English](https://github.com/its-a-unixsystem/kiecli/blob/main/README.md) | 🇩🇪 Deutsch

Bei Abweichungen gilt die englische Fassung.

KIECLI ist ein Kommandozeilenprogramm für KI-Agenten. Es liest das ELSTER-Postfach, holt Datenpakete und Anhänge ab und fragt das Steuerkonto ab. Jeder Aufruf gilt für einen Steuerpflichtigen. KIECLI fragt nicht interaktiv nach Eingaben und führt kein eigenes Archiv. Die Standardausgabe ist JSON; `--format text` gibt lesbaren Text aus.

## Installation

KIECLI läuft unter Linux auf x86_64. Für das [Release-Archiv](https://github.com/its-a-unixsystem/kiecli/releases) sind glibc ab 2.25 und `libgcc_s.so.1` nötig. Die [Kurzanleitung](https://github.com/its-a-unixsystem/kiecli/blob/main/docs/getting-started.de.md) erklärt die Installation aus dem Archiv, per `cargo binstall` oder per `cargo install` und die anschließende Einrichtung von ERiC. Vor Version 1.0 kann sich das JSON-Format mit einer neuen Minor-Version inkompatibel ändern.

## Befehle

- `eric install` installiert ERiC nach Zustimmung zu den Lizenzbedingungen. `eric status` prüft die Installation ohne Netzwerkzugriff.
- `config show` zeigt Einstellungen und deren Herkunft, ohne das Zertifikat zu öffnen oder die PIN zu lesen.
- `zertifikat status` prüft mit der PIN die Gültigkeit und zeigt die Konto-ID. Der Aufruf sendet keine Anfrage an ELSTER.
- `postfach status` fragt Anzahlen ab, `postfach anfrage` listet Metadaten und IDs auf. Verwenden Sie `--alle` für bereits bestätigte Nachrichten. `--von` und `--bis` begrenzen den Zeitraum.
- `postfach abholung --id ID --out DIR --bestaetigen` schreibt Datenpaket und Anhänge ins Zielverzeichnis und bestätigt den Empfang erst, wenn alle Dateien der Nachricht vorliegen. Mit `--ohne-bestaetigung` verschieben Sie die Bestätigung. Rufen Sie dann `postfach bestaetigung --id ID` innerhalb der ausgegebenen Frist auf, auch wenn Sie die Abholung bis dahin nicht abschließen konnten. Andernfalls kann ELSTER die Hersteller-ID sperren.
- `kontoabfrage offene-betraege|zahlungen|sollstellungen` fragt offene Beträge, Zahlungen oder Sollstellungen ab. Für Zahlungen ist `--ab`, für Sollstellungen sind `--steuerart` und `--jahr` erforderlich.

Postfach und Kontoabfrage benötigen ein Zertifikat, eine PIN-Quelle und eine eigene Hersteller-ID. Mit einem Testzertifikat müssen Sie `--test` angeben. Ohne `--test` gehen Anfragen an das Produktivsystem.

`kiecli skill` gibt die Anleitung für Agenten aus; `kiecli describe` liefert die maschinenlesbare Befehlsreferenz samt JSON-Schemas. Beide Befehle funktionieren ohne ERiC und ohne Zugangsdaten.

## Lizenz

`AGPL-3.0-only` ([LICENSE](LICENSE)) mit einer [zusätzlichen Erlaubnis](NOTICE), KIECLI mit ERiC und Otto in der von ELSTER veröffentlichten Form zu kombinieren.
