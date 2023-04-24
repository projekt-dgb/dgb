# dgb

Desktop-Client zur Digitalisierung von Grundbuchdaten aus PDF-Dateien

## Installation

Hinweis: Für Windows 7, 8 und 10 wird zusätzlich die Microsoft WebView2 Runtime benötigt, welche Sie [hier](https://developer.microsoft.com/de-de/microsoft-edge/webview2/#download-section) herunterladen können.

- [Windows x64 (.msi)]()
- [Windows x64 (.exe)]()
- [Linux x64 (.deb)]()
- [macOS x64 (.dmg)]()

## Anwendung

Um ein Grundbuchblatt von der PDF-Form in eine JSON-Form zu digitalisieren, öffnen Sie das Grundbuch über "Start > Grundbuch laden".
Die Seiten werden danach mittels Texterkennung analysiert, mittels Rechtsklick auf die Seite könnnen Sie den Seitentyp (das Formular)
ändern, falls es falsch erkannt wurde. Über "Start > Grundbuch neu laden" kann das aktuelle Grundbuch neu analysiert werden.

![Screenshot](https://user-images.githubusercontent.com/12084016/233946067-40ec2384-742f-49fa-afab-1ee079581442.png)

Spalten können mit der Maus an den Ecken verändert werden, Zeilen mit Rechtsklick gesetzt, mit Mittel- oder Linksklick gelöscht werden.
Die LEFIS-Analyse kann mit Klick auf die blauen Pfeile neben "LEFIS" ein- oder ausgeblendet werden. Einstellungen zum Digitalisieren sowie
Skripte zum Analysieren können über "Start > Einstellungen bearbeiten" angepasst werden.

![Screenshot](https://user-images.githubusercontent.com/12084016/233946936-377549c5-18ff-4908-8333-1e94e131e69c.png)

Wenn die Digitalisierung und Überprüfung abgeschlossen ist, können Änderungen zu dem Grundbuch-Server hochgeladen werden. 
Hierfür benötigen Sie ein Konto (E-Mail und Passwort) sowie einen privaten Schlüssel, den Sie über "Start > Einstellungen bearbeiten" laden können. 
Dieser Schlüssel kann Ihnen von einem Administrator eingerichtet werden und dient zum digitalen Unterschreiben der Änderungen am Grundbuch.

![Screenshot](https://user-images.githubusercontent.com/12084016/233946237-43fc8dd7-9efa-4e69-81e8-d518ab92f8a1.png)

Mehr Hilfe und Informationen finden Sie über die eingebaute Hilfe ("Start > Hilfe").

## Lizenz, Hilfe und Support

Copyright 2022 - 2023 Felix Schütt, lizensiert unter der GPL-3.0
