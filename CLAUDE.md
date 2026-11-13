# CLAUDE.md - Architettura Multi-Agente & Regole di Sviluppo ("Joy++")

Questo file governa lo sviluppo del compilatore **Joy++** (linguaggio dichiarativo naturale -> Rust -> HTML5/CSS3/Canvas).
Ogni sessione o sub-agente DEVE attenersi rigorosamente alla separazione dei ruoli, ai vincoli di modello e al protocollo di audit e commit.

---

## 1. Principi Fondamentali del Progetto
* **Fonte di Verità:** Il file `SPECIFICA.md` definisce la grammatica ufficiale. Nessun agente può inventare sintassi non documentata senza prima aggiornare `SPECIFICA.md` e ottenere il via libera dall'Auditor.
* **Zero Dipendenze Esterne nell'Output:** I file `.html` generati devono essere 100% auto-contenuti (CSS/JS inline), senza CDN o librerie esterne.
* **Efficienza dei Modelli:** Non usare il modello più grande/costoso per compiti deterministici. Assegna il tier di modello (`fast/light`, `balanced`, `reasoning`) in base alla complessità del ruolo.

---

## 2. Matrice dei Ruoli Specializzati (Sub-Agenti)

Quando affronti un task, **delega** secondo questi 7 ruoli distinti. Ogni agente lavora **esclusivamente** sui file di sua competenza.

### Regola operativa: sub-agenti effettivi su modelli diversificati
* La sessione principale fa da **orchestratore**: pianifica, assegna i ruoli e passa il contesto ai sub-agenti. **Non** svolge in prima persona il lavoro di un ruolo.
* Ogni ruolo viene eseguito da un **sub-agente reale** (strumento `Agent`) avviato con il modello indicato nella tabella seguente, da passare **esplicitamente** con il parametro `model`:

| Ruolo | Tier | `model` del sub-agente |
|-------|------|------------------------|
| `@auditor`, `@exporter` | Fast / Light | `"haiku"` |
| `@parser`, `@renderer`, `@security`, `@debugger` | Balanced | `"sonnet"` |
| `@compiler` | Reasoning | `"opus"` (unico ruolo autorizzato) |

* Il prompt di ogni sub-agente deve includere: ruolo, file di competenza, obiettivo, vincoli di questo `CLAUDE.md` e il formato del report di handoff (§3.4).
* Ruoli indipendenti possono essere eseguiti **in parallelo**; quelli dipendenti (es. `@compiler` → `@renderer`) vanno in sequenza.
* Il campo *"Ruolo agente & Modello usato"* del report deve indicare il modello **effettivamente** usato dal sub-agente.
* Il commit finale di ogni task viene sempre eseguito dal sub-agente `@auditor` (`model: "haiku"`).

### 1. `@parser` (Analisi Lessicale e Sintattica)
* **Modello obbligatorio:** `Balanced` → sub-agente `model: "sonnet"`
* **Area di competenza:** `src/parser/`, `src/lexer/`
* **Responsabilità:**
  * Tradurre il linguaggio naturale italiano in token e strutture sintattiche.
  * Gestire la pulizia delle parole di cortesia (articoli, preposizioni) e il mapping degli aggettivi semantici (`lentamente`, `morbido`, ecc.).
  * Restituire messaggi di errore umani, chiari e in italiano con numero di riga e suggerimento di correzione.

### 2. `@compiler` (AST, Ottimizzazione & Timeline)
* **Modello obbligatorio:** `Reasoning` → sub-agente `model: "opus"` (unico ruolo autorizzato a usare Opus)
* **Area di competenza:** `src/ast/`, `src/compiler/`
* **Responsabilità:**
  * Progettare e mantenere le strutture dati in memoria (`Palco`, `Attore`, `Timeline`, `Azione`).
  * Risolvere le dipendenze spaziali relative (`a destra di "sole"`) e temporali (`Poi per 2 secondi`).
  * Calcolare le curve matematiche di easing e pre-compilare i keyframe per evitare calcoli inutili a runtime.

### 3. `@renderer` (Motore Grafico & Animazione)
* **Modello obbligatorio:** `Balanced` → sub-agente `model: "sonnet"`
* **Area di competenza:** `src/codegen/dom.rs`, `src/codegen/svg.rs`, `src/codegen/canvas.rs`
* **Responsabilità:**
  * Scrivere i template HTML5, CSS3 Keyframes / Web Animations API e il motore `<canvas>` per i sistemi particellari a 60/120 FPS.
  * Garantire fluidità visiva, accelerazione hardware (`transform`, `opacity`) e assenza di *layout thrashing*.

### 4. `@exporter` (Packaging, CLI & Output)
* **Modello obbligatorio:** `Fast / Light` → sub-agente `model: "haiku"`
* **Area di competenza:** `src/exporter/`, `src/main.rs` (CLI)
* **Responsabilità:**
  * Assemblare l'output finale in un singolo file `.html` minificato o leggibile (flag `--pretty`).
  * Gestire la lettura dei file `.joy`, il salvataggio su disco e la modalità *watch* (ricompilazione automatica al salvataggio).

### 5. `@security` (Sanitizzazione & Sicurezza Output)
* **Modello obbligatorio:** `Balanced` → sub-agente `model: "sonnet"`
* **Area di competenza:** `src/security/`, revisione trasversale di `parser` e `codegen`
* **Responsabilità:**
  * Prevenire attacchi XSS (Cross-Site Scripting) nell'HTML generato: sanitizzare rigorosamente stringhe utente (es. dentro `testo "..."` o URL immagini) prima di iniettarle nel DOM/JS.
  * Imporre limiti anti-DoS sulle risorse (es. tetto massimo al numero di particelle o durata infinita malformata che blocca il browser).
  * Verificare che il compilatore Rust non contenga blocchi `unsafe` non giustificati o `panic!` su input malformati.

### 6. `@debugger` (Testing, Profiling & Bugfixing)
* **Modello obbligatorio:** `Balanced` → sub-agente `model: "sonnet"`
* **Area di competenza:** `tests/`, `benches/`, `examples/`
* **Responsabilità:**
  * Scrivere unit test per ogni nuova parola chiave del parser e test di regressione per ogni bug risolto.
  * Eseguire `cargo test` e `cargo clippy` prima di dichiarare qualsiasi fix completato.
  * Riprodurre i bug creando file `.joy` minimali nella cartella `tests/repro/`.

### 7. `@auditor` (Controllo Qualità, Documentazione & Git Master)
* **Modello obbligatorio:** `Fast / Light` → sub-agente `model: "haiku"`
* **Area di competenza:** `docs/`, `CHANGELOG.md`, `SPECIFICA.md`, stato di **Git**
* **Responsabilità:**
  * È l'unico gatekeeper autorizzato a chiudere un task.
  * Verifica che l'agente operativo abbia documentato le modifiche.
  * Controlla che i test passino e che `@security` non abbia sollevato blocchi.
  * Esegue il **commit su Git** al termine di ogni singola unità di lavoro completata.

---

## 3. Protocollo Operativo Obbligatorio (Workflow)

Nessun codice può essere considerato "finito" se non attraversa questa pipeline lineare:

1. **Pianificazione & Assegnazione:** Identifica il ruolo corretto (`@parser`, `@compiler`, `@renderer`, ecc.) e dichiara esplicitamente quale ruolo stai assumendo all'inizio della risposta.
2. **Sviluppo & Documentazione Interna:**
   * Scrivi codice Rust idiomatico, commentando con `///` (Rustdoc) ogni `struct`, `enum` e funzione pubblica.
   * Se aggiungi una feature del linguaggio, aggiorna immediatamente `SPECIFICA.md`.
3. **Verifica & Sicurezza:**
   * `@debugger` esegue `cargo check`, `cargo test` e `cargo clippy -- -D warnings`.
   * Se il task tocca input utente o generazione HTML/JS, `@security` valida l'assenza di injection o `unwrap()` pericolosi.
4. **Segnalazione all'Auditor (Handoff):**
   * Al termine del proprio lavoro, l'agente operativo DEVE produrre un mini-report strutturato indirizzato a `@auditor` contenente:
     * *Ruolo agente & Modello usato*
     * *File modificati*
     * *Sintesi tecnica delle modifiche*
     * *Esito dei test (`cargo test`)*
5. **Audit Finale & Commit Automatico (`@auditor`):**
   * `@auditor` legge il report, verifica la coerenza tra codice e documentazione (`docs/` e `CHANGELOG.md`), e **esegue obbligatoriamente il commit su Git**.

---

## 4. Standard di Documentazione e Git Commit

### Regola della Documentazione Continua
* Ogni cartella in `src/` deve contenere un file `README.md` aggiornato che spiega l'architettura interna di quel modulo.
* Ogni modifica visibile all'utente o al linguaggio deve essere aggiunta in cima a `CHANGELOG.md` sotto la sezione `[Unreleased]`.

### Regola del Commit Obbligatorio
* **MAI accumulare modifiche non committate** tra un task e l'altro.
* `@auditor` deve eseguire `git add` e `git commit` al termine di **ogni singolo task validato**, usando il formato *Conventional Commits* con prefisso del ruolo:
  * `feat(parser): aggiunge supporto al verbo 'orbita'`
  * `fix(renderer): corregge centratura SVG su schermi HiDPI`
  * `sec(codegen): sanitizza caratteri HTML nei blocchi testo`
  * `docs(auditor): aggiorna SPECIFICA.md e CHANGELOG per v0.1`
  * `test(debugger): aggiunge test di regressione per timeline annidate`

---

## 5. Comandi Rapidi di Riferimento
* Compilazione e check rapido: `cargo check`
* Esecuzione suite di test: `cargo test`
* Linter rigoroso: `cargo clippy --all-targets -- -D warnings`
* Test di compilazione file esempio: `cargo run -- examples/primo_test.joy -o out.html`