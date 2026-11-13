# `src/compiler` — Risoluzione, Timeline e Keyframe (`@compiler`)

Trasforma il `Programma` (AST) in una `ScenaCompilata`: una rappresentazione già
"appiattita" che il generatore può emettere senza fare calcoli.

## Moduli

| File | Contenuto |
|------|-----------|
| `mod.rs` | Tipi pubblici (`ScenaCompilata`, `AttoreCompilato`, `Canale`, `Valore`, ...), `compila`, validazione, segmenti, canali indipendenti |
| `gruppi.rs` | Albero dei gruppi, controlli di B.3.8, livelli, pre-ordine (B.3.5) |
| `posizioni.rs` | Centri dichiarati e pivot (A.2, B.3.3, B.3.6), ingombri iniziali, cicli |
| `ingombro.rs` | Ingombri iniziali e prudenti (B.0.4, B.3.4), stima dei testi, fuori campo (B.2.1) |
| `keyframe.rs` | `Accumulatore` dei keyframe in secondi, limite sinistro (B.0.2), estensione dei canali |
| `traslazione.rs` | Canale `traslazione` per livelli (B.0.5): `entra da`, `esce verso`, `orbita`, salto (B.0.3) |
| `orbita.rs` | Campioni per giro (sagitta ≤ 0.5 px, 36-360), angolo di partenza e verso |
| `easing.rs` | Curve CSS, campionamento di `elastico`/`rimbalzo`, `Curva::valuta`, `Curva::Gradino` |

## Fasi

1. **Validazione attori**: nomi duplicati (attori e gruppi condividono i nomi), limiti
   anti-DoS (vedi `src/security`; i gruppi contano in `MAX_ATTORI`), compreso lo stato
   iniziale `scala` (0 ≤ s0 ≤ `MAX_SCALA`) e `rotazione` (|r0| ≤ `MAX_GRADI`) di
   SPECIFICA B.1. Gli errori citano la riga di `Crea`.
2. **Albero dei gruppi** (`gruppi.rs`): genitore di ogni attore, controlli di B.3.8
   (membri inesistenti, ripetuti, creati dopo il gruppo, già in un altro gruppo, gruppo che
   contiene sé stesso, profondità > `MAX_PROFONDITA_GRUPPI`), ordine di emissione.
3. **Risoluzione spaziale** (`posizioni.rs`): ogni attore ottiene il proprio **centro** nelle
   coordinate del palco non trasformate; un gruppo ottiene il pivot `P` (centro dell'AABB degli
   ingombri iniziali dei membri). Il grafo «relativa → riferimento» e «gruppo → membri» è
   visitato in profondità senza ricorsione; un ciclo produce `posizioni circolari: …`. Se il
   riferimento non esiste, il suggerimento «forse intendevi …?» non propone mai l'attore che si
   sta posizionando.
4. **Risoluzione temporale**: i blocchi relativi (`Poi per 2 secondi:`) vengono ancorati
   alla fine del blocco precedente; si calcola la durata totale della scena.
5. **Canali di animazione**: ogni azione diventa un *segmento* su uno dei canali
   indipendenti `opacita`, `traslazione`, `rotazione`, `scala`, `colore`. Segmenti sovrapposti
   sullo stesso canale dello stesso attore producono un errore leggibile.
6. **Pre-calcolo keyframe** nell'ordine di B.0.5: prima `opacita`, `rotazione`, `scala`,
   `colore` di tutti gli attori; poi `traslazione`, un livello di annidamento alla volta dal più
   interno, con i segmenti di un livello in ordine di inizio (a parità, di riga). Le coreografie
   (`pulsa`, `orbita`) sono espanse in Rust in keyframe espliciti.
7. **Avvisi** (SPECIFICA B.6): problemi non bloccanti raccolti in `ScenaCompilata::avvisi`
   (`Vec<Avviso>`, tipo definito in `src/errore.rs`), ordinati per riga e deduplicati
   (stessa riga + stesso messaggio = uno solo). Il compilatore non li stampa: lo fa
   `@exporter` su stderr. Avvisi attuali: «rotazione invisibile su un cerchio», per
   `ruota di` su un `cerchio` (riga dell'azione) e per `rotazione` ≠ 0 alla creazione di un
   `cerchio` (riga di `Crea`); «opacità invariata» (v0.2.1), per un'azione di opacità il cui
   arrivo è uguale (|Δ| < 1e-9, `TOLLERANZA_OPACITA`) al valore corrente all'inizio del
   segmento. Quest'ultimo è rilevato da `controlla_opacita_invariata` nel passo 2 di B.0.5,
   subito dopo aver fissato l'opacità iniziale del canale (0 se il primo segmento è `appare
   sfumando`): scorre i segmenti ordinati e non sovrapposti del solo canale `opacita`
   dell'attore o gruppo, portando avanti il valore di arrivo di ciascuno. I testi sono in
   `avviso_opacita_invariata`, una variante per verbo.
8. **Easing** (`easing.rs`): le curve `elastico` e `rimbalzo` sono campionate
   matematicamente e convertite nella funzione CSS `linear(...)`.

## Scelte d'implementazione del blocco 2B

* **Keyframe nello stesso istante:** come nella v0.1 si fondono quando l'offset differisce
  meno di 1e-9; la `δ` del salto (≥ 0.001 s e ≥ 2·10⁻⁶ × durata) li tiene sempre distinti.
* **Limite sinistro** (B.0.2): un keyframe che cade esattamente in `t0` vale come valore
  «prima di `t0`», perché porta il valore precedente al segmento; dopo un `Gradino` il limite
  sinistro in `t0 + δ` è ancora il valore prima del salto.
* **Estensione dei canali:** il tratto si allarga di 1.36 quando la curva campionata supera 1
  (`Curva::supera_bersaglio`, oggi solo `elastico`).
* **`orbita` senza salto** (`|d − raggio| ≤ 0.5`): il campione 0 sostituisce il keyframe in
  `t0` (differenza ≤ 0.5 px). I campioni hanno curva lineare (`None`), tranne il primo
  (`Curva::Lineare` esplicita, così prevale su un'eventuale curva già presente in `t0`).
  L'errore «orbita troppo lunga» conta `N + 1` fotogrammi chiave.
* **Raggio dell'orbita oltre `MAX_COORDINATA`:** errore
  `il raggio dell'orbita è troppo grande (massimo 1000000 pixel)`.
* **Fuori campo oltre `MAX_COORDINATA`:** errore esistente `coordinate fuori dai limiti`.
* **Tetto globale dei keyframe** (`keyframe::Tetto`, `MAX_KEYFRAME_TOTALI = 200000`): un
  contatore condiviso da tutti gli `Accumulatore` della compilazione conta ogni keyframe nuovo
  (non quelli fusi) di tutti i canali di attori e gruppi. Appena il totale supera il tetto la
  compilazione si ferma con `animazione troppo complessa: più di 200000 fotogrammi chiave in
  totale` (suggerimento: «riduci il numero di attori animati, i giri delle orbite o il ritmo
  delle pulsazioni»), senza generare il resto. Resta anche il limite per canale `MAX_KEYFRAME`.
  `compila_con_tetto` (privata) permette ai test di usare un tetto piccolo.
* **Coordinate dell'orbita:** una verifica per orbita, `|Cx| + raggio` e `|Cy| + raggio` entro
  `MAX_COORDINATA`, altrimenti `coordinate fuori dai limiti` alla riga dell'azione.
* **Gruppo senza membri risolti:** impossibile dopo i controlli dell'albero; per prudenza
  produce l'errore «deve contenere almeno un membro» e non un panic.

Perché canali separati: le proprietà CSS individuali `translate`, `rotate`, `scale` e
`opacity` si compongono tra loro, quindi animazioni diverse sullo stesso attore non si
sovrascrivono (cosa che invece accadrebbe con un'unica proprietà `transform`).

## Stato iniziale dei canali (SPECIFICA B.1)

| Canale | Valore a `t = 0` |
|--------|------------------|
| `opacita` | `stile.opacita` (0 se il primo segmento è `appare sfumando`) |
| `traslazione` | `(0, 0)`, relativo al centro risolto |
| `rotazione` | `stile.rotazione` (`r0`, predefinito 0) |
| `scala` | `stile.scala` (`s0`, predefinito 1) |

* `ruota di G` è **relativo**: porta la rotazione corrente a `corrente + G`.
* `si ingrandisce` / `si rimpicciolisce fino a F` sono **assoluti**: portano la scala a F.
* `pulsa` oscilla tra la scala corrente `b` e `b × (1 + 0.25 × intensità)`; con `b = 0` resta a 0.
* `appare sfumando` (A.3, v0.2.1) porta l'opacità a `stile.opacita` se è > 0, altrimenti a 1
  (`arrivo_appare`). Vale anche per i gruppi. Il verbo di opacità che ha prodotto un segmento è
  nel campo `Segmento::opacita` (`AzioneOpacita`: `Appare`, `Scompare`, `Cambia`).

`AttoreCompilato` espone `scala_iniziale` e `rotazione_iniziale`: il generatore li emette
come `scale` / `rotate` nella regola statica del wrapper `#aN` quando differiscono da 1 e 0,
anche se l'attore ha animazioni.

## Contratto v0.2 blocco 2B

Contratto dei tipi tra `@parser`, `@compiler` e `@renderer` per SPECIFICA B.2 (`entra da` /
`esce verso`), B.3 (gruppi), B.4.1 (`orbita`) e B.4.2 (`cambia colore`). Implementato
interamente dal compilatore (fase 2).

### Cosa produce `@parser` (AST, `src/ast/mod.rs`)

| Frase | AST |
|-------|-----|
| `Crea un gruppo chiamato "g" con "a", "b" e "c", scala 0.8 e livello 1.` | `Attore { nome: "g", forma: Forma::Gruppo { membri: vec!["a", "b", "c"] }, stile: Stile { scala: 0.8, livello: 1, ..Stile::default() }, posizione: Posizione::Palco(Ancora::Centro), riga }` |
| `"t" entra dall'alto con movimento scatto.` | `Azione { attore: "t", verbo: Verbo::EntraDa(Direzione::Alto), easing: Easing::Scatto, riga }` |
| `"s" esce verso destra.` | `Verbo::EsceVerso(Direzione::Destra)` (easing predefinito `Morbido`) |
| `"terra" orbita attorno a "sole" con raggio 200 per 2 giri.` | `Verbo::Orbita { centro: "sole", raggio: 200.0, giri: 2.0, orario: false }` |
| `"luna" orbita attorno a "terra" con raggio 40 per 0.5 giri in senso orario.` | `Verbo::Orbita { centro: "terra", raggio: 40.0, giri: 0.5, orario: true }` |
| `"sole" cambia colore in #FDE68A.` | `Verbo::CambiaColore(Colore { r: 0xFD, g: 0xE6, b: 0x8A, a: 1.0 })` |

* **Gruppo:** `membri` nell'ordine scritto, nomi ripuliti dagli spazi, almeno uno. Il parser
  rifiuta: lista vuota, stringhe dopo l'inizio delle proprietà, una posizione esplicita
  (B.3.8, anche `dentro a`, B.3.1) e le proprietà diverse da `opacità`, `livello`, `scala`,
  `rotazione`. Nello `Stile` di un gruppo gli altri campi restano a `Stile::default()`; la
  `posizione` è sempre `Posizione::Palco(Ancora::Centro)` e viene ignorata.
* **Verbi:** `Verbo` non è più `Copy`. `EntraDa` contiene il lato di provenienza, `EsceVerso`
  quello di uscita. In `Orbita`, `orario: false` vale sia per `in senso antiorario` sia per il
  verso omesso; `giri` può essere frazionario. I valori numerici sono passati così come scritti:
  segni, zero e limiti (`raggio > 0`, `giri > 0`, `MAX_COORDINATA`) li controlla il compilatore.
* Errori semantici (membri inesistenti o creati dopo il gruppo, appartenenza doppia, membro
  ripetuto, gruppo che contiene sé stesso, profondità > `MAX_PROFONDITA_GRUPPI`, `cambia colore`
  su un gruppo, orbita attorno a sé stessi o tra gruppi diversi): li segnala `@compiler`.

### Cosa riceve `@renderer` (`ScenaCompilata`)

* `ScenaCompilata.attori` è in **pre-ordine** dell'albero dei gruppi: il genitore precede i
  figli, ogni sottoalbero è contiguo e i fratelli sono già nell'ordine del DOM di B.3.5 (riga di
  creazione; un gruppo occupa il posto del suo discendente creato per primo). Il generatore
  emette gli elementi in quest'ordine senza riordinarli. Senza gruppi l'ordine e gli `id`
  (`aK`, `K` = indice nel vettore) sono identici alla v0.1.
* `AttoreCompilato.genitore: Option<usize>`: indice del gruppo genitore diretto, `None` per i
  figli del palco. Emissione con una pila dei gruppi aperti (prima di `attori[i]` si chiudono i
  wrapper finché la cima non è `attori[i].genitore`), oppure ricorsiva con
  `ScenaCompilata::figli(genitore)`.
* `x`, `y` sono **relativi al genitore** e diventano `left`/`top` del wrapper: per il primo
  livello relativi al palco (come prima), per un membro `posizione − P_genitore`, per un gruppo
  `P − P_genitore`.
* `Forma::Gruppo`: wrapper `<div class="attore gruppo" id="aK" data-nome="…">` con i figli
  dentro, nessuna `.forma` e nessuna regola `#aK>.forma`. Riceve come gli altri `z-index`
  (`stile.livello`), `opacity` iniziale, `scale`/`rotate` iniziali e le animazioni dei canali
  `opacita`, `traslazione`, `rotazione`, `scala` (mai `colore`).
* `Canale::Colore`: `suffisso()` = `"colore"`; `bersaglio()` = `Bersaglio::Forma` (gli altri
  canali danno `Bersaglio::Wrapper`); `proprieta_css(&forma)` = `"color"` per `Forma::Testo`,
  `"background-color"` per cerchi e rettangoli. Il generatore emette
  `@keyframes aK-colore{…}` e la regola `#aK>.forma{animation:aK-colore Ds linear both}`;
  il canale non entra in `will-change` né nell'`animation` del wrapper.
* `Valore::Colore(Colore)`: riemesso con la stessa funzione dei colori statici (`#rrggbb` se
  opaco, altrimenti `rgba(…)`).
* `Curva::Gradino`: emessa come `steps(1,end)` nell'`animation-timing-function` del keyframe
  (salto istantaneo di B.0.3). Già implementato in `dom.rs`.
* `entra da`, `esce verso` e `orbita` producono normali keyframe del canale `traslazione`
  (eventualmente con `Curva::Gradino`): nessun lavoro aggiuntivo per il generatore.
