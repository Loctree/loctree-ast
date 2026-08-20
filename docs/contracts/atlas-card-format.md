# Kontrakt · Golden format kart atlasu (atlas-card-format, kanon v4)

> **KANON.** Przeniesiony do repo przez C0-01 (substrate-makieta v4) z draftu
> `design/card-format-spec.md`. Briefy i weryfikatory (L1-00, L1-01, L1-02,
> M1-01, E1-01a) powołują się na TEN plik. Draft poza repo nie jest kontraktem.

Doktryna formatu: karta czytana OD DECHY DO DECHY przymusowo → każda linia
niesie wiedzę. Sed-readable = jeden fakt na linię, stałe prefiksy sekcji,
zero wieloliniowych struktur. Tabele TYLKO dla enumeracji jednorodnych;
proza TYLKO dla przyczyn. Zakaz płotków ```json w .md (payload → .full.json).
**v4: każda linia faktu bazowego ma gramatykę pozwalającą ODTWORZYĆ fact_id
maszynowo (parser weryfikatora L1-01 rekonstruuje zbiór i porównuje z
coverage_receipt — równość zbiorów, nie substring).**

## Markery — lifecycle i dowodowość to OSOBNE osie (v4)

- Lifecycle (aktualność): `✓` current · `⊘` superseded · `✗` disputed /
  anti-rekomendacja. Mapowanie 1:1 na `status` kontraktu
  `loctree.overlay.intent.v1`.
- Dowodowość (verification_status): `[V]` verified · `[U]` unverified ·
  `[R]` refuted. Mapowanie 1:1 na `verification_status` kontraktu.
- `agent_derived` + `unverified` NIE może wyglądać jak potwierdzona prawda —
  `✓[U]` czytelnie niesie "aktualna, niezweryfikowana". `[R]` (refuted) nigdy
  nie występuje z `✓` (kontrakt waliduje: refuted ⊥ current).
- Lifecycle bez dowodowości (`✓` maskujący unverified) to v3 bug — naprawiony
  w v4 i ZAKAZANY.

## Karty atlasu — inwentarz i właścicielstwo domen

Sześć kart, stałe nazwy plików. Nagłówki sekcji STAŁE — weryfikatory i mapa
właścicielstwa polegają na nich; mapa `domain_owner` w MANIFEŚCIE (L1-02)
jest source of truth, nagłówki są jej projekcją. Jedna domena = jeden
właściciel; inna karta może domenę REFERENCJONOWAĆ (1 linia + wskazanie
właściciela), nigdy duplikować.

| Karta | Plik | Domeny (owner) |
|---|---|---|
| 00 core | `00-core-map.md` | identity, risk summary (projekcja z 01), safe next commands |
| 01 structural | `01-structural-map.md` | hubs, hotspots, consumers/edges, import graph, authority, reachability |
| 02 runtime | `02-runtime-map.md` | entrypoints, env contracts, framework hints, dispatch edges |
| 03 intent | `03-intent-map.md` (upgrade z `03-memory-trail.md`, M1-01) | decyzje/intencje, relacje, anti-rekomendacje, superseded history |
| 04 verification | `04-verification-gates.md` | gates, likely tests, downstream checks |
| 05 risk | `05-risk-register.md` | cache/snapshot health (owner domeny freshness), stale assumptions, actions |

Mapa `domain_owners` w `manifest.json` (L1-02) niesie `{domena: stem karty}`
(np. `"hotspots": "01-structural-map"`), dokładnie jeden właściciel na domenę;
`manifest.md` projektuje ją w sekcji `## Domain owners`. Fact_id domen
przekrojowych noszą prefiks domeny (`hotspots:` / `authority:` /
`reachability:`), a coverage_receipt karty-właściciela jest JEDYNYM receiptem,
w którym te prefiksy występują — właścicielstwo weryfikowalne po FAKTACH,
nie tylko po nagłówkach (duplikat przemycony bez nagłówka też jest
naruszeniem). Payloady `.full.json` mogą nieść duplikację maszynową między
kartami (slice-local truth); duplikacja na powierzchni markdown i w receiptach
jest zakazana. Domena `intent → 03` jest forward-declared dla M1-01
(właścicielem karty intencji jest 03, nie żadna nowa karta).

Reguły wspólne (wszystkie karty):

- Nagłówek karty: `# <Tytuł> · <repo> @ <branch>@<snapshot_commit>` + linia
  freshness + linia `Full payload: <card>.full.json`. `.full.json` istnieje
  ZAWSZE (L1-00), nie tylko przy overflow.
- Fakty bazowe (edges/deps/risks/hubs/tezy/gates/reachability) inline
  w całości; `.full.json#<klucz>` prowadzi do dowodu/szczegółu, nigdy nie
  zastępuje faktu (decision-complete).
- `+N kolejnych` ZAKAZANE dla faktów bazowych; legalne tylko dla listy
  symboli w ramach wypisanego faktu.
- Budżet linii miękki (01: ~350 · 03: ≤200 · pozostałe: ~200): kompaktuj
  FORMĘ (grupowanie, szerokość linii), nigdy nie wycinaj faktów bazowych.
  Realne przepełnienie ⇒ eskaluj próg JAWNIE w nagłówku sekcji, nie tnij cicho.
- Sekcja pusta = jedna jawna linia (`no <what> — corpus: <N>`), nie znika.

## 00-core-map.md — sekcje kanoniczne

- `## Identity` — repo_id, branch, snapshot_commit, store_revision,
  overlay_revision, anchor_catalog_revision (po jednej linii; brak = jawny
  `unavailable` + powód). Gramatyka: `<pole>: <wartość>`.
- `## Freshness` — stan snapshotu (fresh/dirty/stale); szczegóły
  cache/snapshot to 1-liniowa referencja do 05 (owner domeny freshness),
  nie kopia pól.
- `## Risk Summary` — projekcja top-N ryzyk fan-in z karty 01 (owner domeny
  hotspots: 01); każda linia kończy się `→ 01-structural-map.md`.
- Authority: BEZ własnej sekcji (L1-02) — 1-liniowa referencja
  `authority (liczniki per label) → 01-structural-map.md §Authority`.
- `## Safe Next Commands` — komendy bezpieczne w bieżącym stanie repo, jedna
  na linię, z jednym zdaniem "kiedy".

## 01-structural-map.md — sekcje kanoniczne (golden, fragment wzorcowy)

```markdown
# Structural Map · loctree-suite @ feat/substrate-scaffold@c53aa559
Freshness: DIRTY — zweryfikuj zmienione pliki przed poleganiem na karcie.
Full payload: 01-structural-map.full.json

## Hubs (fan-in ≥ 10) — domain owner: this card
| # | Plik | Fan-in | Fan-out | Rola |
|--:|---|--:|--:|---|
| 1 | loctree-rs/src/types.rs | 82 | 3 | shared types (celowo scentralizowane) |
| 2 | loctree-rs/src/snapshot.rs | 39 | 22 | snapshot lifecycle + cache |
| 3 | reports/src/types.rs | 22 | 1 | report data bridge |

hotspots:loctree-rs/src/types.rs · importers 82 · mitigation: `loct slice` before edit, `loct impact` before delete
hotspots:loctree-rs/src/snapshot.rs · importers 39 · mitigation: `loct slice` before edit, `loct impact` before delete

## Consumers per hub (DECISION-COMPLETE: wszystkie edges inline, grupowane)
types.rs ← analyzer/ast_js/{calls,exports,imports,…}.rs · analyzer/{coverage,cycles,dead_parrots,…}.rs · cli/dispatch/handlers/{context,watch,…}.rs · …(KAŻDY konsument wymieniony; grupowanie po katalogu kompaktuje FORMĘ, nie zbiór — gramatyka linii: `hub ← grupa{a,b,c}` rozwija się deterministycznie do fact_id `edge:<hub>:<konsument>`)

## Import graph — shape
942 edges · 30 ranked hubs · max re-export chain depth: 4
File clusters: analyzer/* (76k LOC, samowystarczalny) · cli/* → analyzer · reports/* (izolowany od cli)

## Reachability — domain owner: this card
reachable: 113 of 113 reachability claims
no unreachable surfaces — claims: 113

## Authority — domain owner: this card
authority:RepoVerified 1 · LoctreeDerived 147 · AicxOperator 0 · AicxAgent 0 · AicxFailure 0 · SemanticGuess 221 · StaleOrUnknown 0
```

Reguły twarde 01:

- Nagłówki sekcji STAŁE: `## Hubs`, `## Consumers per hub`, `## Import graph`,
  `## Reachability`, `## Authority`.
- Linia consumers: `plik ← konsument (symbole…)`, symbole ≤5 + `+N kolejnych`
  (legalne — symbole to detal wypisanego edge'a, nie fakt bazowy).
- fact_id edges: `edge:<hub>:<konsument>`; hubs: `hub:<plik>`. Fakty `hub:`
  parser odtwarza z wierszy tabeli `## Hubs` (wiersz z liczbą w pierwszej
  kolumnie ⇒ `hub:<Plik>`); payload karty 01 niesie `high_fan_in` (L1-01),
  a karta 05 hubów NIE dubluje — tylko 1-liniowa referencja do właściciela.
- Hotspoty (L1-02): linie `hotspots:<file> · importers N · mitigation…` w
  sekcji `## Hubs` (jedna domena hub/hotspot, jedna sekcja). fact_id
  `hotspots:<plik>`. Dane maszynowe żyją w payloadzie karty 05
  (`risk.hotspots`) — derywacja weryfikatora jest cross-payload.
- Authority (L1-02): JEDNA linia licznikowa
  `authority:<Label> <N> · <Label> <N> · …` (stała kolejność 7 labeli,
  liczniki pack-wide). fact_id `authority:<Label>` wyłącznie dla labeli
  z N > 0 — zera nie są faktami. Inline `(RepoVerified)` przy pojedynczych
  faktach INNYCH kart pozostaje legalne (metadana faktu, nie duplikacja
  domeny authority).
- Reachability (L1-02): sekcja przeniesiona z karty 02 (definicja niżej,
  przy karcie 02). fact_id `reachability:<plik>` per powierzchnia
  nieosiągalna; gramatyka linii:
  `reachability:<file> · unreachable · hypothesis: <reason>`.
- Gramatyka zakłada ścieżki bez sekwencji ` · `, ` ← `, `{`, `}` i `,` —
  spełnione dla całego indeksowanego uniwersum; naruszenie = eskalacja do
  trybu "jeden fakt = jedna pełna linia" (recovery hint L1-01).

## 02-runtime-map.md — sekcje kanoniczne

- `## Entrypoints` — punkty startu runtime (bin, handler, serwis); gramatyka:
  `entry:<plik> · <rodzaj> · <jak uruchamiany>`. fact_id `entry:<plik>`.
- `## Env Contracts` — zmienne środowiskowe i ich konsumenci; gramatyka:
  `env:<NAZWA> · <plik konsumenta> · required|optional · default?`.
  fact_id `env:<NAZWA>:<plik>`.
- `## Framework Hints` — wykryte mosty frameworkowe (leptos, tokio, clap…),
  po jednej linii z dowodem plikowym.
- `## Dispatch Edges` — krawędzie dynamicznego dispatchu (command→handler,
  event→listener); gramatyka: `dispatch:<źródło>→<cel> · <mechanizm>`.
  fact_id `dispatch:<źródło>:<cel>`.
- Reachability: BEZ własnej sekcji na tej karcie (L1-02) — 1-liniowa
  referencja `reachability → 01-structural-map.md §Reachability`.

### Definicja `## Reachability` (sekcja żyje na karcie 01, owner: 01)

Reachability = osiągalność pliku/symbolu z runtime'owych entrypointów przez
złożenie grafu importów (karta 01) z dispatch edges (karta 02) — dlatego od
L1-02 sekcja i fakty mieszkają na karcie 01 (mapa `domain_owners`), a karta 02
niesie tylko referencję. Sekcja niesie DWA rodzaje faktów bazowych, oba inline:

1. Osiągalne powierzchnie: `reach:<entrypoint> → <plik|klaster> · głębokość N`
   — jedna linia per entrypoint per klaster docelowy; grupowanie katalogowe
   kompaktuje formę, nie zbiór (rozwija się deterministycznie jak edges 01).
   **Stan v4/L1-01:** pack `loctree.context.v1` niesie reachability na
   poziomie symbolu (reached/unreached) BEZ atrybucji entrypointu — fakty
   `reach:*` wchodzą dopiero, gdy composer zacznie nieść per-entry
   attribution; do tego czasu powierzchnia osiągalna to jawna linia zbiorcza
   (`reachable: <N> of <M> reachability claims`).
2. Nieosiągalne powierzchnie: `reachability:<file> · unreachable ·
   hypothesis: <dead|lazy|reflection|test-only>` — kandydaci dead-code z
   hipotezą przyczyny; to jest WEJŚCIE dla `loct follow dead`, nie werdykt.
   fact_id `reachability:<plik>` (prefiks domeny — do L1-02 `unreachable:`,
   przemianowany, by właścicielstwo było weryfikowalne po receiptach).

Weryfikatory polegają na tym nagłówku: L1-01/L1-02 rekonstruują zbiór fact_id
`reach:*`/`reachability:*` z gramatyki linii i porównują RÓWNOŚCIĄ ZBIORÓW
z coverage_receipt karty 01; M1-01 i E1-01a traktują brak sekcji na karcie 01
(or an explicit empty line `no reachability data — semantic pass found no
entrypoints`) as a completeness FAIL. Reachability does not encode intent —
"dlaczego coś jest celowo nieosiągalne" żyje w karcie 03 (owner: intent).
Dane maszynowe pozostają w payloadzie karty 02 (`.full.json#reachability`) —
derywacja weryfikatora jest cross-payload.

## 03-intent-map.md — sekcje kanoniczne (golden, karta makiety — upgrade 03-memory-trail)

```markdown
# Intent Map · loctree-suite @ feat/substrate-scaffold@c53aa559
Źródło: aicx overlay v1 · store_revision <fp> · overlay_revision <fp> · wygenerowano 2026-07-13T12:00Z
Zejście po dowód: `aicx read chunk:<id>` · pełny payload: 03-intent-map.full.json
Markery: ✓ current · ⊘ superseded · ✗ anti-rekomendacja || dowodowość: [V] verified · [U] unverified · [R] refuted

## Per-hub — formative decisions (fan-in ≥ 10)
loctree-rs/src/types.rs
  ✓[V] 2026-05-14 · operator_confirmed · centralizacja shared types utrzymana; rozbicie odrzucone (koszt cache-invalidation) · session c935402e §L120
loctree-rs/src/snapshot.rs
  ✓[U] 2026-06-05 · agent_derived · single-flight lock na cache-bucket przy równoległych skanach · session d9100591 §L40
  ⊘[V] 2026-06-04 · agent_derived · per-plik atomic write uznany za wystarczający → superseded przez single-flight · session e0568e95

## Repo-wide
  ✓[V] 2026-07-12 · operator_confirmed · force-feed pełnej struktury; "on-demand" zakazany dla warstwy bazowej · session c4d187f9 §turn-42
  ✓[V] 2026-07-12 · operator_confirmed · dedup intencji SEMANTIC, nie fingerprint (fingerprint już poległ) · session c4d187f9 §turn-58

## Anti-recommendations (AicxFailure)
  ✗[V] 2026-06-30 · shadow-cleanup w install.sh kasował binarki hosta — nie przywracać · session 0ac5209

## Superseded (historia — 1 linia/wpis)
  ⊘[·] <data> · <teza skrócona> → superseded przez <data> · session <id>
```

Reguły twarde 03:

- Nagłówki sekcji STAŁE: `## Per-hub`, `## Repo-wide`, `## Anti-recommendations`,
  `## Superseded`.
- Jedna teza = JEDNA linia: `lifecycle[dowodowość] · data · authority · teza ·
  ref`. fact_id `thesis:<intent_id>` (intent_id z kontraktu overlay, nie
  wyliczany z treści linii). Konsekwencja (M1-01): weryfikacja domeny intent
  jest cross-payload — receipt ↔ payload porównują się per `intent_id`
  (`.full.json#rendered_theses`), a uczciwość markdown dowodzi parytet LICZBY
  linii tezowych w obu kierunkach (mutation test widzi usunięcie linii przez
  spadek licznika).
- Do bazowej karty trafiają WYŁĄCZNIE atrybucje powyżej progu confidence;
  `unresolved_attributions` żyją w .full.json (payload-only) — force-fed truth
  nie niesie zgadywanek (Doktryna 7).
- Ref zawsze na końcu linii, format `session <short-id> §<span>` — sed/awk-owalne;
  odpowiada opaque ref `session:<id>#<span>` kontraktu. Zero ścieżek absolutnych.
- Tezy per-hub tylko dla plików z fan-in ≥ progu (spójnie z sekcją Hubs karty 01).
- Budżet ≤200 linii; tezy są faktami bazowymi — nadmiar kompaktuj formą, przy
  realnym przepełnieniu eskaluj próg fan-in JAWNIE w nagłówku sekcji (np.
  'fan-in ≥ 15'), nie tnij cicho.
- Empty section = one explicit line (`no registered per-hub decisions — corpus: <N> overlay entries` / `no repo-wide entries — corpus: <N>`), never omit the section.

## 04-verification-gates.md — sekcje kanoniczne

- `## Gates` — komendy bram jakości repo (build/lint/test/semgrep), jedna na
  linię: `gate:<komenda> · <zakres> · <kiedy obowiązkowa>`. fact_id
  `gate:<komenda>`.
- `## Gates` gramatyka doprecyzowana (L1-01): fact_id `gate:<komenda>` =
  wszystko po `gate:` do pierwszego ` · ` (albo końca linii); zakres/kiedy
  po ` · ` są opcjonalnym detalem, nie częścią fact_id.
- `## Likely Tests` — testy najprawdopodobniej dotknięte bieżącym scope;
  gramatyka: `test:<ścieżka testu>`. fact_id `test:<ścieżka>`. **Korekta
  L1-01:** pack v1 nie atrybuuje testu do pliku zmienianego (`likely_tests`
  to płaskie ścieżki), więc sufiks ` ← <plik zmieniany>` jest opcjonalnym
  detalem przyszłości, nie częścią fact_id.
- `## Downstream Checks` — sprawdzenia poza tym repo (konsumenci, mirror
  fixtures, integracje), z JEDNYM zdaniem jak uruchomić.

## 05-risk-register.md — sekcje kanoniczne

- Hotspots: BEZ własnej sekcji (L1-02) — 1-liniowa referencja
  `hotspots + hub-y (fan-in) → 01-structural-map.md §Hubs`. Rodzina fact_id
  `risk:<plik>` wycofana; receipt karty 05 jest pusty (karta referencyjna),
  a dane maszynowe zostają w jej payloadzie (`.full.json#risk`).
- `## Cache & Snapshot Health` — świeżość snapshotu, spójność cache,
  rozjazdy snapshot↔worktree (owner domeny freshness).
- `## Stale Assumptions` — założenia z poprzednich sesji wymagające
  re-weryfikacji, po jednej linii z datą pochodzenia.
- `## Actions` — następne ruchy redukujące ryzyko, posortowane malejąco po
  dźwigni; jedna linia = jedna akcja z powodem.
- Authority: BEZ własnej sekcji (L1-02) — 1-liniowa referencja
  `authority (liczniki per label) → 01-structural-map.md §Authority`.

## Anty-wzorce (dla wszystkich kart)

- Blok JSON w .md (payload jest w .full.json — kropka).
- Wieloliniowy wpis tam, gdzie ustalono jedną linię.
- "Ładne" nagłówki zmieniane per-generację (łamią weryfikatory i sed-nawyki agentów).
- Cytat z sesji zamiast tezy (teza = destylat deklaratywny, ≤200 znaków).
- Lifecycle bez dowodowości (✓ maskujący unverified — v3 bug, naprawiony w v4).
- Duplikacja domeny między kartami (mapa domain_owner w MANIFEŚCIE rozstrzyga;
  referencja 1-liniowa legalna, kopia faktów — nie).
- Ścieżka absolutna gdziekolwiek w karcie (bucket leak — kontrakt refs jest
  opaque: `chunk:<id>` / `session:<id>#<span>`).
