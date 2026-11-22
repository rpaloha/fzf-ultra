# fzf-ultra — stan na 2026-11-22

Projekt jest wczesnym prototypem lekkiego launchera aplikacji dla GTK4/Wayland, z daemonem działającym w tle.

## Co działa

- Wyszukiwanie fuzzy aplikacji z wpisów `.desktop`, ikony i uruchamianie zaznaczonej aplikacji.
- GUI GTK4 z warstwą Wayland, motywem Graphite i obsługą klawiatury.
- Demon z cache aplikacji oraz lokalnym socketem; `show`, `hide`, `toggle`.
- Tryb filtrowania stdin: `fzf-ultra --filter QUERY`.
- Błąd cyklu życia UI został naprawiony: daemon utrzymuje GUI przy życiu.
- Binarka zbudowana i zainstalowana w `~/.local/bin/fzf-ultra`.
- Usługa `fzf-ultra.service` jest włączona i działa w sesji użytkownika.
- Panel ma 460 px szerokości i przewijaną listę o wysokości do 280 px. Panel jest osobną warstwą Wayland, wyśrodkowaną na monitorze. Położenie sprawdzono na DP-3.
- Tryb `--dmenu --placeholder TEKST` wyświetla listę ze stdin w tym samym demonie i zwraca wybraną pozycję na stdout.
- Demon reaguje na polecenia z gniazda bez cyklicznego odpytywania GTK.

## Stan migracji

Nowa binarka jest zainstalowana w `~/.local/bin/fzf-ultra`. MangoWC ma aktywne
skróty `Super+Space` (aplikacje), `Alt+M` (radio) i `Alt+V` (filmy).
Skrypty `fzf-ultra-radio` i `fzf-ultra-videos` używają istniejących ustawień
radia i mpv. Lista stacji jest w `~/.config/fzf-ultra/stations.csv`.
Pakiety i usługi Walkera oraz Elephanta usunięto. Pozostałe pliki użytkownika przeniesiono do Kosza.

Sprawdzono kompilację, filtrowanie i anulowanie menu. Po wdrożeniu
wywołano z MangoWC polecenia wszystkich trzech skrótów i potwierdzono
otwieranie okna. W ramach tej weryfikacji nie uruchamiano aplikacji ani odtwarzania.
