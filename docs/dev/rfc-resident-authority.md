# RFC: Resident Snapshot-Authority ("jeden ciepły proces prawdy")

- **Status:** DRAFT (W3-00 recon — zero zmian produkcyjnych)
- **Autor:** claude (W3-00, wave kosmos, 2026-07-01)
- **Baseline:** `fix/loctree-old-and-new@e3f3d774`, loct 0.13.0
- **Spike:** `experiments/resident-authority-spike/spike.py` (repro niżej)

## 1. Problem

Cała klasa staleness z fail-logu ma jeden wspólny fundament: każdy surface
(CLI, MCP, LSP) buduje i odświeża własny stan według własnych reguł, a agent
nie ma jednego miejsca, które mówi "to jest aktualna prawda o repo".

Instancje tej klasy (z `~/.vibecrafted/loctree/loctree-fail.md` i briefów W1):

1. **Stale MCP binary** (2026-06-22, ponownie 2026-06-26 CodeScribe):
   długożyjący proces MCP serwuje *starszy kod* niż źródło; wykrywalne dziś
   tylko dzięki BUILD-stampowi z `initialize`
   (`loctree-mcp/src/main.rs:3370`, build.rs `LOCTREE_MCP_GIT_COMMIT`).
2. **Stale per-file cache → `--full-scan`** (memory `reference_loctree_per_file_cache_full_scan`,
   commit f4be8124): fix analyzera nie widać na realnym repo, dopóki nie
   wymusi się pełnego rescanu — "działa isolated, pada w dużym repo".
3. **Stale atlas cards**: karty `.loctree/context-atlas` materializowane per
   wywołanie, mogą przeżyć snapshot, z którego powstały.
4. **Latencja pierwszego zapytania**: `loct context` = 12.1s cold i **12.0s
   warm** (brief W1-06, live probe 2026-07-01); dzisiejszy pomiar tej samej
   ścieżki: **15.0s** (patrz §3). Doktrynalny "pierwszy ruch sesji" kosztuje
   kilkanaście sekund za każdym razem.

Do tego dochodzi fakt, którego framing "wszystko jest zimne" nie oddaje —
i który ten recon ustala jako ground truth:

**Mamy już DWA niezależne rezydenty i jedno zimne CLI, każdy z innym
mechanizmem świeżości:**

| Surface | Stan dziś | Mechanizm świeżości | Dowód |
|---|---|---|---|
| **CLI (`loct`)** | zimny proces per komenda | `Snapshot::load()` z cache per `branch@commit`; staleness sprawdzana ad-hoc per handler | `loctree-rs/src/snapshot.rs:2367` (pełny read + `serde_json::from_str` całego pliku) |
| **MCP (`loctree-mcp`)** | rezydent per sesja agenta | in-RAM `HashMap<PathBuf, Arc<Snapshot>>`; per-call `Snapshot::is_stale()` (git HEAD + dirty worktree) → auto-rescan | `loctree-mcp/src/main.rs:1088` (cache), `:1275-1283` (is_stale delegacja) |
| **LSP (`loctree-lsp`)** | rezydent per edytor / `loct watch --lsp` | fs-watcher (debounce 300ms) → rescan → atomowy swap `Arc<RwLock<Option<LoadedSnapshot>>>`; brak per-call staleness | `loctree-lsp/src/snapshot.rs:20-30`, `loctree-lsp/src/watcher.rs:1-17`, `backend.rs:1669-1689` |

Konsekwencje: (a) dwie kopie ~480 MB RAM, gdy MCP i LSP żyją naraz na tym
samym repo; (b) trzy różne odpowiedzi na "czy snapshot jest świeży" w tym
samym momencie; (c) CLI płaci pełny koszt load+parse przy każdej komendzie;
(d) każdy rezydent może osobno zdriftować binarką (HAK-17).

## 2. Co LSP już ma (najbliższy istniejący kandydat na hosta)

`loctree-lsp` to dziś de facto prototyp resident authority:

- **Pełny snapshot w RAM** — nie tylko live-AST. `SnapshotState` trzyma cały
  `loctree::snapshot::Snapshot` (`loctree-lsp/src/snapshot.rs:25-30`), ładowany
  tym samym `Snapshot::load()` co CLI, w `spawn_blocking` poza event-loopem.
- **15 requestów `loctree/*`** (`loctree-lsp/src/lib.rs:96-110`): refresh,
  contextAtlas, body, symbolContext, contextPack, find, follow, health,
  impact, slice, workspaces, diff, semantic, aicx, astQuery.
- **Watcher świeżości** — notify + debounce, rescan pisze snapshot.json do
  global cache i atomowo podmienia stan w RAM (`watcher.rs` doc-comment,
  design "Plan 10").
- **Pin roota bez klienta** — `loctree-lsp --root <DIR>` adoptuje workspace
  przed (i niezależnie od) `initialize`; już używany przez `loct watch --lsp`
  jako co-proces (`loctree-lsp/src/main.rs:34-43`).
- **Multi-workspace routing** — `discover_and_load_workspaces()` w
  `initialized` (backend.rs:1678).
- **Live-AST na otwartych buforach** (`LiveAstStore`) — warstwa, której
  snapshot-authority nie ma i mieć nie musi, ale która na hoście już jest.

Czego LSP-jako-authority dziś NIE ma: per-call staleness check w stylu MCP
(polega w 100% na watcherze), transportu dla wielu klientów naraz (stdio =
jeden klient), oraz version-handshake'u w stylu BUILD-stampa MCP.

## 3. Spike — liczby, nie opinie

Środowisko: to repo (827 plików, 289k LOC wg manifestu cache), snapshot
`fix_loctree-old-and-new@e3f3d774`, **snapshot.json = 236.8 MB**, binarki
zainstalowane `loct 0.13.0` / `loctree-lsp 0.13.0` (`~/.local/bin`), Apple
Silicon (darwin), page cache ciepły (N powtórzeń pod rząd).

| Pomiar | Wartość | Metoda |
|---|---|---|
| snapshot.json (ten commit) | **236.8 MB** | `ls -la ~/Library/Caches/loctree/projects/63beb6c32331fa44/fix_loctree-old-and-new@e3f3d774/` |
| Baseline procesu: `loct --version` | **4.9 ms** (mediana, N=7) | spike.py |
| **Zimne CLI: `loct slice loctree-rs/src/types.rs`** | **259.8 ms** (mediana, N=7; min 256.0, max 267.3) | spike.py |
| RSS zimnego `loct slice` | **~500 MB** (501,612,544 B max RSS) | `/usr/bin/time -l loct slice …` |
| Referencyjny parse 236.8 MB (python `json.loads`) | read 0.13 s + parse 0.61 s | inline (repro niżej) |
| **Rezydentny LSP: `loctree/slice` (ten sam plik, warm)** | **0.6 ms** (mediana, N=7; wszystkie próbki 0.6–0.7 ms) | spike.py |
| Cold-start rezydenta (spawn → initialize → initialized → pierwszy udany slice) | **423.5 ms** | spike.py |
| RSS rezydentnego loctree-lsp (po załadowaniu snapshotu) | **481.8 MB** | `ps -o rss=` na żywym procesie |
| `loct context` (atlas, ta sama sesja) | **15.0 s real** (4.2 user + 5.3 sys), RSS 559 MB | `/usr/bin/time -l loct context` |
| `loct health` (zimny, pełny load) | 0.46 s, RSS 512 MB | `/usr/bin/time -l loct health` |

**Wnioski z liczb:**

1. **Rezydent vs zimne CLI na tym samym zapytaniu: ~433× (259.8 ms → 0.6 ms).**
   Cold-start rezydenta (423 ms) zwraca się po *dwóch* zapytaniach.
2. **RSS ~500 MB potwierdza mechanizm**: zimny `loct slice` naprawdę parsuje
   pełne 236.8 MB JSON do structów przy każdym wywołaniu. 260 ms to nie "mało"
   — to M-series + ciepły page cache maskujące pracę, która na słabszym
   sprzęcie / zimnym FS będzie sekundowa (python-referencja: sam parse 0.61 s).
3. **Prawdziwy pożeracz sesji to nie slice, tylko `context`**: 15 s przy
   świeżym snapshocie (warm==cold, potwierdza tezę briefu W1-06, że pali
   compose/render/overlay, nie scan). Rezydent, który trzyma sparsowany
   snapshot ORAZ zmaterializowany atlas, amortyzuje obie klasy.
4. **Pamięć jest realnym kosztem wariantów rezydentnych**: ~480 MB per proces
   per repo. Dziś przy MCP+LSP naraz płacimy to podwójnie; konsolidacja do
   jednego hosta ten koszt *zmniejsza*, nie zwiększa.

**Repro (komendy, wszystkie uruchomione podczas tego recon):**

```bash
# pełny spike (baseline + cold CLI + resident LSP, mediana N=7):
python3 experiments/resident-authority-spike/spike.py 7

# RSS zimnych ścieżek:
/usr/bin/time -l loct slice loctree-rs/src/types.rs > /dev/null
/usr/bin/time -l loct health > /dev/null
/usr/bin/time -l loct context > /dev/null

# referencyjny koszt parse (python):
python3 -c "
import json,time,os
p=os.path.expanduser('~/Library/Caches/loctree/projects/63beb6c32331fa44/fix_loctree-old-and-new@e3f3d774/snapshot.json')
print('MB:',round(os.path.getsize(p)/1e6,1))
t=time.monotonic(); s=open(p).read(); print('read',round(time.monotonic()-t,3))
t=time.monotonic(); json.loads(s);   print('parse',round(time.monotonic()-t,3))"
```

Uwaga metodologiczna: `resident_lsp_slice_warm` mierzy round-trip JSON-RPC po
stdio do procesu, który ma snapshot w RAM; `cold_cli_slice` mierzy pełny
lifecycle procesu. To jest dokładnie ta różnica, o którą pyta RFC — "co kupuje
rezydencja" — a nie porównanie dwóch implementacji slice (obie liczą slice na
tych samych strukturach `loctree` lib).

### 3.1 Replikacja (refire W3-00, 2026-07-02, HEAD 58be9503)

Drugi, niezależny bieg tego samego spike'a — inna sesja, drzewo z żywym WIP-em
równoległych cutów W1 (7 zmodyfikowanych plików src/), dwa commity dalej:

| Pomiar | Run 1 (e3f3d774) | Run 2 (58be9503) |
|---|---|---|
| Zimne CLI `loct slice` (mediana, N=7) | 259.8 ms | 335.1 ms (322.1–340.6) |
| Rezydentny LSP `loctree/slice` warm | 0.6 ms | 0.8 ms (0.7–0.9) |
| **Przewaga rezydenta** | ~433× | **~419×** |
| Cold-start rezydenta | 423.5 ms | 664.8 ms |
| `loct context` (atlas) | 15.0 s | 14.87 s real, RSS 507 MB |

Absoluty run 2 lekko wyższe (współbieżne obciążenie fleet + świeży rescan po
dwóch commitach); klasa wyników i wszystkie wnioski §3 bez zmian. Rekomendacja
§5 potwierdzona replikacją.

## 4. Macierz decyzyjna

Osie: **latencja** (per zapytanie + pierwsze zapytanie sesji), **spójność**
(jedna prawda między surface'ami), **złożoność operacyjna** (lifecycle,
wersjonowanie, klasa HAK-17), **degradacja** (co się dzieje, gdy rezydent
padł/nie istnieje).

### Wariant A — LSP jako host; CLI i MCP klientami

CLI/MCP próbują najpierw żywego `loctree-lsp --root <repo>` (discovery przez
plik-socket/lock w project cache dir); miss → dzisiejsza zimna ścieżka.

- **Latencja:** 0.6 ms per zapytanie (zmierzone); cold-start hosta 423 ms
  jednorazowo. Atlas/context liczone raz, serwowane z RAM.
- **Spójność:** JEDEN snapshot w RAM + jeden watcher = jedna odpowiedź na
  "czy świeże" dla wszystkich surface'ów. Kanon metryk W1-04 serwowany z
  jednego miejsca.
- **Złożoność:** średnia. Host istnieje (15 requestów, watcher, --root pin,
  multi-workspace). Do zrobienia: discovery+handshake dla klientów CLI/MCP,
  per-call staleness (przeniesienie wzorca MCP `is_stale` do request-path
  LSP), BUILD-stamp handshake (wzorzec z MCP już jest). Ryzyko HAK-17:
  jeden proces do upilnowania zamiast dwóch.
- **Degradacja:** naturalna — klient nie znalazł hosta → zimna ścieżka,
  która działa dziś i pozostaje nietknięta. Brak nowego single point of
  failure: host to akcelerator, nie zależność.
- **Koszt RAM:** ~480 MB × 1 zamiast × 2 (dzisiejsze MCP+LSP równolegle).

### Wariant B — nowy `loctreed` daemon; CLI, MCP i LSP klientami

- **Latencja:** jak A (ten sam mechanizm).
- **Spójność:** jak A, plus czysty rozdział "authority ≠ protokół edytora".
- **Złożoność:** NAJWYŻSZA. Nowy binarny artefakt = nowy członek klasy
  HAK-17 (czwarta binarka do wersjonowania, instalowania, restartowania);
  nowy protokół RPC; migracja LSP na klienta (przepisanie 15 handlerów na
  proxy albo utrzymywanie dwóch ścieżek). Duplikuje to, co LSP już umie.
- **Degradacja:** jak A, ale trzy surface'y zależne od czwartego procesu.
- **Werdykt:** poprawny docelowo-akademicko, nieuzasadniony kosztowo, dopóki
  LSP-host nie udowodni ograniczeń (np. izolacja edytorowego live-AST od
  agentowego ruchu). B pozostaje ścieżką ewolucji A (wydzielenie hosta z
  loctree-lsp do osobnego crate'a), nie punktem startu.

### Wariant C — status quo + watcher odświeżający cache w tle

Osobny `loct watch` pisze świeży snapshot.json; CLI/MCP dalej zimne.

- **Latencja:** BEZ ZMIAN per zapytanie (260 ms slice / 15 s context nadal
  płacone za każdym razem) — nie rozwiązuje problemu, który mierzy spike;
  leczy wyłącznie staleness *danych*.
- **Spójność:** częściowa — dane świeższe, ale nadal N niezależnych parsów
  i N momentów odczytu = smear okien czasowych.
- **Złożoność:** niska (fundament istnieje: `loct watch --lsp` już spawnuje
  LSP jako co-proces).
- **Degradacja:** trywialna.
- **Werdykt:** to nie jest wariant docelowy, tylko opis dzisiejszego
  kierunku dryfu. Odrzucony jako cel; zachowany jako fallback-warstwa w A.

### Wariant D — mmap / zero-copy snapshot bez daemona

Format binarny (np. rkyv/capnp) obok snapshot.json; zimny proces mapuje
plik zamiast parsować 236.8 MB JSON.

- **Latencja:** realny zysk na load+parse (~250 ms → ~ms mapowania +
  lazy-access), ale NIE amortyzuje compose/atlas (15 s context dalej
  liczone per proces) i nie daje watcher-świeżości.
- **Spójność:** żadnej zmiany — nadal N czytelników, N momentów odczytu;
  dochodzi RYZYKO NOWEJ KLASY STALE: drugi format snapshotu obok JSON
  (dwa artefakty do zsynchronizowania per scan) + wersjonowanie layoutu
  binarnego (rkyv nie jest self-describing; bump struktur = niema
  niekompatybilność, dokładnie duch HAK-17 w danych zamiast w binarce).
- **Złożoność:** wysoka w niewidocznym miejscu (schema evolution, ABI
  stabilność między wersjami loct, cross-arch).
- **Degradacja:** n/d (brak procesu), ale patrz ryzyko formatu.
- **Werdykt:** odrzucony jako oś główna. Zasługuje na osobny, mały spike
  JEŚLI po wdrożeniu A zimna ścieżka fallbacku okaże się bolesna na
  słabszym sprzęcie — wtedy jako optymalizacja fallbacku, nie architektura.

### Tabela zbiorcza

| Oś | A: LSP-host | B: loctreed | C: watcher-cache | D: mmap |
|---|---|---|---|---|
| Latencja per zapytanie | ✅ 0.6 ms (zmierzone) | ✅ 0.6 ms | ❌ bez zmian | 🟡 lepszy load, compose bez zmian |
| Pierwsze zapytanie sesji | ✅ amortyzowane (423 ms raz) | ✅ | ❌ 15 s context | 🟡 |
| Jedna prawda (spójność) | ✅ jeden RAM-snapshot + jeden watcher | ✅ | 🟡 dane tak, odczyt nie | ❌ + nowy format do driftu |
| Złożoność operacyjna / HAK-17 | 🟡 1 proces, mechanizmy istnieją | ❌ nowa binarka, nowy protokół | ✅ | ❌ ukryta (ABI/schema) |
| Degradacja bez rezydenta | ✅ dzisiejsza zimna ścieżka | ✅ ale 3 surfaces zależne | ✅ | n/d |
| Dystans od dzisiejszego kodu | ✅ krótki (LSP ma 80%) | ❌ długi | ✅ zerowy | 🟡 średni |

## 5. Rekomendacja

**Wariant A: `loctree-lsp` zostaje hostem snapshot-authority; CLI i MCP
stają się jego klientami z twardym fallbackiem na dzisiejszą zimną ścieżkę.**

Dlaczego nie pozostałe:

- **Nie B**, bo B kupuje dokładnie te same własności co A za cenę nowej
  binarki (nowy członek klasy HAK-17), nowego protokołu i migracji 15
  działających handlerów. Jeśli A kiedyś urośnie w B (wydzielenie hosta do
  crate'a `loctree-authority` używanego przez lsp-binarkę), to będzie
  refactor wewnętrzny, nie zmiana architektury klientów.
- **Nie C**, bo C nie dotyka latencji, którą spike mierzy (260 ms / 15 s
  per proces zostają), a spójność odczytu dalej rozmyta. C i tak żyje
  wewnątrz A jako mechanizm świeżości hosta.
- **Nie D**, bo D optymalizuje wyłącznie faze load+parse, nie rozwiązuje
  spójności ani compose-kosztu, a wprowadza drugi format snapshotu — nową
  powierzchnię driftu dokładnie tej klasy, którą ten RFC ma zamknąć.

Decyzja brzegowa wpisana w rekomendację: **host jest akceleratorem, nigdy
zależnością.** Każdy klient MUSI umieć przeżyć bez hosta (zimna ścieżka
zostaje nietknięta i testowana). To odróżnia A od "daemon-centryzmu" i
utrzymuje degradację jako własność pierwszej klasy.

## 6. Stale-daemon (HAK-17 na sterydach) — mechanizm, nie deklaracja

Rezydent, który serwuje starszy kod niż źródło, jest gorszy niż zimny
proces, bo kłamie szybko i pewnie. Trzy warstwy obrony, wszystkie oparte na
wzorcach już obecnych w repo:

1. **Version handshake na każdym połączeniu klienta.** Host odpowiada w
   handshake'u tym, co MCP już dziś wkłada do `initialize` instructions:
   `BUILD: <semver>+<git describe --always --dirty --tags>` (wzorzec:
   `loctree-mcp/src/main.rs:3342-3370`, stamp z build.rs
   `LOCTREE_MCP_GIT_COMMIT`). Klient (CLI/MCP) porównuje ze SWOIM stampem:
   - stamp klienta == stamp hosta → serwuj z hosta;
   - mismatch → **klient loguje jawnie i spada na zimną ścieżkę** (własny,
     świeży kod), a hostowi wysyła `authority/retire`.
   Zasada: *nowszy klient nigdy nie konsumuje odpowiedzi starszego hosta.*
   To odwraca dzisiejszą sytuację HAK-17: drift przestaje być cichy, bo
   każde połączenie go mierzy.
2. **Self-retire hosta.** Host przy każdym rescanie porównuje własny stamp
   z binarką na dysku (`argv[0]` mtime/hash zapamiętane przy starcie);
   podmiana binarki → host kończy pracę po ostatnim in-flight request
   (`authority: retiring, binary changed on disk`). Restart należy do
   supervisora (launchd/`loct watch`), nie do hosta.
3. **Snapshot handshake (dane, nie tylko kod).** Odpowiedź hosta niesie
   `snapshot_fingerprint` (algorytm `sha256:loctree-snapshot-authority-v1`
   już emitowany w receipt — patrz `context.receipt.snapshot.fingerprint`),
   `branch@commit` i wynik `is_stale`. Klient, który wymaga świeżości
   (`--fail-stale`), egzekwuje ją NA KLIENCIE, niezależnie od tego, co
   twierdzi host — pojedynczy kanon `Snapshot::is_stale()` (loctree lib)
   pozostaje jedynym źródłem tej decyzji po obu stronach.

Degradacja (spisana, testowalna):

```
klient → discovery (lock/socket w project_cache_dir)
  ├─ brak hosta        → zimna ścieżka (dzisiejsza, bez zmian)
  ├─ handshake timeout → zimna ścieżka + log
  ├─ stamp mismatch    → zimna ścieżka + authority/retire do hosta
  └─ OK                → zapytanie do hosta (0.6 ms klasa)
```

## 7. Plan cutów W3-01..0n (wsad do scaffoldu fazy 3)

Każdy cut agent-sized, z verifierem; kolejność honoruje "host = akcelerator".

- **W3-01 — Authority handshake w loctree-lsp.**
  Nowy request `loctree/authority`: zwraca BUILD-stamp (build.rs w
  loctree-lsp analogiczny do loctree-mcp), snapshot fingerprint,
  branch@commit, `is_stale`, uptime. Verifier: test integracyjny
  `loctree-lsp/tests/authority_request.rs` — stamp obecny, fingerprint
  zgodny z receipt snapshotu; `--capabilities` wypisuje request.
- **W3-02 — Discovery: plik kontaktowy hosta w project cache dir.**
  Host przy starcie pisze `authority.json` (pid, transport, stamp) do
  `project_cache_dir(root)`, sprząta przy shutdown; stale-pid detection
  (pid nie żyje → plik ignorowany/kasowany). Verifier: test jednostkowy na
  create/cleanup/stale-pid; brak wpływu na istniejące testy watcher_smoke.
- **W3-03 — Per-call staleness w request-path LSP.**
  Przeniesienie wzorca MCP (`is_stale` przy każdym get_snapshot,
  main.rs:1275) do handlerów LSP: request na stale snapshot → rescan albo
  jawne `stale: true` w odpowiedzi (bez czekania na watcher-debounce).
  Verifier: test — commit w fixture repo między dwoma slice'ami zmienia
  odpowiedź bez eventu watchera.
- **W3-04 — CLI jako klient: `loct slice/impact/find --via-authority` (za
  feature-flagą), potem default-on.**
  `loct` czyta `authority.json`, handshake wg §6, na miss → dzisiejsza
  ścieżka. Verifier: (a) test degradacji — brak hosta → wynik identyczny z
  dzisiejszym (golden diff), (b) spike-rerun: mediana `loct slice`
  z żywym hostem < 20 ms end-to-end (proces CLI + RPC), (c) stamp-mismatch
  → fallback + log.
- **W3-05 — MCP jako klient tego samego hosta.**
  `loctree-mcp` przy get_snapshot próbuje hosta zanim załaduje własną kopię;
  trafienie = zero drugiej kopii 480 MB. Verifier: test — dwa procesy
  (host+mcp) na tym samym repo; RSS MCP < 150 MB przy serwowaniu slice;
  odpowiedzi MCP == odpowiedzi hosta (jedna prawda, kanon W1-04).
- **W3-06 — Self-retire + supervisor happy-path.**
  Mechanizm §6 pkt 2 + integracja `loct watch` jako supervisora (respawn po
  retire). Verifier: test — podmiana binarki (touch/copy) → host kończy się
  czysto; watch respawnuje; klient w międzyczasie NIE dostaje odpowiedzi ze
  starego kodu (fallback na zimno).
- **W3-07 — Atlas/context z hosta.**
  `loct context` przez authority: compose raz po rescanie (lub lazy przy
  pierwszym żądaniu), karty serwowane z RAM/cache z fingerprintem snapshotu
  w nagłówku karty (zamyka "stale atlas cards"). Verifier: spike-rerun —
  warm `loct context` przez hosta < 2 s (cel W1-06) na tym repo; karta
  niesie fingerprint == fingerprint hosta.
- **W3-08 — Fail-log closure + doktryna.**
  Wpisy klasy staleness w loctree-fail.md dostają dyspozycje (resolved-by
  authority / wontfix z powodem); AGENTS.md/docs dostają sekcję "authority
  first, cold path always works". Verifier: przegląd fail-log → zero
  otwartych wpisów tej klasy bez dyspozycji.

Zależności: 01 → 02 → {03, 04} → 05 → 06 → 07 → 08. Cuty 03/04
równoległe (różne pliki: lsp handlers vs cli dispatch).

## 8. Integration points (opisowo — poza scope implementacji)

- **Context-pack do briefów (vibecrafted):** brief-generator odpytuje hosta
  zamiast zimnego `loct context` — sekundy zamiast kilkunastu na każdy brief;
  fingerprint snapshotu wchodzi do frontmattera briefu jako proweniencja.
- **Impact w pre-commit:** hook `loct impact --via-authority` w budżecie
  <50 ms robi blast-radius check przy commicie realnym, nie aspiracyjnym
  (zimne 260 ms+ per plik czyniło to dziś nieakceptowalnym w hookach).
- **Prism-gate (vc-polarize):** `loct prism` liczy pack per task-framing —
  N framingów × zimny compose to dziś minuty; host amortyzuje wspólny
  snapshot między framingami. Transport vibecrafted-weave: decyzja poza tym
  RFC; authority wystawia lokalny RPC, weave może go mostkować później.

## 9. Luki i uczciwe zastrzeżenia

- **Profil W1-06 (rozbicie 12 s na fazy) nie był dostępny** — katalog
  raportów W1 pusty w chwili tego recon (`.../2026_0701/reports/implement/`
  bez plików). Zamiast niego: własny pomiar 15.0 s (real) + teza briefu
  W1-06 (warm==cold ⇒ compose/render). Cut W3-07 musi skonsumować realny
  profil W1-06, gdy wyląduje.
- **Kanon W1-04 w toku** — plan cutów zakłada, że host serwuje kanon metryk;
  jeśli W1-04 osadzi kanon w innym module niż pack, W3-07 adaptuje się do
  tego modułu (konsumuje, nie buduje drugiego).
- **Spike mierzył jeden target i jedno repo** — 433× jest specyficzne dla
  236.8 MB snapshotu; na małych repo przewaga zmaleje (i to jest OK —
  authority ma być bezobsługowe, nie obowiązkowe).
- **stdio = jeden klient naraz** — W3-02/04 musi wybrać transport
  multi-klient (unix socket z framingiem LSP albo krótkie połączenia
  per zapytanie). Spike nie mierzył kosztu unix-socket vs stdio; różnica
  oczekiwana w dziesiątkach µs, do potwierdzenia w W3-04 verifierze (<20 ms
  end-to-end zostawia zapas dwóch rzędów wielkości).
- **`loctree-mcp` w tej sesji miał stamp `g007e9755`** przy HEAD `e3f3d774`
  (1 commit różnicy, test-only) — żywy przykład klasy, którą §6 zamyka.

---

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
