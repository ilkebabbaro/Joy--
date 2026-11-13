# SPECIFICA UFFICIALE LINGUAGGIO "JOY++" (v1.0)
**Target:** Compilatore Rust -> Output HTML5 / CSS3 / SVG / Canvas Auto-contenuto

## 1. Filosofia e Sintassi Generale
* **Scopo:** Linguaggio dichiarativo ad alto livello per la regia di animazioni web ad alte prestazioni.
* **Struttura a frasi:** Ogni istruzione è una frase completa in italiano naturale e termina obbligatoriamente con un punto (`.`).
* **Commenti:** Le righe che iniziano con `#` vengono ignorate dal compilatore.
* **Parole chiave vs Parole di cortesia:** Il parser aggancia i *Verbi*, i *Sostantivi strutturali* e le *Preposizioni* (`con`, `al`, `da`, `per`, `in`, `verso`, `attorno a`). Articoli (`il, lo, la, i, gli, le, un, una`) e congiunzioni (`e, ed`) servono a rendere fluido il testo per l'uomo e l'LLM, ma vengono scartati in fase di compilazione.
* **Identificatori:** I nomi assegnati agli elementi vanno sempre racchiusi tra virgolette doppie (`"nome"`).
* **Unità di misura:** Le distanze e dimensioni senza unità si intendono in **pixel (`px`)** o **percentuale (`%`)** se specificato. Il tempo si esprime in **secondi (`s`)** o **millisecondi (`ms`)**.
* Estensione riconosciuta in .joy.

---

## 2. Configurazione della Scena (Intestazione)
Ogni file deve iniziare definendo l'area di lavoro (il contenitore HTML principale):

`Prepara un palco grande [LARGHEZZA] per [ALTEZZA] con sfondo [COLORE] [OPZIONALE: a [FPS] fotogrammi].`
* *Esempio:* `Prepara un palco grande 1920 per 1080 con sfondo #0B0C10 a 60 fotogrammi.`
* *Esempio semplice:* `Prepara un palco grande 800 per 600 con sfondo nero.`

---

## 3. Creazione degli Attori (Elementi Visivi)
Gli attori sono gli elementi HTML/SVG/Canvas che popolano la scena.

**Sintassi base:**
`Crea [TIPO] chiamato "[NOME]" con [PROPRIETÀ] [POSIZIONE].`

### Tipi di Attori supportati:
1. **Forme geometriche (SVG/DOM):**
   * `un cerchio` (proprietà: `raggio [N]`)
   * `un rettangolo` / `un quadrato` (proprietà: `larghezza [N]`, `altezza [N]`, `lato [N]`, `arrotondamento [N]`)
   * `una linea` (proprietà: `da [X1, Y1] a [X2, Y2]`, `spessore [N]`)
2. **Testo (DOM):**
   * `un titolo` / `un testo` (proprietà: `testo "[STRINGA]"`, `grandezza [N]`, `font "[NOME_FONT]"`, `peso [normale|grassetto]`)
3. **Media e Contenitori:**
   * `un'immagine` (proprietà: `da "[URL_O_PERCORSO]"`, `larghezza [N]`, `altezza [N]`)
   * `un gruppo` (contenitore logico per animare più attori insieme)
4. **Sistemi ad alta densità (HTML5 Canvas):**
   * `[N] particelle` (es. `Crea 5000 particelle chiamate "neve" con dimensione 2 e colore bianco.`)

### Proprietà visive comuni:
* `colore [COLORE]` (accetta nomi italiani: `bianco`, `nero`, `rosso`, `blu`, `verde`, `giallo`, `azzurro`, `viola`, `grigio`, `trasparente` oppure codici esadecimali `#FF0055` e `rgb(...)`).
* `bordo [SPESSORE] [COLORE]`
* `opacità [0.0 - 1.0]` (default: `1.0`)
* `ombra [SFOCATURA] [COLORE]`
* `sfocatura [N]` (effetto blur)
* `livello [N]` (equivalente dello `z-index`, chi ha il numero più alto sta davanti).

### Posizionamento spaziale:
* **Assoluto:** `a [X, Y]` (es. `a 100, 250`)
* **Relativo al palco:** `al centro`, `in alto`, `in basso`, `a sinistra`, `a destra`, `in alto a sinistra`, ecc.
* **Relativo a un altro attore:**
  * `sopra a "[ALTRO]" di [N]`
  * `sotto a "[ALTRO]" di [N]`
  * `a destra di "[ALTRO]" di [N]`
  * `a sinistra di "[ALTRO]" di [N]`
  * `dentro a "[GRUPPO]"`

---

## 4. La Timeline (Gestione del Tempo)
Le azioni vengono racchiuse in blocchi temporali indendati di 2 spazi.

* **Intervallo assoluto:**
  `Dal secondo [INIZIO] al secondo [FINE]:`
* **Istante preciso (cambio immediato o durata default 0.3s):**
  `Al secondo [TEMPO]:`
* **Sequenza relativa (agganciata al blocco precedente):**
  `Poi per [DURATA] secondi:`
  `Dopo [ATTESA] secondi per [DURATA] secondi:`
* **Ciclo continuo:**
  `Per tutta la durata:` / `In loop continuo:`

---

## 5. Vocabolario dei Verbi (Azioni e Trasformazioni)
All'interno di un blocco temporale, ogni riga descrive cosa fa un attore. È possibile concatenare più azioni sulla stessa riga con `e`.

### Trasformazioni Base (CSS Transform / Web Animations API):
* **Movimento:**
  * `"[NOME]" si sposta a [X, Y].`
  * `"[NOME]" scivola verso [alto|basso|destra|sinistra] di [N].`
  * `"[NOME]" raggiunge "[ALTRO]".`
* **Rotazione:**
  * `"[NOME]" ruota di [GRADI] gradi.` (es. `360 gradi`, `-90 gradi`)
  * `"[NOME]" gira continuamente a velocità [N].`
* **Scala:**
  * `"[NOME]" si ingrandisce fino a [FATTORE].` (es. `1.5`, `2.0`)
  * `"[NOME]" si rimpicciolisce fino a [FATTORE].` (es. `0.5`, `0.0`)
  * `"[NOME]" passa da scala [A] a [B].`
* **Visibilità e Aspetto:**
  * `"[NOME]" appare sfumando.` (opacità -> 1)
  * `"[NOME]" scompare sfumando.` (opacità -> 0)
  * `"[NOME]" cambia opacità a [0.0 - 1.0].`
  * `"[NOME]" cambia colore in [COLORE].`

### Coreografie Complessi (Pre-calcolate da Rust):
* **Orbita:** `"[NOME]" orbita attorno a "[ALTRO]" con raggio [N] per [GIRI] giri.`
* **Pulsazione:** `"[NOME]" pulsa con intensità [AGGETTIVO|NUMERO] a ritmo [AGGETTIVO|BPM].`
* **Vibrazione:** `"[NOME]" trema con intensità [N].`
* **Onda:** `"[NOME]" fluttua verticalmente di [N] pixel.`
* **Scia:** `"[NOME]" lascia una scia lunga [N] di colore [COLORE].`

### Azioni Particellari (Solo per attori di tipo `particelle`):
* `"[NOME]" esplode dal centro con forza [N].`
* `"[NOME]" viene attratto verso "[ALTRO]" con forza [N].`
* `"[NOME]" cade come pioggia a velocità [N].`
* `"[NOME]" si disperde nel vento verso [destra|sinistra].`

---

## 6. Modificatori di Ritmo (Easing) e Aggettivi Semantici
Alla fine di qualsiasi verbo d'azione è possibile aggiungere `con movimento [TIPO]`.

### Curve di movimento (Easing):
* `lineare` -> `linear` (velocità costante)
* `morbido` -> `ease-in-out` (parte piano, accelera, frena dolce)
* `scatto` -> `cubic-bezier(0.16, 1, 0.3, 1)` (partenza esplosiva, frenata lunga)
* `elastico` -> effetto molla con rimbalzo finale
* `rimbalzo` -> effetto palla che cade e rimbalza

### Tabella di conversione degli Aggettivi (Costanti del Compilatore):
Per mantenere il linguaggio umano ma deterministico, gli aggettivi corrispondono a valori esatti:
* **Velocità / Ritmo:** `lentamente` (0.5x), `normalmente` (1.0x), `velocemente` (2.0x), `freneticamente` (4.0x).
* **Intensità:** `leggermente` (0.2), `moderatamente` (0.5), `intensamente` (1.0), `estremamente` (2.0).

---

## 7. Esempio Completo Valido

```text
# Test completo v1.0
Prepara un palco grande 1280 per 720 con sfondo #0F172A.

Crea un cerchio chiamato "sole" con raggio 60 e colore #F59E0B al centro.
Crea un cerchio chiamato "terra" con raggio 18 e colore #38BDF8 a destra di "sole" di 200.
Crea un titolo chiamato "testo" con testo "SISTEMA ORBITALE", grandezza 36 e colore bianco in alto.

Dal secondo 0 al secondo 1:
  "testo" appare sfumando e scivola verso basso di 40 con movimento morbido.

Dal secondo 0 al secondo 8:
  "sole" pulsa leggermente a ritmo lentamente.
  "terra" orbita attorno a "sole" con raggio 200 per 2 giri con movimento lineare.

Dal secondo 7 al secondo 8:
  "testo" scompare sfumando.
  "sole" si ingrandisce fino a 15 e cambia colore in bianco con movimento scatto.
```

---

## Appendice A: Note di implementazione v0.1 (MVP)

Questa appendice **non aggiunge sintassi**: precisa i valori predefiniti e le scelte che la
specifica lascia aperte, così che il comportamento del compilatore sia deterministico.

### A.1 Lessico
* Una frase per riga. Le righe indentate (spazi o tab) appartengono all'ultimo blocco temporale.
* Le parole sono confrontate senza maiuscole né accenti (`opacità` = `opacita`).
* Le elisioni sono separate (`un'immagine` → `immagine`); sono ammesse le virgolette tipografiche `“ ”`.
* I decimali usano il punto (`1.5`).

### A.2 Valori predefiniti degli attori
| Tipo | Predefiniti |
|------|-------------|
| `cerchio` | `raggio 50` |
| `quadrato` | `lato 100` |
| `rettangolo` | `larghezza 160`, `altezza 100` (oppure `lato` per entrambe) |
| `titolo` | `grandezza 48`, `peso grassetto` (il `testo "..."` è obbligatorio) |
| `testo` | `grandezza 24`, `peso normale` (il `testo "..."` è obbligatorio) |
| tutti | `colore bianco`, `opacità 1.0`, `livello 0`, posizione `al centro` |

* Le coordinate (`a X, Y`, `si sposta a X, Y`) indicano il **centro** dell'attore.
* Le ancore del palco usano un margine del 10%: `in alto` = (50%, 10%), `in basso a destra` = (90%, 90%), ecc.
* Le distanze relative (`a destra di "sole" di 200`) sono misurate tra i centri.
* Lo sfondo del palco, se omesso, è `nero`.

### A.3 Timeline
* `Al secondo T:` dura 0.3 secondi. `Poi per D secondi:` inizia alla fine del blocco precedente
  (nell'ordine del file); `Dopo A secondi per D secondi:` aggiunge un'attesa A.
* Due azioni dello stesso attore che agiscono sulla stessa grandezza (opacità, posizione, scala,
  rotazione) **non** possono sovrapporsi nel tempo: è un errore di compilazione.
* `appare sfumando` porta l'opacità al valore dichiarato dall'attore (default 1.0) se è maggiore
  di 0; se l'opacità dichiarata è `0`, la porta a `1` (come dice il §5, «opacità → 1»). Se è la
  prima azione di opacità dell'attore, l'attore parte invisibile.
  *Modificato in v0.2.1: prima andava al valore dichiarato anche se 0, quindi un attore creato con
  `opacità 0` e animato con `appare sfumando` restava invisibile.*
* `ruota di N gradi` e `scivola verso` sono relativi allo stato raggiunto; `si sposta a` è assoluto.
* `con movimento X` in fondo alla riga vale anche per le azioni precedenti della stessa riga
  che non ne specificano uno. Il movimento predefinito è `morbido`.
* `pulsa`: intensità predefinita `moderatamente` (0.5), ritmo predefinito `normalmente` (60 BPM).
  La scala oscilla tra 1 e `1 + 0.25 × intensità`; il ritmo è in battiti al minuto (`lentamente` = 30 BPM)
  e il numero di battiti viene arrotondato per riempire esattamente il blocco.

### A.4 Funzioni non ancora supportate nella v0.1
Producono un errore esplicito "non ancora supportato": tipi `linea`, `immagine`, `gruppo`,
`particelle`; proprietà `ombra`, `sfocatura`; blocchi `Per tutta la durata:` e `In loop continuo:`;
verbi `orbita`, `trema`, `fluttua`, `lascia una scia`, `gira`, `raggiunge`, `passa da scala`,
`cambia colore` e le azioni particellari.

**Pianificate per la v0.2** (sintassi e semantica nell'Appendice B):
* tipo `gruppo` → **B.3** (la posizione `dentro a "[GRUPPO]"` del §3 viene invece **rimossa**, vedi B.3.1);
* verbo `orbita` → **B.4.1**;
* verbo `cambia colore` → **B.4.2**.

Restano non supportate anche nella v0.2: tipi `linea`, `immagine`, `particelle`; proprietà
`ombra`, `sfocatura`; blocchi `Per tutta la durata:` e `In loop continuo:`; verbi `trema`,
`fluttua`, `lascia una scia`, `gira`, `raggiunge`, `passa da scala` e le azioni particellari.

---

## Appendice B: Specifica v0.2 (approvata per lo sviluppo)

Questa appendice **aggiunge sintassi**: definisce le funzioni approvate per la v0.2. Le sezioni
1-7 e l'Appendice A restano valide, salvo dove questa appendice dice esplicitamente il contrario
(solo la posizione `dentro a "[GRUPPO]"`, vedi B.3.1). Ogni regola è deterministica: a parità di
sorgente il compilatore produce sempre lo stesso HTML.

### B.0 Regole comuni

#### B.0.1 Canali e sovrapposizioni
La v0.2 aggiunge il canale `colore` ai quattro della v0.1. La regola di A.3 vale per tutti i canali:
due azioni dello stesso attore (o gruppo) sullo stesso canale non possono sovrapporsi nel tempo.
Attori diversi, e quindi anche un gruppo e i suoi membri, hanno canali indipendenti.

| Canale | Proprietà CSS | Elemento animato | Verbi |
|--------|---------------|------------------|-------|
| `opacita` | `opacity` | wrapper `.attore` | `appare sfumando`, `scompare sfumando`, `cambia opacità a` |
| `traslazione` | `translate` | wrapper `.attore` | `si sposta a`, `scivola verso`, `entra da` (B.2), `esce verso` (B.2), `orbita` (B.4.1) |
| `rotazione` | `rotate` | wrapper `.attore` | `ruota di` |
| `scala` | `scale` | wrapper `.attore` | `si ingrandisce fino a`, `si rimpicciolisce fino a`, `pulsa` |
| `colore` (nuovo) | `background-color` o `color` | `.forma` | `cambia colore in` (B.4.2) |

Il messaggio d'errore delle sovrapposizioni per il nuovo canale usa la descrizione «il colore».

#### B.0.2 Posizione corrente
La **posizione dichiarata** è il centro risolto alla creazione (per un gruppo: il suo pivot, B.3.3).
La **posizione corrente** all'istante `t` è la posizione dichiarata più il valore del canale
`traslazione` in `t`. Quando serve «la posizione all'inizio di un blocco» (istante `t0`) si usa il
limite sinistro: il valore subito prima di `t0`. Un segmento che inizia esattamente in `t0` non conta
ancora; un segmento iniziato prima di `t0` e ancora in corso si valuta in `t0` con i keyframe già
generati, applicando la curva di easing del keyframe sinistro (cubic-bezier: risoluzione numerica
con tolleranza 1e-6; curve campionate: interpolazione lineare tra i campioni).

Tutte le coordinate (dichiarate, correnti e quelle dei verbi `si sposta a`, `entra da`, `esce verso`,
`orbita`) sono espresse nel **sistema del genitore** (B.3.5): per un attore che non appartiene a
nessun gruppo è il palco. Le trasformazioni dei gruppi antenati non vengono mai considerate.

#### B.0.3 Salto istantaneo
Alcune azioni (`entra da` non iniziale, B.2.3; `orbita` con distanza diversa dal raggio, B.4.1)
devono spostare l'attore di colpo all'inizio del segmento. Il salto avviene così:
* keyframe in `t0` con il valore corrente e curva `steps(1, end)` (nuova variante di `Curva`, che
  `@renderer` emette come `steps(1,end)`);
* keyframe in `t0 + δ` con il valore di partenza dell'azione e la curva dell'azione;
* `δ = max(0.001 s, 2·10⁻⁶ × durata_scena)`, limitato a `(t1 − t0) / 2`. Il valore minimo garantisce
  che i due offset restino distinti anche dopo l'arrotondamento a 4 decimali in percentuale.

Il movimento vero e proprio occupa quindi `[t0 + δ, t1]`. Se la distanza del salto è ≤ 0.5 px, il
salto viene omesso. Il salto non è nascosto: se l'attore è visibile, si vede «teletrasportarsi».
Per renderlo invisibile basta far precedere l'azione da `esce verso` o `scompare sfumando`.

#### B.0.4 Ingombro
L'ingombro è il riquadro allineato agli assi (AABB) che contiene l'attore. Lo usano il pivot dei gruppi
(B.3.3) e il fuori campo (B.2.1). È descritto da quattro estensioni misurate dal centro:
`sinistra`, `destra`, `alto`, `basso` (≥ 0; per gli attori semplici sono simmetriche).

**Dimensioni di base `(w, h)`** (prima di scala e rotazione):
* `cerchio`: `w = h = 2 × raggio`.
* `rettangolo` / `quadrato`: `w = larghezza`, `h = altezza`.
* Il **bordo** delle forme è già incluso: il generatore usa `box-sizing:border-box` e lo disegna
  dentro le dimensioni dichiarate. Se `@renderer` cambiasse questa scelta, la regola andrebbe
  aggiornata.
* `testo` / `titolo`: la larghezza reale dipende dal font e non è nota a compile time. Si usa una
  **stima prudente** (sovrastima volontaria):
  * `w = grandezza × max_r Σ_c k(c) + 2 × spessore_bordo`, dove `k(c) = 1.0` per i caratteri con
    codice < U+1100 e `k(c) = 1.25` per gli altri (CJK, emoji, ...); la somma è per riga e si prende
    la riga più lunga (righe separate da `\n`);
  * `h = 1.5 × grandezza × numero_righe + 2 × spessore_bordo`. L'1.5 copre l'interlinea 1.2 del
    generatore e gli accenti o i discendenti che escono dalla riga.

  Un glifo latino è largo in pratica tra 0.5 e 0.95 em, quindi la stima garantisce l'uscita
  completa per i font comuni. Un font decorativo eccezionalmente largo può superarla (limite noto
  e documentato).

**Estensione di un canale**: è l'intervallo `[min, max]` dei valori che il canale assume nell'intera
scena. Si calcola come minimo e massimo dei valori di tutti i keyframe precalcolati. In più, per ogni
tratto tra due keyframe con curva `elastico` (che supera il bersaglio), si aggiunge il punto
`partenza + 1.36 × (arrivo − partenza)`: 0.36 è la sovraelongazione massima della curva (0.354),
arrotondata per eccesso. Le altre curve (`lineare`, `morbido`, `scatto`, `rimbalzo`) non superano il
bersaglio. Un canale senza segmenti ha come estensione il solo valore iniziale.

**Ingombro iniziale** (per il pivot dei gruppi): stato a `t = 0` prima di ogni azione, alla posizione
dichiarata, con la scala iniziale `s0` e la rotazione iniziale `r0` (B.1):
* `ex = s0 × (|w/2 · cos r0| + |h/2 · sin r0|)` e `ey = s0 × (|w/2 · sin r0| + |h/2 · cos r0|)`;
* estensioni: `sinistra = destra = ex`, `alto = basso = ey`. Per il cerchio `ex = ey = s0 × raggio`.

**Ingombro prudente** (per il fuori campo):
* `s_rif = max(|min|, |max|)` dell'estensione del canale `scala`;
* se il canale `rotazione` **non** ha segmenti, si usa la formula esatta dell'ingombro iniziale con
  `s_rif` al posto di `s0` (e `r0` costante);
* se il canale `rotazione` ha segmenti, si usa il cerchio circoscritto:
  `ex = ey = s_rif × √((w/2)² + (h/2)²)` (per il cerchio: `s_rif × raggio`).

Gli ingombri dei gruppi sono definiti in B.3.4.

#### B.0.5 Ordine di calcolo (per `@compiler`)
1. Risoluzione delle posizioni dichiarate (A.2 e B.3.6), compresi i pivot dei gruppi dall'interno
   verso l'esterno.
2. Canali `opacita`, `rotazione`, `scala`, `colore` di tutti gli attori e gruppi. Non dipendono da
   altri attori.
3. Canale `traslazione`, un livello di annidamento alla volta, dal più interno al più esterno. Un
   gruppo si calcola dopo tutti i suoi discendenti, perché il suo ingombro prudente usa le loro
   estensioni (B.3.4). Dentro un livello, i segmenti di traslazione di tutti gli attori di quel
   livello si elaborano in ordine di istante di inizio crescente (a parità di istante, in ordine di
   riga). Così `orbita` (B.4.1) trova già calcolata la posizione dell'attore attorno a cui gira.
   Una dipendenza circolare è impossibile, perché un segmento dipende solo da valori di istanti
   strettamente precedenti.

---

### B.1 Stato iniziale dichiarativo: `scala` e `rotazione`
**Implementano:** `@parser` (proprietà), `@compiler` (valore iniziale dei canali), `@renderer`
(regola CSS statica), `@security` (limiti, riusa `MAX_SCALA` e `MAX_GRADI`).

**Sintassi:** due nuove proprietà di `Crea`, che si aggiungono a `opacità`, in qualsiasi ordine e
con gli stessi separatori (`con`, `e`, `,`):
* `scala [N]`: N ≥ 0, N ≤ `MAX_SCALA` (1000). Predefinita: `1`.
* `rotazione [GRADI] [gradi]`: numero reale (anche negativo), |GRADI| ≤ `MAX_GRADI`. La parola
  `gradi`/`grado` è facoltativa. Predefinita: `0`. Il valore non viene normalizzato
  (`rotazione 370` resta 370).

Se ripetute nella stessa frase vale l'ultima, come per tutte le proprietà della v0.1. Sono ammesse
anche sui gruppi (B.3.2).

**Esempi validi:**
```text
Crea un quadrato chiamato "stella" con lato 24, scala 0 e rotazione 45 al centro.
Crea un titolo chiamato "logo" con testo "JOY++", rotazione -8 gradi e scala 1.2 in alto.
```

**Semantica:**
* Lo stato a `t = 0` dei canali diventa `scala = s0` e `rotazione = r0`, al posto di 1 e 0.
* `si ingrandisce fino a F` / `si rimpicciolisce fino a F` sono **assoluti**: portano la scala da quella
  corrente (inizialmente `s0`) a F.
* `ruota di G gradi` è **relativo**: porta la rotazione da quella corrente (inizialmente `r0`) a
  `corrente + G`.
* `pulsa` oscilla tra la scala corrente `b` (inizialmente `s0`) e `b × (1 + 0.25 × intensità)`, poi
  torna a `b`. Con `b = 0` la pulsazione resta a 0: è accettata senza errore.
* `appare sfumando` agisce solo sul canale `opacita`: l'attore appare già con `s0` e `r0`. L'ingresso
  «a scatto» si ottiene combinando `scala 0`, `si ingrandisce fino a 1` e `appare sfumando`.
* Le posizioni relative (A.2) restano misurate tra i centri: scala e rotazione iniziali non le
  cambiano.
* Uscita CSS: se `s0 ≠ 1` o `r0 ≠ 0`, il generatore aggiunge `scale:s0` e/o `rotate:r0deg` alla
  regola statica del wrapper `#aN`, anche quando l'attore ha animazioni. Il primo keyframe
  contiene comunque lo stesso valore.

**Errori:**
* `la scala iniziale non può essere negativa` (suggerimento: «usa un valore da 0 in su, es. «scala 0.5»»).
* `la scala iniziale è troppo grande (massimo 1000)`; `la rotazione iniziale è troppo grande (massimo 360000 gradi)`.
* Numero mancante: `mi aspettavo un numero dopo «scala»` / `dopo «rotazione»`.

**Avvisi:** `rotazione` diversa da 0 su un `cerchio` produce l'avviso di B.6.

---

### B.2 Ingresso e uscita dai bordi: `entra da` / `esce verso`
**Implementano:** `@parser` (verbi), `@compiler` (ingombro, fuori campo, salto). `@renderer` non deve
fare nulla: si usa il canale `traslazione` esistente, più la curva `steps(1,end)` di B.0.3.

**Sintassi:**
* `"[NOME]" entra da [sinistra|destra|alto|basso] [con movimento TIPO].`
* `"[NOME]" esce verso [sinistra|destra|alto|basso] [con movimento TIPO].`
* Dopo `entra` sono ammesse anche le preposizioni articolate `dal`, `dall'` e `dalla`
  (`entra dall'alto`, `entra dal basso`, `entra dalla destra`). Dopo `verso` gli articoli vengono
  scartati come sempre (`esce verso l'alto`).
* Si possono concatenare con `e` come gli altri verbi. Movimento predefinito: `morbido` (A.3).

**Esempi validi:**
```text
"titolo" entra dall'alto con movimento scatto.
"pannello" entra da sinistra e appare sfumando.
"sistema" esce verso destra con movimento morbido.
```

#### B.2.1 Fuori campo
Il palco è il rettangolo `[0, L] × [0, H]`. Un attore è **fuori campo** quando il suo ingombro
prudente (B.0.4; per i gruppi B.3.4) è interamente fuori dal palco dal lato indicato, con un margine
di sicurezza `M = 2 px`. Le coordinate del centro fuori campo (`xF`, `yF`) valgono:

| Lato | Coordinata fuori campo del centro |
|------|-----------------------------------|
| `sinistra` | `xF = −destra − M` |
| `destra` | `xF = L + sinistra + M` |
| `alto` | `yF = −basso − M` |
| `basso` | `yF = H + alto + M` |

(`sinistra`, `destra`, `alto`, `basso` sono le estensioni dell'ingombro prudente.) Le coordinate
sono nel sistema del genitore (B.0.2). Per un membro di un gruppo il fuori campo è calcolato come se
gli antenati non fossero trasformati: se il gruppo è spostato, ruotato o scalato, il membro può non
risultare davvero fuori dal palco. Per far entrare o uscire un insieme, conviene animare il gruppo.

#### B.2.2 `esce verso`
Porta l'attore dalla posizione corrente in `t0` al fuori campo del lato indicato. Cambia solo la
coordinata dell'asse del movimento (`x` per sinistra/destra, `y` per alto/basso); l'altra resta quella
corrente. Nel canale diventa un segmento «verso» sull'asse: il valore di arrivo è
`(xF − x_dichiarata, dy_corrente)` oppure `(dx_corrente, yF − y_dichiarata)`.

#### B.2.3 `entra da`
Porta l'attore **dal fuori campo alla sua posizione dichiarata** (traslazione finale `(0, 0)`).
Il punto di partenza è la posizione dichiarata spostata sull'asse del lato: `(xF, y_dichiarata)` per
sinistra/destra, `(x_dichiarata, yF)` per alto/basso.
* Se è il **primo segmento di traslazione** dell'attore, il valore del canale a `t = 0` è già il punto
  di partenza: l'attore parte fuori campo e resta lì fino a `t0`. È analogo ad `appare sfumando`
  (A.3).
* Altrimenti, in `t0` l'attore salta al punto di partenza (B.0.3) e poi si muove verso la posizione
  dichiarata. Uso tipico: `esce verso destra`, poi `entra da sinistra`.

Entrambi i verbi usano il canale `traslazione`, quindi non possono sovrapporsi a `si sposta a`,
`scivola verso`, `orbita` né tra loro (B.0.1).

**Errori:**
* `direzione non valida: «X»` (suggerimento: «usa sinistra, destra, alto o basso», con «forse
  intendevi …?» se il nome è simile).
* `mi aspettavo «da» dopo «entra»` (es. «"titolo" entra da sinistra.») e `mi aspettavo «verso» dopo «esce»`.
* `coordinate fuori dai limiti` se il punto fuori campo supera `MAX_COORDINATA` (succede solo con
  attori enormi).

---

### B.3 Gruppi
**Implementano:** `@parser` (frase del gruppo), `@compiler` (albero, pivot, ingombri, controlli),
`@renderer` (wrapper annidati), `@security` (nuovo limite `MAX_PROFONDITA_GRUPPI = 16`; i gruppi
contano nel limite `MAX_ATTORI`; escape del nome in `data-nome`).

#### B.3.1 Riconciliazione con il §3
* `un gruppo` (§3, «contenitore logico per animare più attori insieme») **resta**: è il tipo `gruppo`
  di `Crea`, con la sintassi di B.3.2.
* La posizione `dentro a "[GRUPPO]"` (§3) viene **rimossa**: l'appartenenza si dichiara solo
  nella frase del gruppo. Motivo: il pivot del gruppo dipende dai suoi membri (B.3.3), quindi un
  membro posizionato «dentro» il gruppo creerebbe una dipendenza circolare; inoltre `dentro a` non
  indica nessun punto. Usarla produce l'errore `la posizione «dentro a» non esiste: i membri si
  elencano nella frase del gruppo`, con il suggerimento
  `es. «Crea un gruppo chiamato "g" con "a" e "b".»`.

#### B.3.2 Sintassi
`Crea un gruppo chiamato "[NOME]" con "[MEMBRO1]", "[MEMBRO2]" [e "[MEMBRO3]"...] [PROPRIETÀ].`
* I membri sono stringhe e vanno elencati **tutti subito dopo `con`**, separati da `,` o `e`. La lista
  finisce al primo elemento che non è una stringa; da lì iniziano le proprietà.
* Proprietà ammesse: `opacità`, `livello`, `scala`, `rotazione` (B.1). Predefiniti: opacità 1,
  livello 0, scala 1, rotazione 0.
* Un gruppo **non** ha posizione propria: il suo centro si calcola dai membri (B.3.3).
* Un membro può essere un attore o un altro gruppo (annidamento).

**Esempi validi:**
```text
Crea un gruppo chiamato "logo" con "lettera_j", "lettera_o" e "lettera_y".
Crea un gruppo chiamato "sistema" con "sole" e "terra", scala 0.8 e livello 1.
Crea un gruppo chiamato "scena" con "sistema", "titolo" e opacità 0.9.
```

#### B.3.3 Centro del gruppo (pivot)
Il pivot `P` è il **centro dell'AABB che contiene gli ingombri iniziali** (B.0.4) di tutti i membri,
ciascuno alla propria posizione dichiarata. Se un membro è a sua volta un gruppo, si usa il suo
ingombro iniziale (B.3.4). `P` è:
* il punto attorno a cui il gruppo ruota e si scala;
* la sua posizione dichiarata, usata da `si sposta a` (che porta `P` alle coordinate date),
  `entra da`, dalle posizioni relative e da `orbita`.

Le posizioni dichiarate dei membri sono quelle **prima** della trasformazione iniziale del gruppo: un
gruppo con `scala 0.8` mostra i membri già avvicinati a `P` dell'80% fin da `t = 0`.

#### B.3.4 Ingombri del gruppo
Si parte da un riquadro in coordinate relative a `P`, poi si applica la trasformazione del gruppo.
* **Iniziale:** si prende l'unione degli ingombri iniziali dei membri. Se ne ruotano i 4 vertici di
  `r0` attorno a `P` e si scalano di `s0`; l'ingombro è l'AABB del risultato (in generale
  asimmetrico).
* **Prudente:** per ogni membro `m` si prende il suo ingombro prudente, spostato di
  `(posizione_dichiarata_m − P)` e allargato dell'estensione del canale `traslazione` di `m`
  (l'intervallo `[dx_min, dx_max] × [dy_min, dy_max]`, B.0.4). Poi si fa l'unione. Infine:
  * se il canale `rotazione` del gruppo non ha segmenti, si ruotano i 4 vertici di `r0`, si scalano di
    `s_rif` del gruppo e si prende l'AABB;
  * altrimenti si usa il cerchio di raggio `s_rif × (massima distanza tra P e i 4 vertici)`.

#### B.3.5 Semantica e composizione
* **DOM:** il gruppo è un wrapper `<div class="attore gruppo" id="aN" data-nome="…">` con le stesse
  regole di `.attore` (`position:absolute; width:0; height:0; transform-origin:0 0`) e
  `left/top = P − P_genitore`, dove `P_genitore` è l'origine del palco `(0, 0)` per i gruppi di primo
  livello. I membri ne sono figli, con `left/top = posizione_dichiarata − P`. Il gruppo non ha `.forma`.
* **Composizione:** le animazioni del gruppo (sul suo wrapper) si compongono con quelle dei membri
  (sui loro wrapper). La trasformazione visibile di un membro è quella del gruppo applicata dopo la
  sua. L'opacità del gruppo moltiplica quella dei membri (`opacity` CSS di gruppo).
* **Sistema del genitore:** le coordinate delle azioni di un membro (`si sposta a`, `orbita`, ...)
  sono espresse nel sistema del gruppo **non trasformato**, cioè in coordinate del palco come se il
  gruppo fosse fermo, con scala 1 e rotazione 0 (B.0.2).
* **Azioni ammesse su un gruppo:** opacità, traslazione (incluse `entra da`, `esce verso`, `orbita`),
  rotazione, scala. `appare sfumando` su un gruppo lo fa partire invisibile, come per un attore (A.3),
  e porta l'opacità del gruppo a quella dichiarata nella sua frase, oppure a `1` se è `0`
  (*modificato in v0.2.1: prima andava al valore dichiarato anche se 0*).
  `cambia colore` non è ammesso (B.3.8).
* **Annidamento:** sì, fino a `MAX_PROFONDITA_GRUPPI = 16` livelli. Un gruppo di primo livello ha
  profondità 1.
* **Appartenenza unica:** ogni attore o gruppo appartiene al massimo a **un** gruppo.
* **Ordine di dichiarazione:** i membri devono essere creati in righe **precedenti** a quella del
  gruppo. Questo rende impossibili i cicli di appartenenza.
* **`livello` e ordine di disegno:** ogni wrapper ha `z-index` e forma un contesto di impilamento.
  * Il `livello` di un membro ordina il membro **solo tra i figli del suo gruppo**: un membro con
    `livello 10` non può stare sopra un attore esterno se il suo gruppo ha un livello più basso.
  * Il `livello` del gruppo lo ordina tra i suoi fratelli (gli altri figli dello stesso genitore).
  * A parità di livello conta l'ordine nel DOM. Ogni genitore elenca i suoi figli nell'ordine di
    creazione e un gruppo occupa il posto del suo **discendente creato per primo** (riga minima).
    Così l'ordine di disegno della v0.1 resta invariato per gli attori senza gruppo.

#### B.3.6 Posizioni relative che riferiscono un gruppo
* `a destra di "[GRUPPO]" di N` (e le altre forme) usano il pivot `P` del gruppo come centro di
  riferimento.
* Il risolutore delle posizioni (A.2) aggiunge un arco «il gruppo dipende da ciascun membro». I
  riferimenti in avanti sono ammessi come nella v0.1; un ciclo (es. un membro posizionato rispetto al
  proprio gruppo) produce l'errore esistente `posizioni circolari: …`.
* Riferire un membro di un gruppo da fuori usa la sua posizione dichiarata (B.3.3). Vale la stessa
  regola della v0.1: le posizioni relative si risolvono sulle posizioni iniziali.

#### B.3.7 Implementazione (indicativa, per `@compiler`)
`Forma::Gruppo { membri: Vec<String> }`. Gli attori diventano un albero (genitore opzionale per
nodo). `AttoreCompilato` riceve `genitore: Option<usize>` e le coordinate `x`, `y` relative al
genitore. Il generatore emette l'albero in profondità.

#### B.3.8 Errori
* `non esiste nessun attore o gruppo chiamato "x"` (con «forse intendevi …?»).
* `"x" è creato alla riga N, dopo il gruppo "g"` (suggerimento: «crea il gruppo dopo tutti i suoi
  membri»).
* `"x" fa già parte del gruppo "g1" (riga N)` (suggerimento: «un attore può appartenere a un solo
  gruppo: raggruppa i gruppi, es. «Crea un gruppo chiamato "tutto" con "g1" e "g2".»»).
* `"x" compare due volte nel gruppo "g"`.
* `un gruppo non può contenere sé stesso`.
* `il gruppo "g" deve contenere almeno un membro` (suggerimento: `es. «Crea un gruppo chiamato "g" con "a" e "b".»`).
* `i membri del gruppo vanno elencati tutti subito dopo «con»` (stringa trovata tra le proprietà).
* `un gruppo non ha una posizione propria: il suo centro è calcolato dai membri` (suggerimento:
  «sposta i membri, oppure anima il gruppo con «si sposta a»»).
* `la proprietà «colore» non si applica a un gruppo` (vale anche per `bordo`, `raggio`, `testo`, ...;
  suggerimento: «assegnala ai singoli membri»).
* `"g" è un gruppo e non ha un colore da cambiare` (verbo `cambia colore`; suggerimento: «cambia il
  colore dei singoli membri»).
* `gruppi annidati troppo in profondità (massimo 16 livelli)`.
* La posizione `dentro a`: vedi B.3.1.

---

### B.4 Verbi del §5 precisati

#### B.4.1 `orbita`
**Implementano:** `@parser`, `@compiler` (precalcolo dei campioni). `@renderer` non deve fare nulla:
l'orbita produce keyframe del canale `traslazione`.

**Sintassi:**
`"[NOME]" orbita attorno a "[ALTRO]" con raggio [N] per [GIRI] giri [in senso orario|antiorario] [con movimento TIPO].`
* `giro` al singolare è accettato (`per 1 giro`). L'ordine delle parti è fisso.
* Verso predefinito: **antiorario**, come appare sullo schermo (il verso dei pianeti visti dal polo
  nord celeste).

**Esempi validi:**
```text
"terra" orbita attorno a "sole" con raggio 200 per 2 giri con movimento lineare.
"luna" orbita attorno a "terra" con raggio 40 per 0.5 giri in senso orario.
```

**Semantica:**
* **Centro statico:** il centro dell'orbita `C` è la posizione corrente di ALTRO in `t0` (B.0.2) e
  resta fermo per tutto il segmento. Motivo: i keyframe sono precalcolati per canale, e seguire un
  centro in movimento richiederebbe di comporre in Rust le curve di easing di due attori. Il caso
  «orbita attorno a qualcosa che si muove» si ottiene già con i gruppi: si mettono ALTRO e NOME in un
  gruppo e si anima il gruppo.
* Se ALTRO è il **gruppo genitore diretto** di NOME, `C` è il pivot del gruppo nel suo sistema, cioè
  `P`, costante.
* **Angolo di partenza:** `θ0 = atan2(yN − Cy, xN − Cx)`, in coordinate dello schermo (y verso il
  basso), dove `(xN, yN)` è la posizione corrente di NOME in `t0`. Se la distanza `d` tra NOME e `C` è
  < 0.5 px, `θ0 = 0` (cioè a destra di `C`).
* **Distanza diversa dal raggio:** l'orbita usa sempre il raggio dichiarato. Se `|d − raggio| > 0.5 px`,
  in `t0` NOME salta (B.0.3) sul punto `C + raggio·(cos θ0, sin θ0)`; altrimenti parte da lì senza salto.
* **Verso:** `σ = +1` per `orario` (con y verso il basso, θ crescente è orario sullo schermo),
  `σ = −1` per `antiorario`.
* **Precalcolo:** si usa un numero di campioni per giro tale che l'errore della corda (sagitta) sia
  ≤ 0.5 px:
  `n_giro = clamp(⌈2π / (2·acos(1 − 0.5/raggio))⌉, 36, 360)` per raggio > 1 e `n_giro = 36` per
  raggio ≤ 1.
  Campioni totali `N = max(1, ⌈n_giro × GIRI⌉)`. Il campione `k = 0..N` cade nell'istante
  `t0' + k·(t1 − t0')/N` (con `t0' = t0 + δ` se c'è il salto, altrimenti `t0`) e vale
  `C + raggio·(cos θk, sin θk) − posizione_dichiarata`, con `θk = θ0 + σ·2π·GIRI·E(k/N)`.
  `E` è la curva di easing valutata in Rust, e tra i campioni l'interpolazione è lineare.
  Anche i giri frazionari seguono questa formula: `0.25` è un quarto di giro.
* Al termine, la posizione corrente è il punto di arrivo sul cerchio, che le azioni successive usano
  come punto di partenza. L'orbita **non** ruota l'attore su sé stesso.

**Errori:**
* `"x" non può orbitare attorno a sé stesso`.
* `il raggio dell'orbita deve essere maggiore di 0` (e ≤ `MAX_COORDINATA`).
* `il numero di giri deve essere maggiore di 0` (suggerimento: «per invertire il verso usa «in senso
  orario» o «in senso antiorario»»).
* `"x" e "y" appartengono a gruppi diversi: l'orbita richiede lo stesso gruppo`. ALTRO deve essere un
  fratello di NOME (stesso genitore, anche il palco) oppure il suo gruppo genitore diretto.
  Suggerimento: «metti entrambi nello stesso gruppo, oppure fai orbitare il gruppo».
* `orbita troppo lunga: N fotogrammi chiave (massimo 10000)` (suggerimento: «riduci i giri»).
* Attore inesistente: errore standard con «forse intendevi …?».

#### B.4.2 `cambia colore`
**Implementano:** `@parser` (colore, riusa il lessico di §3), `@compiler` (nuovo `Canale::Colore`),
`@renderer` (keyframe sulla `.forma`), `@security` (il colore è riemesso dai valori numerici, mai dal
testo utente).

**Sintassi:** `"[NOME]" cambia colore in [COLORE] [con movimento TIPO].`
`[COLORE]` accetta le stesse forme di §3: nomi italiani, `#RGB`, `#RRGGBB`, `#RRGGBBAA`, `rgb(...)` e
`trasparente`.

**Esempi validi:**
```text
"sole" cambia colore in #FDE68A con movimento morbido.
"titolo" cambia colore in rosso e scivola verso basso di 20.
```

**Semantica:**
* Nuovo canale `colore`, che parte dal `colore` dichiarato dall'attore (predefinito bianco). Anima la
  proprietà visibile della forma: `background-color` per cerchi e rettangoli, `color` per testi e
  titoli. Il colore del `bordo` non cambia.
* I keyframe (`@keyframes aN-colore`) sono applicati all'elemento `#aN>.forma`, non al wrapper.
  Durata, `linear both` e curve per keyframe sono gli stessi degli altri canali.
* L'interpolazione è quella del browser per colori in sintassi classica (sRGB con alfa
  premoltiplicato). Se `elastico` porta i valori fuori gamma, il browser li limita.
* **Prestazioni:** `background-color`/`color` **non** sono proprietà composite. Il cambio non provoca
  un nuovo layout (relayout) ma ridisegna la forma (repaint) a ogni fotogramma per tutta la durata
  del segmento. `@renderer` **non** deve aggiungerli a `will-change`.
* **Sovrapposizioni:** due `cambia colore` dello stesso attore non possono sovrapporsi (canale
  `colore`, B.0.1). Sono indipendenti da opacità, traslazione, rotazione e scala.

**Errori:**
* `colore non valido: …` (stessi messaggi di §3).
* `mi aspettavo «in» dopo «cambia colore»` (es. «"sole" cambia colore in rosso.»).
* Sui gruppi: vedi B.3.8.

---

### B.5 Mini-player nell'HTML generato
**Implementano:** `@renderer` (HTML/CSS/JS inline), `@exporter` (opzione CLI), `@security` (revisione
dello script).

**Interfaccia:**
* Una barra discreta in basso al centro della finestra, con: un pulsante **Play/Pausa** (un solo
  pulsante che cambia icona), un pulsante **Replay**, uno **scrubber** `<input type="range">` e
  l'etichetta del tempo (`1.2 / 8.5 s`, un decimale). Le icone sono caratteri Unicode (▶, ❚❚, ↺):
  niente immagini né font esterni.
* **A scomparsa:** è nascosta al caricamento (`opacity:0` con transizione di 0.2 s). Compare quando il
  puntatore si muove sulla pagina, a un tocco, quando un suo elemento riceve il focus (Tab) e a ogni
  scorciatoia. Scompare dopo 2.5 s senza interazioni, se il puntatore non è sopra la barra e nessun
  suo elemento ha il focus.
* Accessibilità: `<button type="button">` con `aria-label` in italiano («Riproduci», «Pausa»,
  «Ricomincia», «Posizione nel tempo»).

**Non deve alterare il disegno della scena:**
* La barra è fuori da `#palco`, come fratella successiva nel `<body>`, con `position:fixed`. Così è
  fuori dal flusso e non sposta la centratura flex del palco né la sua scala.
* Non modifica stili, classi o elementi del palco. Agisce solo sullo stato di riproduzione delle
  animazioni. Senza interazioni la scena si comporta esattamente come senza player (parte da sola al
  caricamento).

**Logica (Web Animations API):**
* Animazioni gestite: `document.getAnimations()` filtrate su quelle il cui `effect.target` è dentro
  `#palco`.
* Play: `play()` su tutte. Se la scena è finita (`currentTime ≥ durata`), Play equivale a Replay.
* Pausa: `pause()` su tutte.
* Replay: `currentTime = 0`, poi `play()`.
* Scrubber: `min=0`, `max=durata in ms`, `step=10`. Trascinandolo, le animazioni vanno in pausa e
  ricevono `currentTime = valore`.
* Aggiornamento di scrubber ed etichetta: un ciclo `requestAnimationFrame` attivo solo mentre la barra
  è visibile.
* Se `document.getAnimations` non esiste, la barra non viene mostrata (degradazione silenziosa).
* Se la durata della scena è 0 (nessun blocco temporale), il player non viene emesso.

**Scorciatoie da tastiera** (sul `document`; ignorate se il target dell'evento è un `button` o un
`input`, per non duplicare il comportamento nativo):
* `Spazio`: Play/Pausa (con `preventDefault`, per non far scorrere la pagina);
* `R`: Replay;
* `←` / `→`: indietro / avanti di 1 s, senza cambiare lo stato di riproduzione.

**Sicurezza:** lo script è un testo **costante** del compilatore. L'unico valore variabile è la durata,
un numero formattato con `numero()`. Non contiene, né legge da `data-nome`, nessuna stringa
dell'utente (nomi, testi, font). È solo JS/CSS inline, senza risorse esterne, con identificatori
fissi prefissati `joy-` che non collidono con gli id `aN` degli attori. Budget indicativo: ≤ 3 KB
compattato.

**CLI (`@exporter`):** nuova opzione `--senza-player`, che lo omette. Predefinito: player incluso.
Nella libreria è il campo `Opzioni::player: bool` (predefinito `true`). Vale anche con `--watch`.
L'aiuto (`-h`) elenca la nuova opzione.

---

### B.6 Avvisi del compilatore (non bloccanti)
**Implementano:** `@compiler` (rilevazione, tipo `Avviso` accanto a `Errore` in `src/errore.rs`),
`@exporter` (stampa su stderr).

È il primo avviso del compilatore e ne fissa il formato per i successivi:

```text
Avviso alla riga N: <messaggio>
  Suggerimento: <suggerimento>
```

Il formato è lo stesso di `Errore` (A.1 e `Display` di `Errore`), con `Avviso` al posto di
`Errore`. La riga del suggerimento è facoltativa.

**Regole:**
* Gli avvisi **non** cambiano l'exit code: una compilazione riuscita con avvisi termina con 0 e
  scrive l'HTML.
* Sono stampati su **stderr**, in ordine di riga, prima del messaggio di successo. Se la
  compilazione fallisce si stampa solo l'errore. Con `--watch` sono ristampati a ogni
  ricompilazione riuscita.
* `ScenaCompilata` riceve il campo `avvisi: Vec<Avviso>`, così l'API di libreria li restituisce senza
  stamparli.
* Se due avvisi hanno la stessa riga e lo stesso messaggio, se ne stampa uno solo.

**Avviso «rotazione invisibile su un cerchio»** (un cerchio ha simmetria radiale: ruotarlo non
cambia nulla di ciò che si vede):
* azione `ruota di` su un attore di tipo `cerchio`, alla riga dell'azione:
  `Avviso alla riga N: "x" è un cerchio: la rotazione non cambia il suo aspetto`
  `  Suggerimento: togli «ruota di …», oppure usa un quadrato o un rettangolo se vuoi vederlo girare`
* proprietà `rotazione` diversa da 0 su un `cerchio` (B.1), alla riga di `Crea`: stesso messaggio, con
  il suggerimento `togli «rotazione …» dalla creazione del cerchio`.
* Non producono l'avviso: `ruota` su un gruppo che contiene cerchi, `orbita` su un cerchio (è una
  traslazione).

**Avviso «opacità invariata»** (*aggiunto in v0.2.1*): un'azione sul canale `opacita` che non
cambia il valore è quasi sempre una svista (es. un attore che dovrebbe comparire ma è già visibile).
* **Condizione:** per ogni segmento del canale `opacita` di un attore o di un gruppo, in ordine di
  inizio, si confrontano il valore **corrente** all'inizio del segmento e il valore di **arrivo**
  dell'azione. Se `|arrivo − corrente| < 1e-9`, l'azione produce l'avviso, alla riga dell'azione.
  * Il valore corrente del primo segmento è l'opacità iniziale del canale: `0` se il primo segmento
    è `appare sfumando` (A.3), altrimenti l'opacità dichiarata (default 1). Per i segmenti successivi
    è il valore di arrivo del segmento precedente sullo stesso canale: i segmenti non si
    sovrappongono (B.0.1) e tra due segmenti il valore resta fermo, anche se il successivo inizia
    nell'istante esatto in cui finisce il precedente.
  * Arrivo: `appare sfumando` → opacità dichiarata, o 1 se è 0 (A.3); `scompare sfumando` → 0;
    `cambia opacità a X` → X.
* **Testo** (`V` è il valore di arrivo, scritto come gli altri numeri dei messaggi: `1`, `0.6`, `0`;
  `"x"` è il nome dell'attore o del gruppo):
  * `appare sfumando`:
    `Avviso alla riga N: "x" appare sfumando ma la sua opacità è già V: l'azione non cambia nulla`
    `  Suggerimento: togli «appare sfumando», oppure fallo prima scomparire con «scompare sfumando»`
  * `scompare sfumando`:
    `Avviso alla riga N: "x" scompare sfumando ma la sua opacità è già 0: l'azione non cambia nulla`
    `  Suggerimento: togli «scompare sfumando», oppure fallo prima apparire con «appare sfumando»`
  * `cambia opacità a`:
    `Avviso alla riga N: "x" cambia opacità a V ma la sua opacità è già V: l'azione non cambia nulla`
    `  Suggerimento: togli «cambia opacità a …», oppure scegli un valore diverso da quello attuale`
* **Esempi che producono l'avviso:** due `appare sfumando` di seguito sullo stesso attore (sul
  secondo); `scompare sfumando` su un attore già scomparso o creato con `opacità 0`;
  `cambia opacità a 1` come prima azione di opacità di un attore con l'opacità predefinita.
* **Non producono l'avviso:**
  * `appare sfumando` come **prima** azione di opacità: parte da 0 e arriva a un valore > 0
    (anche con `opacità 0`, che con la regola di A.3 va da 0 a 1);
  * le azioni sugli altri canali dello stesso attore (`scivola`, `si ingrandisce`, …), anche nella
    stessa riga: si confronta solo il canale `opacita`;
  * le azioni di opacità di un membro rispetto a quella del gruppo e viceversa: il gruppo e i suoi
    membri hanno canali indipendenti (B.0.1), quindi l'avviso guarda solo l'opacità **propria**
    dell'attore o del gruppo, non quella risultante dalla moltiplicazione (B.3.5). Esempio: un
    membro che appare dentro un gruppo invisibile non produce l'avviso, e nemmeno un gruppo che
    appare con i membri già visibili.

---

### B.7 Decisioni confermate (2026-11-13)
Le questioni aperte di questa appendice e le decisioni segnalate dall'orchestratore sono state
**confermate dall'utente** il 2026-11-13. Da questa data sono specifica a tutti gli effetti, non più
proposte: cambiarle richiede una nuova approvazione.

1. **Rimozione della posizione `dentro a "[GRUPPO]"`** (B.3.1). *Confermato dall'utente.* L'appartenenza
   si dichiara solo nella frase del gruppo; usare `dentro a` è un errore.
2. **Nessun pivot esplicito dei gruppi nella v0.2** (es. `con centro a X, Y`). *Confermato
   dall'utente.* Il pivot è il centro dell'AABB degli ingombri iniziali (B.3.3). Un pivot esplicito
   richiederebbe nuova sintassi: se ne riparlerà per la v0.3.
3. **Verso predefinito di `orbita`: antiorario** sullo schermo (B.4.1). *Confermato dall'utente.*
4. **Centro dell'orbita statico** (B.4.1). *Confermato dall'utente.* L'orbita attorno a un attore
   che si muove si ottiene con i gruppi. Seguire un centro mobile (campionando la posizione di ALTRO)
   resta un possibile sviluppo della v0.3.
5. **Avviso «rotazione invisibile su un cerchio» esteso alla proprietà `rotazione` di `Crea`**
   (B.1, B.6), oltre al verbo `ruota di`. *Confermato dall'utente.*
6. **Stima prudente della dimensione dei testi** (B.0.4: 1.0 / 1.25 em di larghezza per carattere,
   1.5 em di altezza per riga). *Confermato dall'utente.* La sovrastima del 30-50% per i testi latini
   è accettata in cambio dell'uscita completa dal palco.
7. **L'esempio del §7 resta non valido** anche con la v0.2 (nota di B.8). *Confermato dall'utente.*
   Il §7 non viene modificato.

8. **`--fps` predefinito e intestazione `a [FPS] fotogrammi` (§2)** (nata da B.9). *Confermato
   dall'utente.* Il predefinito di `joypp render` segue il valore `a N fotogrammi` dell'intestazione
   del palco; se l'intestazione non lo dichiara, è 60. Il flag `--fps` lo sovrascrive sempre
   (precedenza in B.9.1).

Non restano questioni aperte in questa appendice.

---

### B.8 Esempio completo v0.2

```text
# Esempio completo v0.2: stato iniziale, gruppi, ingressi/uscite, orbita, colore
Prepara un palco grande 1280 per 720 con sfondo #0F172A.

Crea un cerchio chiamato "sole" con raggio 60 e colore #F59E0B al centro.
Crea un cerchio chiamato "terra" con raggio 18 e colore #38BDF8 a destra di "sole" di 200.
Crea un gruppo chiamato "sistema" con "sole" e "terra", scala 0.8 e livello 1.
Crea un titolo chiamato "titolo" con testo "SISTEMA ORBITALE", grandezza 36 e colore bianco in alto.
Crea un quadrato chiamato "stella" con lato 24, colore bianco, scala 0 e rotazione 45 in basso a destra.

Dal secondo 0 al secondo 1.5:
  "sistema" entra da sinistra con movimento scatto.
  "titolo" entra dall'alto con movimento morbido.

Dal secondo 1.5 al secondo 7.5:
  "terra" orbita attorno a "sole" con raggio 200 per 2 giri in senso orario con movimento lineare.
  "sole" pulsa leggermente a ritmo lentamente e cambia colore in #FDE68A.
  "stella" si ingrandisce fino a 1.5 e ruota di 180 gradi con movimento elastico.

Poi per 1.5 secondi:
  "sistema" esce verso destra con movimento morbido.
  "titolo" esce verso l'alto.
  "stella" scompare sfumando.
```

Risultati attesi (verificabili nei test di `@debugger`):
* Pivot di "sistema": AABB tra `[580, 858] × [300, 420]`, quindi `P = (719, 360)`. La scala 0.8 è
  applicata attorno a `P` fin da `t = 0`.
* "sistema" parte fuori campo a sinistra. Le estensioni di "terra" includono l'intero cerchio
  dell'orbita, quindi l'estensione destra del gruppo è `858 − 719 = 139`, che scalata di 0.8 dà
  `111.2`. Il pivot parte da `x = −113.2`.
* "terra" e "sole" sono fratelli nel gruppo, con distanza = raggio = 200: niente salto.
* "titolo" (stima `w = 576`, `h = 54`) parte con il centro a `y = −29`.
* Nessun avviso: nessun cerchio viene ruotato.

Nota per `@auditor`: l'esempio del §7 resta **non valido** anche con la v0.2. "sole" vi `pulsa` dal
secondo 0 all'8 e `si ingrandisce` dal 7 all'8: due azioni sullo stesso canale `scala` che si
sovrappongono (A.3).

---

### B.9 Esportazione video MP4 (`joypp render`)
**Implementano:** `@renderer` (funzione `__joy_seek` nell'HTML e sua integrazione con lo script di
adattamento e con il mini-player), `@exporter` (sottocomando, ricerca e pilotaggio del browser,
ffmpeg, progresso), `@security` (argomenti dei processi, file temporanei, porta di debug, limiti),
`@debugger` (test, B.9.9). `@compiler` non deve fare nulla: usa la durata già presente in
`ScenaCompilata`. Il riparto completo è in B.9.10.

**Panoramica.** `joypp render` compila il sorgente come la forma normale, scrive l'HTML in una
cartella temporanea e lo apre in un browser headless di sistema (Chrome, Chromium o Edge), alla
risoluzione esatta del palco. Per ogni fotogramma `k` chiama `__joy_seek(t_k)`, cattura uno
screenshot PNG e lo scrive nello stdin di `ffmpeg`, che produce un `.mp4` H.264 `yuv420p`. Il tempo
reale non conta: ogni fotogramma è posizionato al millisecondo esatto e catturato una sola volta,
quindi il video è fluido e senza fotogrammi persi anche se la cattura è lenta.

#### B.9.1 Sintassi (CLI)
```text
joypp render <file.joy> [-o <file.mp4>] [--fps N] [--crf N]
```

**Riconoscimento del sottocomando:**
* `render` è un sottocomando solo se è il **primo** argomento ed è esattamente la parola `render`
  (minuscola, senza estensione né percorso). In tutti gli altri casi la riga si interpreta come
  oggi: la forma `joypp <file.joy> [-o <file.html>] [--pretty] [--watch] [--senza-player]` resta
  invariata e compatibile.
* Un file che si chiama davvero `render` si indica con un percorso che non sia la parola nuda:
  `joypp ./render` (su Windows anche `joypp .\render`) lo compila in HTML, mentre
  `joypp render ./render` ne fa il video. `joypp render.joy` e `joypp --pretty render` trattano
  `render.joy` e `render` come file sorgente, perché la parola non è il primo argomento o non è nuda.
* `joypp render` senza file produce l'errore `manca il file sorgente .joy`.

**Opzioni ammesse con `render`:**

| Opzione | Valori ammessi | Predefinito |
|---------|----------------|-------------|
| `-o`, `--output <file>` | qualsiasi percorso | il sorgente con estensione `.mp4` (`video.joy` → `video.mp4`) |
| `--fps <N>` | intero da 1 a 240 | il valore `a N fotogrammi` dell'intestazione del palco (§2); se l'intestazione non lo dichiara, `60` |
| `--crf <N>` | intero da 0 a 51 (0 = senza perdita, 51 = qualità minima) | `18` |
| `-h`, `--help` / `-V`, `--version` | — | come nella forma normale |

* **Fotogrammi al secondo del video** (decisione confermata dall'utente, B.7), in ordine di
  precedenza:
  1. `--fps N`, se presente: **sovrascrive sempre** l'intestazione;
  2. altrimenti il valore `a N fotogrammi` dell'intestazione del palco (§2), cioè `ScenaCompilata::fps`;
  3. altrimenti `60`.

  L'intestazione ammette già solo interi da 1 a 240 (errore del parser «i fotogrammi al secondo
  devono essere un intero tra 1 e 240»), lo stesso intervallo di `--fps`: il valore che ne deriva è
  sempre valido e non richiede altri controlli. Esempio: con `Prepara un palco grande 640 per 360 a 30
  fotogrammi.`, `joypp render scena.joy` produce un video a 30 fps e
  `joypp render scena.joy --fps 60` uno a 60 fps.

* I valori di `--fps` e `--crf` sono interi decimali composti solo da cifre (`0`-`9`, zeri iniziali
  ammessi). Segno, punto decimale, spazi ed esponenti sono rifiutati (`--fps 29.97`, `--fps +30`,
  `--crf 1e1`). Il valore va nell'argomento successivo: la forma `--fps=60` non è ammessa, come per
  `-o`.
* Se un'opzione è ripetuta vale l'ultima, come oggi per `-o`.
* Le opzioni possono stare prima o dopo il file, purché dopo la parola `render`.
* **Non ammesse con `render`:** `--watch`, `--pretty`, `--senza-player`. Il video si genera una volta
  sola, e l'HTML intermedio è sempre compatto e senza player (B.9.4).
* **Non ammesse senza `render`:** `--fps` e `--crf`.
* Il video ha sempre le dimensioni del palco: non esiste un'opzione di risoluzione.
* L'aiuto (`-h`) aggiunge una sezione `joypp render` con le tre opzioni, i predefiniti (per `--fps`:
  «quello dell'intestazione, altrimenti 60») e le variabili d'ambiente `JOYPP_BROWSER` e
  `JOYPP_FFMPEG`.

#### B.9.2 `window.__joy_seek(ms)` nell'HTML (`@renderer`)
**Emissione:** sempre, in ogni HTML generato: anche con `--senza-player`, anche con `--pretty`,
anche con durata 0 e anche al di fuori di `joypp render`. È uno `<script>` **costante** emesso dopo
lo script di adattamento alla finestra. L'unico valore variabile è la durata `D` della scena in
millisecondi, formattata con `numero(durata × 1000)`.

**Firma:** `window.__joy_seek(ms: number): Promise<number>`. La funzione è definita con
`Object.defineProperty(window, "__joy_seek", { value: … })`, quindi non è né riscrivibile né
cancellabile.

**Semantica** (passi in ordine):
1. **Validazione (sincrona).** Se `typeof ms !== "number"` oppure `ms` non è finito (`NaN`,
   `±Infinity`), restituisce una Promise rifiutata con
   `TypeError("__joy_seek: ms deve essere un numero finito")`. Non ci sono conversioni implicite:
   `"100"`, `null` e `undefined` sono rifiutati.
2. **Limitazione (clamp).** `t = min(max(ms, 0), D)`. `-0` diventa `0`. Valori frazionari
   (`16.6667`) sono ammessi.
3. **Serializzazione.** Le chiamate sono messe in coda: ognuna inizia solo quando la precedente è
   terminata, anche se questa è stata rifiutata. Le Promise si risolvono quindi nell'ordine di
   chiamata.
4. **Modalità render** (alla prima chiamata, irreversibile fino al ricaricamento della pagina):
   * imposta l'attributo `data-joy-render` su `<html>` (`document.documentElement`);
   * imposta `#palco.style.transform = "none"`, cioè scala 1;
   * lo script di adattamento alla finestra (oggi `function f(){…}` in `dom.rs`) diventa:
     `if (document.documentElement.hasAttribute("data-joy-render")) return;` come prima istruzione.
     Così un `resize` successivo non riapplica la scala;
   * il mini-player (B.5), se presente, risulta **nascosto e fermo**. Il suo CSS contiene
     `html[data-joy-render] #joy-player{display:none!important}`. I suoi gestori di puntatore,
     tocco, focus e tastiera e il suo ciclo `requestAnimationFrame` non fanno nulla se l'attributo è
     presente, e il player non chiama mai `play()`.
5. **Font.** Attende `document.fonts.ready`, se `document.fonts` esiste.
6. **Posizionamento.** Se `Element.prototype.getAnimations` non esiste, rifiuta con
   `Error("__joy_seek: Web Animations API non disponibile")`. Altrimenti
   `A = palco.getAnimations({ subtree: true })`, ricalcolato a ogni chiamata e senza cache. Comprende
   le animazioni dei wrapper `.attore`, dei gruppi annidati e delle `.forma` (canale `colore`, B.4.2).
   Per ogni `a` di `A`: `a.pause()` e poi `a.currentTime = t`.
7. **Attesa dello stato.** `await Promise.all(A.map(a => a.ready))`.
8. **Attesa del disegno.** Doppio `requestAnimationFrame`: la prima callback precede il fotogramma
   che dipinge il nuovo stato, la seconda arriva dopo che quel fotogramma è stato prodotto.
9. **Risultato.** Risolve con `t`, il tempo effettivo dopo il clamp.

**Correttezza del tempo:** tutte le animazioni CSS della scena hanno durata `D`, ritardo 0 e
`linear both` (B.0, `dom.rs`). Quindi `currentTime = t` corrisponde esattamente all'istante `t`
della scena. In `t = D`, `fill: both` mantiene l'ultimo keyframe. Anche un'animazione con una durata
diversa resterebbe corretta, perché `currentTime` è assoluto e il riempimento è `both`.

**Implementazione di riferimento** (indicativa; `D` = durata in ms):
```js
(function(){var D=8500,p=document.getElementById("palco"),q=Promise.resolve();
function passo(t){var h=document.documentElement;
 if(!h.hasAttribute("data-joy-render")){h.setAttribute("data-joy-render","");p.style.transform="none"}
 return(document.fonts?document.fonts.ready:Promise.resolve()).then(function(){
  if(!p.getAnimations)throw new Error("__joy_seek: Web Animations API non disponibile");
  var a=p.getAnimations({subtree:true});
  a.forEach(function(x){x.pause();x.currentTime=t});
  return Promise.all(a.map(function(x){return x.ready}))
 }).then(function(){return new Promise(function(r){
  requestAnimationFrame(function(){requestAnimationFrame(function(){r(t)})})})})}
Object.defineProperty(window,"__joy_seek",{value:function(ms){
 if(typeof ms!=="number"||!isFinite(ms))
  return Promise.reject(new TypeError("__joy_seek: ms deve essere un numero finito"));
 var t=Math.min(Math.max(ms,0),D)||0,r=q.then(function(){return passo(t)});
 q=r.catch(function(){});return r}})})();
```

**Sicurezza:** lo script è un testo costante del compilatore. Non contiene stringhe dell'utente e
non legge `data-nome` né il contenuto dei testi. Non usa `eval`, `Function` o `innerHTML`.
`ms` è validato come numero finito prima di qualsiasi uso. Budget indicativo: ≤ 1 KB compattato.

#### B.9.3 Campionamento dei fotogrammi
* `D` è la durata della scena in millisecondi, **lo stesso valore** emesso nell'HTML
  (`numero(durata × 1000)`, arrotondato a 4 decimali).
* `fps` è il valore effettivo scelto con la precedenza di B.9.1: `--fps`, poi l'intestazione
  `a N fotogrammi`, poi 60.
* **Numero di fotogrammi:** `N = ⌈D × fps / 1000 − 10⁻⁹⌉ + 1`. Il termine `10⁻⁹` evita che un errore
  di virgola mobile aggiunga un fotogramma quando `D × fps / 1000` è intero.
* **Tempo del fotogramma `k`** (`k = 0 … N−1`): `t_k = min(k × 1000 / fps, D)` ms, calcolato a
  partire da `k` (mai per somme successive, che accumulerebbero errore).
* Il fotogramma 0 è lo stato iniziale. L'ultimo è **esattamente** lo stato finale: se
  `D × fps / 1000` non è intero, il suo tempo viene limitato a `D`. Il video dura `N / fps` secondi.
* Gli istanti `t_k` sono passati al browser come numeri (B.9.5) e `__joy_seek` li limita di nuovo a
  `[0, D]`. Le due limitazioni coincidono perché usano lo stesso `D`.

| `D` (ms) | `fps` | `N` | Ultimi istanti |
|----------|-------|-----|----------------|
| 8500 | 60 | 511 | `t_509 = 8483.33`, `t_510 = 8500` |
| 1000 | 30 | 31 | `t_30 = 1000` |
| 1010 | 60 | 62 | `t_60 = 1000`, `t_61 = min(1016.67, 1010) = 1010` |
| 300 | 1 | 2 | `t_0 = 0`, `t_1 = min(1000, 300) = 300` |

* **Scena senza timeline (`D = 0`):** è un **errore** (B.9.7). Un video di un solo fotogramma
  (lungo `1/fps` secondi) non è un risultato utile, e il caso segnala quasi sempre un blocco
  temporale dimenticato.

#### B.9.4 HTML intermedio e file temporanei (`@exporter`)
* **Compilazione:** come `compila_file`, con gli stessi controlli (dimensione massima, UTF-8, output
  uguale al sorgente) e con la stampa degli avvisi di B.6. Le opzioni sono fisse:
  `Opzioni { leggibile: false, player: false }`. Il player non viene emesso, mentre `__joy_seek` c'è
  sempre (B.9.2).
* **Dimensioni:** larghezza `W` e altezza `H` del palco devono essere intere
  (`|v − round(v)| < 10⁻⁹`), altrimenti è un errore (B.9.7). Con dimensioni frazionarie il palco,
  centrato dal flex, cadrebbe a metà pixel.
* **Cartella temporanea:** `std::env::temp_dir()/joypp-render-<pid>-<n>`, creata con
  `fs::create_dir` in modo esclusivo. Se esiste già si prova con `n + 1`, fino a 100 tentativi. Su
  Unix ha permessi `0o700` (`DirBuilderExt::mode`, API sicura). Contiene solo nomi fissi, mai
  derivati dall'input dell'utente:
  * `scena.html`: l'HTML intermedio;
  * `profilo/`: la cartella del profilo del browser (`--user-data-dir`), vuota e usa e getta.
* **URL:** `file://` seguito dal percorso assoluto di `scena.html`. Su Windows si toglie il prefisso
  `\\?\` prodotto da `canonicalize`, `\` diventa `/` e si antepone `/` alla lettera dell'unità
  (`file:///C:/Users/…/scena.html`). Ogni byte UTF-8 fuori da `A-Z a-z 0-9 - . _ ~ /` (e dal `:`
  dell'unità) è codificato come `%XX`. Un percorso UNC (`\\server\…`) è un errore (B.9.7).
* **Rimozione:** una guardia (`Drop`) rimuove ricorsivamente la cartella alla fine, sia in caso di
  successo sia in caso di errore, **dopo** la chiusura del browser (su Windows i file aperti non si
  possono cancellare). Se la rimozione fallisce stampa l'avviso di B.9.7.
* **Limite noto: terminazione forzata.** La pulizia (cartella temporanea, file parziale, chiusura di
  browser e ffmpeg) è garantita **solo** per le uscite ordinarie, gli errori e i `panic!`, tramite le
  guardie `Drop`. Se il processo `joypp` viene terminato in modo forzato (`taskkill /F`, `SIGKILL`,
  Ctrl+C o `SIGTERM` senza gestore) i `Drop` non vengono eseguiti e possono restare la cartella
  `joypp-render-*`, il file `.joypp-parziale` e i processi del browser e di ffmpeg orfani. Il crate
  non può installare un gestore di segnali: servirebbe codice `unsafe` (vietato da
  `#![forbid(unsafe_code)]`) o una dipendenza (esclusa da D-B9-1). **Mitigazione per l'utente:**
  eliminare a mano le cartelle `joypp-render-*` nella cartella temporanea di sistema (`%TEMP%` su
  Windows, `$TMPDIR` o `/tmp` su Unix), l'eventuale `<uscita>.joypp-parziale`, e chiudere gli
  eventuali processi rimasti. *Possibile miglioramento futuro (non normativo):* all'avvio di un
  render, rimuovere automaticamente le cartelle `joypp-render-<pid>-*` il cui `pid` non corrisponde
  più a un processo attivo.
* **Output parziale:** ffmpeg scrive in `<cartella di uscita>/<nome di uscita>.joypp-parziale`. Solo
  a fine render riuscito il file viene rinominato in `<uscita>` (`fs::rename`, che sostituisce un
  file esistente). In caso di errore il parziale viene rimosso e un vecchio file `<uscita>` resta
  intatto. La cartella di uscita viene creata se manca, come in `compila_file`.

#### B.9.5 Pilotaggio del browser (`@exporter`)
**Ricerca dell'eseguibile** (vince il primo trovato):
1. Variabile d'ambiente `JOYPP_BROWSER`, se definita e non vuota. Deve indicare un file esistente,
   altrimenti è un errore, senza ripiego sui passi successivi. Il valore è un **percorso**, non una
   riga di comando: non viene diviso e non può aggiungere argomenti.
2. Percorsi standard del sistema operativo, nell'ordine:
   * **Windows** (solo se la variabile è definita):
     `%ProgramFiles%\Google\Chrome\Application\chrome.exe`,
     `%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe`,
     `%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe`,
     `%LOCALAPPDATA%\Chromium\Application\chrome.exe`,
     `%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe`,
     `%ProgramFiles%\Microsoft\Edge\Application\msedge.exe`;
   * **macOS:** `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`,
     `/Applications/Chromium.app/Contents/MacOS/Chromium`,
     `/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge`, poi gli stessi tre sotto
     `$HOME/Applications/`;
   * **Linux e altri Unix:** nessun percorso fisso (si usa il `PATH`).
3. `PATH`, scorso con `std::env::split_paths` (mai con `which`/`where` né con una shell), cercando
   nell'ordine `google-chrome-stable`, `google-chrome`, `chromium`, `chromium-browser`,
   `microsoft-edge-stable`, `microsoft-edge`, `chrome`, `msedge`. Su Windows ogni nome riceve il
   suffisso `.exe`.

**Avvio:** `Command` con argomenti separati, stdin e stdout su null, stderr in pipe. Un thread legge
continuamente lo stderr e ne conserva le ultime 50 righe, che servono per i messaggi d'errore. Gli
argomenti sono esattamente questi:
```text
--headless=new --remote-debugging-port=0 --user-data-dir=<cartella temporanea>/profilo
--no-first-run --no-default-browser-check --disable-extensions --disable-sync
--disable-background-networking --disable-component-update --disable-default-apps
--disable-background-timer-throttling --disable-backgrounding-occluded-windows
--disable-renderer-backgrounding --mute-audio --hide-scrollbars
--force-device-scale-factor=1 --force-color-profile=srgb --window-size=W',H' about:blank
```
**Finestra e viewport (normativo):** `W' = max(W, 500)` e `H' = max(H, 500)` (costante
`LATO_MINIMO_FINESTRA = 500`). La finestra può quindi essere più grande del palco, ma la viewport è
**sempre esattamente** `W × H`: la impone `Emulation.setDeviceMetricsOverride` con
`deviceScaleFactor: 1` (passo 6). Ogni PNG catturato è verificato tramite l'`IHDR` (passo 10): se
larghezza e altezza non sono esattamente `W × H` è un errore (B.9.7), mai un ritaglio o un
ridimensionamento. Motivo: con una finestra più piccola (es. il palco 64×36 di B.9.9) Edge headless
smette di rispondere a `Page.captureScreenshot` dopo 5-6 fotogrammi (timeout).

`--no-sandbox` **non** viene mai aggiunto (B.9.8). Il profilo dedicato serve anche perché le versioni
recenti di Chrome ignorano `--remote-debugging-port` con il profilo predefinito.

**Decisione D-B9-1: client CDP minimale con la sola libreria standard, via WebSocket su
`--remote-debugging-port=0`. Nessuna nuova dipendenza.** Il `Cargo.toml` resta senza dipendenze e il
crate resta `#![forbid(unsafe_code)]`. Alternative valutate:

| Alternativa | Esito | Motivo |
|-------------|-------|--------|
| `--remote-debugging-pipe` (messaggi JSON separati da `\0`) | scartata | Il browser legge e scrive sui descrittori 3 e 4 (POSIX) o su handle ereditati (Windows). Con la libreria standard di Rust non si possono passare al figlio senza `unsafe` (`pre_exec` + `dup2`, `SetHandleInformation`), che il crate vieta. |
| Dipendenze (`tungstenite` + `serde_json`, oppure `headless_chrome` / `chromiumoxide`) | scartata | Decine di crate transitivi (`chromiumoxide` porta anche `tokio`), più superficie per la catena di fornitura e la fine della regola «nessuna dipendenza esterna», per una funzione accessoria. |
| `--screenshot` con un processo per fotogramma | scartata | Nessun protocollo, ma 0.5-1 s per fotogramma e nessun modo affidabile di attendere `__joy_seek` prima della cattura. |
| `Page.startScreencast` | scartata | Cattura in tempo reale: i fotogrammi non sono esatti e alcuni vanno persi. |

Il client minimale comprende (stima ≤ 800 righe, tutte con test unitari):
* **WebSocket (RFC 6455, lato client):** handshake HTTP/1.1 e framing di testo con maschera
  obbligatoria. Gestisce le lunghezze a 7, 16 e 64 bit, la frammentazione (frame di continuazione),
  ping/pong e close. Rifiuta i frame binari. La chiave `Sec-WebSocket-Key` e le maschere sono
  pseudo-casuali (da `RandomState`, tempo e pid): l'entropia non conta su loopback. Si verifica lo
  stato `101`, mentre `Sec-WebSocket-Accept` **non** viene verificato: richiederebbe SHA-1 e il
  server è il processo figlio appena avviato. Non si invia l'intestazione `Origin`.
* **JSON:** uno scrittore per i comandi, che contengono solo nomi costanti, numeri e identificatori
  restituiti dal browser (sempre con escape). Un parser completo ma piccolo per le risposte
  (oggetti, array, stringhe con escape e surrogati `\uXXXX`, numeri, `true`/`false`/`null`), con
  profondità massima 64 e solo errori, mai panic.
* **Base64:** solo la decodifica (RFC 4648), per i dati degli screenshot.
* Moduli indicativi: `src/exporter/render/{mod,browser,cdp,websocket,json,base64,ffmpeg}.rs`.

**Connessione e sessione:**
1. Si attende il file `profilo/DevToolsActivePort` (controllo ogni 50 ms, timeout 30 s). La riga 1
   contiene la porta (intero 1-65535), la riga 2 il percorso (deve iniziare con `/devtools/browser/`).
   Se il processo termina prima, è l'errore «il browser si è chiuso».
2. `TcpStream` verso `127.0.0.1:<porta>` (mai `localhost`, per evitare DNS e IPv6), con timeout di
   lettura e scrittura di 30 s, poi l'handshake WebSocket sul percorso letto.
3. I comandi sono `{"id":n,"method":…,"params":{…}}`, con `"sessionId"` dopo il passo 5 e `id`
   crescenti da 1. Gli eventi (messaggi senza `id`) che arrivano mentre si attende una risposta vanno
   in una coda. Una risposta con `error` diventa un errore (B.9.7).
4. `Target.createTarget {"url":"about:blank"}` → `targetId`.
5. `Target.attachToTarget {"targetId":…,"flatten":true}` → `sessionId`.
6. `Emulation.setDeviceMetricsOverride {"width":W,"height":H,"deviceScaleFactor":1,"mobile":false}`:
   viewport identica al palco, indipendentemente dalla finestra `W' × H'` di avvio.
7. `Emulation.setDefaultBackgroundColorOverride {"color":{"r":0,"g":0,"b":0,"a":1}}`: sfondo
   opaco. Il `body` è già `#000` e il palco copre tutta la viewport. Uno `sfondo` semitrasparente
   risulta quindi composto su nero, come nella pagina aperta normalmente.
8. `Page.enable`, poi `Page.navigate {"url":<URL file://>}`. Se la risposta contiene `errorText` è un
   errore. Poi si attende l'evento `Page.loadEventFired` (timeout 30 s).
9. `Runtime.evaluate {"expression":"typeof window.__joy_seek","returnByValue":true}`. Il risultato
   deve essere `"function"`, altrimenti è un errore interno.
10. Per ogni `k = 0 … N−1`:
    * `Runtime.evaluate {"expression":"window.__joy_seek(T)","awaitPromise":true,"returnByValue":true}`,
      dove `T` è `t_k` (finito per costruzione) formattato da Rust con `{}`. Il risultato è il
      letterale numerico più corto e senza esponente, valido in JS. Se ci sono `exceptionDetails`,
      o se il valore restituito differisce da `t_k` di più di `10⁻⁶`, è un errore;
    * `Page.captureScreenshot {"format":"png","fromSurface":true,"captureBeyondViewport":false}`.
      Il campo `data` viene decodificato da base64. Si verifica la firma PNG e che l'intestazione
      `IHDR` dica `W × H`, altrimenti è un errore (ad esempio un ridimensionamento DPI imprevisto);
    * i byte PNG vengono scritti nello stdin di ffmpeg (B.9.6).
11. **Chiusura:** `Browser.close`, poi attesa dell'uscita del processo per al massimo 5 s, poi
    `kill()` e `wait()`. Una guardia `Drop` garantisce la chiusura anche sui percorsi d'errore.

**Pipeline:** il ciclo può essere sequenziale. È ammesso un thread di scrittura verso ffmpeg con un
canale limitato a 4 fotogrammi, così cattura e codifica si sovrappongono. L'ordine dei fotogrammi
deve restare quello di `k` e la memoria resta limitata.

**Determinismo:** a parità di sorgente, browser e ffmpeg, i fotogrammi sono identici. Il file `.mp4`
non è garantito identico byte per byte tra versioni diverse di browser o ffmpeg.

#### B.9.6 ffmpeg (`@exporter`)
**Ricerca:** variabile `JOYPP_FFMPEG` (stesse regole di `JOYPP_BROWSER`), poi `ffmpeg` nel `PATH`
(`ffmpeg.exe` su Windows). Si verifica che funzioni lanciando `ffmpeg -hide_banner -version`, che
deve terminare con codice 0.

**Riga di comando** (ogni elemento è un argomento separato, mai una shell):
```text
<ffmpeg> -hide_banner -loglevel error -nostats -y
  -f image2pipe -c:v png -framerate <FPS> -i -
  -vf <FILTRO>
  -an -c:v libx264 -preset medium -crf <CRF> -pix_fmt yuv420p
  -color_range tv -colorspace bt709 -color_primaries bt709 -color_trc bt709
  -movflags +faststart -f mp4 file:<percorso assoluto del file parziale>
```
* `<FILTRO>` con `W` e `H` pari: `scale=out_color_matrix=bt709:out_range=tv,format=yuv420p`. Senza
  questa conversione esplicita ffmpeg passerebbe da RGB a YUV con la matrice BT.601, mentre i lettori
  assumono BT.709 per i video HD: i colori risulterebbero leggermente sbagliati. `format=yuv420p`
  nel filtro garantisce che la conversione la faccia proprio questo `scale`. `-pix_fmt yuv420p`
  resta per chiarezza.
* **Dimensioni dispari** (yuv420p richiede larghezza e altezza pari). **Decisione: si aggiunge un
  bordo (`pad`), non è un errore.**
  `<FILTRO> = pad=W2:H2:0:0:color=0xRRGGBB,scale=out_color_matrix=bt709:out_range=tv,format=yuv420p`,
  con `W2 = W + (W mod 2)` e `H2 = H + (H mod 2)`. La colonna e/o la riga aggiunte stanno **a destra e
  in basso**. Centrare il palco sposterebbe tutto di mezzo pixel. Il colore è lo `sfondo` del palco
  composto su nero (`canale × alfa`, arrotondato), riemesso dai valori numerici e mai dal testo
  utente. Viene stampato l'avviso di B.9.7.
* `file:` + percorso assoluto: un nome che inizia con `-` non può diventare un'opzione e un nome come
  `pipe:1` o `http://…` non può diventare un protocollo.
* **Sovrascrittura:** `-y` vale solo per il file parziale, che è di Joy++. L'`<uscita>` viene
  sostituita in silenzio dalla rinomina finale, come oggi l'HTML (B.9.4).
* **Ordine di avvio:** ffmpeg parte dopo il passo 9 di B.9.5, quindi a pagina pronta. Se il browser
  fallisce prima, non si crea nessun file parziale.
* **stderr di ffmpeg:** un thread lo legge continuamente (per evitare il blocco della pipe) e ne
  conserva gli ultimi 64 KiB.
* **Fine e codice d'uscita:** dopo l'ultimo fotogramma si chiude lo stdin e si attende ffmpeg (senza
  timeout: la codifica finale può essere lunga). Un codice diverso da 0 è un errore e riporta fino a
  20 ultime righe dello stderr. Se la scrittura nello stdin fallisce (ad esempio `BrokenPipe`), si
  attende ffmpeg e si riporta il **suo** stderr, che contiene la causa, invece dell'errore di I/O.
* **Nessun fotogramma perso:** si scrivono esattamente `N` PNG, ognuno una sola volta e in ordine.
  `image2pipe` con `-framerate FPS` assegna loro i tempi `k / FPS`, quindi il video ha frequenza
  costante e `N` fotogrammi (verificato in B.9.9).

**Progresso e messaggi** (`@exporter`):
* Gli avvisi di B.6 vanno su stderr prima del render, come nella compilazione normale.
* Il progresso va su **stderr**. Se è un terminale (`std::io::IsTerminal`), una riga
  `Render: fotogramma k/N (P%)` viene riscritta con `\r` al massimo ogni 100 ms e alla fine, poi va
  a capo. Altrimenti si stampa una riga a ogni 10%.
* A fine render, su **stdout**:
  `✔ <sorgente> → <uscita> (N fotogrammi, W×H, FPS fps, S s, B byte)`, dove `W×H` sono le dimensioni
  finali del video e `S` è `N / FPS` con due decimali.

#### B.9.7 Errori, avvisi ed exit code
**Exit code:** `0` per un render riuscito (anche con avvisi), `2` per un uso errato della CLI
(messaggio seguito dall'aiuto, come oggi), `1` per tutti gli altri errori. Un errore stampa una sola
voce nel formato di `Errore` (`Errore: …` più l'eventuale `Suggerimento: …`).

**Errori di uso (exit 2):**
* `il valore di --fps deve essere un intero tra 1 e 240 (trovato «X»)`.
* `il valore di --crf deve essere un intero tra 0 e 51 (trovato «X»)` (suggerimento: «18 è quasi
  indistinguibile dall'originale; valori più alti riducono il file»).
* `l'opzione --fps richiede un valore` (lo stesso per `--crf` e `-o`).
* `l'opzione «--watch» non è ammessa con «joypp render»` (suggerimento: «il video si genera una volta
  sola: rilancia «joypp render» dopo le modifiche»). Lo stesso vale per `--pretty` e
  `--senza-player`, con il suggerimento «il video non dipende da questa opzione: toglila».
* `l'opzione «--fps» vale solo con «joypp render»` (anche `--crf`; suggerimento:
  «es. «joypp render scena.joy --fps 30»»).
* `manca il file sorgente .joy`, `opzione sconosciuta «X»`, `è ammesso un solo file sorgente`: come
  oggi.

**Errori di render (exit 1)**, in ordine di controllo. Si controlla prima ciò che non avvia processi:
* errori di lettura e di compilazione del sorgente: come oggi;
* output uguale al sorgente: il messaggio di `compila_file`, con il suggerimento
  «scegli un percorso diverso con -o, es. -o video.mp4»;
* `la scena non ha una timeline: il video sarebbe un solo fotogramma` (suggerimento: «aggiungi
  almeno un blocco temporale, es. «Dal secondo 0 al secondo 3:»»);
* `per il video le dimensioni del palco devono essere intere (trovato 800.5 × 600)`;
* `il palco è troppo grande per il video (massimo 8192 pixel per lato e 35389440 pixel in tutto)`;
* `il video avrebbe troppi fotogrammi: N (massimo 216001)` (suggerimento: «riduci --fps o la durata
  della scena»). Il controllo usa il `fps` effettivo (B.9.1), anche quando viene dall'intestazione;
* `ffmpeg non trovato` (suggerimento: «installalo da https://ffmpeg.org e aggiungilo al PATH, oppure
  indica il percorso con la variabile d'ambiente JOYPP_FFMPEG»);
* `JOYPP_FFMPEG indica «X», che non esiste` e `impossibile avviare ffmpeg («X»): <errore di sistema>`;
* ``ffmpeg («X») non funziona: `-version` è terminato con un errore``: verifica preliminare di B.9.6
  (`ffmpeg -hide_banner -version` con codice ≠ 0), eseguita subito dopo la ricerca di ffmpeg e
  quindi prima della ricerca del browser e della creazione della cartella temporanea;
* `nessun browser compatibile trovato (Chrome, Chromium o Edge)` (suggerimento: «installa Google
  Chrome o Microsoft Edge, oppure indica il percorso con la variabile d'ambiente JOYPP_BROWSER»);
* `JOYPP_BROWSER indica «X», che non esiste` e `impossibile avviare il browser («X»): <errore di sistema>`;
* `la cartella temporanea «X» non è utilizzabile: <motivo>` (anche per un percorso UNC; suggerimento:
  «imposta TEMP/TMPDIR su una cartella locale»);
* `il browser non ha aperto la porta di debug entro 30 secondi` (suggerimento: «verifica che «X» sia
  Chrome, Chromium o Edge in una versione recente»);
* `il browser ha scritto un file DevToolsActivePort non valido: <motivo>`, con `<motivo>`
  `porta non valida «X»` (riga 1 non intera tra 1 e 65535) o `percorso non valido «X»` (riga 2 che
  non inizia con `/devtools/browser/`), letto quando il file è completo e stabile (B.9.5, passo 1);
* `il browser si è chiuso inaspettatamente (codice C)` (o `(terminato da un segnale)`);
* `il browser non ha risposto entro 30 secondi (comando «M», fotogramma k/N)`;
* `il browser ha segnalato un errore: <messaggio>` (per `error` CDP e `errorText` della navigazione);
* `errore interno: la pagina generata non espone __joy_seek` (passo 9 di B.9.5),
  `errore interno: __joy_seek ha fallito: <motivo>` (la valutazione restituisce `exceptionDetails`)
  ed `errore interno: __joy_seek ha restituito un istante diverso da quello richiesto` (valore non
  numerico o lontano da `t_k` più di `10⁻⁶`, passo 10). Sono bug: il suggerimento è «è un bug di
  Joy++: segnalalo allegando il file .joy»;
* `lo screenshot è W'×H' invece di W×H` (suggerimento: «il browser ridimensiona la pagina: segnala il
  problema indicando browser e sistema operativo»);
* `messaggio del browser troppo grande (massimo 256 MiB)` e `risposta del browser non valida: <motivo>`;
* `ffmpeg è terminato con un errore (codice C)` seguito da `  ffmpeg: <riga>` per le ultime righe
  (al massimo 20). Se lo stderr nomina `libx264`, il suggerimento è «installa una versione di ffmpeg
  che includa libx264 (es. una build GPL)»;
* `impossibile scrivere i fotogrammi a ffmpeg: <errore di I/O>` seguito dalle righe `  ffmpeg: …`:
  la scrittura nello stdin è fallita ma ffmpeg è terminato con codice 0. Se il codice è ≠ 0 prevale
  il messaggio precedente (B.9.6). Stesso suggerimento su `libx264`;
* `ffmpeg ha chiuso la pipe prima della fine`: ffmpeg ha smesso di accettare fotogrammi prima
  dell'`N`-esimo, ma è terminato con codice 0 e senza errori di scrittura. È una salvaguardia: un
  video con meno di `N` fotogrammi non viene mai rinominato in `<uscita>`;
* `impossibile creare la cartella «X»: <errore>`: creazione della cartella di uscita
  (`fs::create_dir_all`), con lo stesso testo di `compila_file`;
* `impossibile scrivere «X»: <errore>`: tutte le altre operazioni su file e percorsi, cioè
  `scena.html` e `profilo/` nella cartella temporanea, la risoluzione del percorso assoluto di
  scena e uscita, la rinomina finale del parziale e la lettura della dimensione del file prodotto.
  Criterio: «creare la cartella» vale solo per la cartella di uscita, «scrivere» per tutto il resto.

Tutti gli errori di questo elenco hanno **exit code 1**.

Per i messaggi che includono lo stderr del browser o di ffmpeg si riportano al massimo le ultime 20
righe, ciascuna preceduta da `  browser: ` o `  ffmpeg: `. **Su ogni errore**: si chiudono browser e
ffmpeg (kill + wait), si rimuovono il file parziale e la cartella temporanea, e l'eventuale `<uscita>`
preesistente resta intatta.

**Avvisi (non bloccanti, stderr, formato `Attenzione: …` come quello attuale sull'estensione):**
* `«X» non ha estensione .joy` (esistente);
* `«X» non ha estensione .mp4: il file sarà comunque un video MP4`;
* `il palco W×H ha dimensioni dispari: il video sarà W2×H2 (aggiunto un bordo a destra/in basso
  del colore dello sfondo)`;
* `impossibile rimuovere la cartella temporanea «X»: <errore>`.

**Limiti anti-DoS** (nuove costanti in `security::limiti`):

| Costante | Valore | Motivo |
|----------|--------|--------|
| `MAX_FPS_VIDEO` | 240 | intervallo di `--fps` |
| `MAX_CRF` | 51 | intervallo di `--crf` (limite di libx264) |
| `MAX_FOTOGRAMMI_VIDEO` | 216 001 | un'ora a 60 fps, ultimo fotogramma incluso: `MAX_DURATA_SCENA` (3600 s) resta esportabile a 60 fps, il predefinito quando l'intestazione non dichiara `a N fotogrammi` |
| `MAX_LATO_VIDEO` | 8192 | dimensioni finali, dopo il `pad` |
| `MAX_PIXEL_VIDEO` | 35 389 440 (8192 × 4320) | dimensione massima di un fotogramma al livello H.264 6.2 |
| `MAX_MESSAGGIO_CDP` | 256 MiB | messaggio WebSocket riassemblato (lo screenshot più grande in base64) |
| `TIMEOUT_AVVIO_BROWSER` | 30 s | attesa di `DevToolsActivePort` |
| `TIMEOUT_COMANDO_CDP` | 30 s | risposta a un comando o evento atteso, compreso `__joy_seek` |
| `TIMEOUT_CHIUSURA` | 5 s | uscita del browser dopo `Browser.close` |

#### B.9.8 Sicurezza (`@security`)
* **Processi:** solo `std::process::Command` con argomenti separati, mai `sh -c`, `cmd /C` o simili.
  Gli eseguibili (da variabile d'ambiente, percorso standard o `PATH`) non vengono mai divisi in
  argomenti. Il percorso di output arriva a ffmpeg come `file:<assoluto>` (B.9.6).
* **Script iniettati:** le espressioni inviate con `Runtime.evaluate` sono testi costanti più un
  numero finito formattato da Rust. Nessuna stringa dell'utente (nomi, testi, percorsi) entra mai in
  un'espressione JS o in un comando CDP. L'URL `file://` è codificato byte per byte (B.9.4) e
  passato come stringa JSON con escape.
* **Porta di debug:** solo loopback, scelta a caso (`=0`), con un profilo dedicato (nessun cookie
  né password dell'utente) e aperta solo per la durata del render. Non si usa
  `--remote-allow-origins`, quindi una pagina web non può collegarsi. Rischio residuo accettato: un
  altro processo locale dello stesso utente potrebbe collegarsi durante il render, ma avrebbe già gli
  stessi privilegi. `--no-sandbox` non viene mai aggiunto. Chi ne ha bisogno (container, root) può
  puntare `JOYPP_BROWSER` a un proprio script, assumendosene la responsabilità.
* **File temporanei:** creazione esclusiva, permessi `0o700` su Unix, nomi interni fissi, rimozione
  garantita dalla guardia (B.9.4).
* **Input dal browser:** messaggi limitati a `MAX_MESSAGGIO_CDP`, JSON con profondità ≤ 64, base64 e
  PNG validati. Ogni anomalia è un `Errore`, mai un `panic!`. Restano valide le regole del crate:
  niente `unsafe`, `unwrap`, `expect` o `panic` sui dati esterni.
* **Terminale:** le righe di stderr di browser e ffmpeg riportate all'utente sono ripulite dai
  caratteri di controllo (tranne la tabulazione, sostituiti con `?`), per evitare sequenze di escape
  nel terminale.
* **Processi figli:** chiusi e attesi (`kill` + `wait`) su ogni percorso, per non lasciare processi
  orfani o zombie.

#### B.9.9 Test (`@debugger`)
**Senza ffmpeg né browser** (sempre eseguiti da `cargo test`):
* CLI: `render` come primo argomento, `./render` come file, `joypp --pretty render`, predefiniti di
  `-o`/`--fps`/`--crf`, limiti (`0`, `1`, `240`, `241`, `-1`, `29.97`, `+30`, `abc`), opzione
  ripetuta, opzioni vietate con e senza `render`, exit code 2.
* Precedenza di `fps` (B.9.1, funzione pura): intestazione senza `a N fotogrammi` e senza `--fps` →
  60; intestazione `a 30 fotogrammi` senza `--fps` → 30; intestazione `a 30 fotogrammi` con
  `--fps 60` → 60.
* Campionamento: la tabella di B.9.3, più la proprietà `t_{N−1} = D` e `t_k` strettamente crescente.
* HTML: `__joy_seek` presente con e senza player, con `--pretty` e con durata 0. La costante `D` è
  corretta. Con un nome e un testo malevoli (`</script><script>alert(1)`) lo script di seek resta
  identico al riferimento. Lo script di adattamento contiene il controllo su `data-joy-render`.
* Costruzione degli argomenti di ffmpeg (funzione pura che restituisce `Vec<OsString>`): 1280×720 a
  60 fps con crf 18 (vettore esatto); 1281×721 con `pad=1282:722:0:0:color=0x0f172a` per lo sfondo
  `#0F172A`; sfondo `rgba(255,0,0,0.5)` → `0x800000`.
* URL `file://`: `C:\Users\Mario Rossi\AppData\Local\Temp\joypp-render-1-0\scena.html` →
  `file:///C:/Users/Mario%20Rossi/AppData/Local/Temp/joypp-render-1-0/scena.html`, il prefisso
  `\\?\`, i caratteri non ASCII e il rifiuto dei percorsi UNC.
* Ricerca degli eseguibili: funzione pura che riceve le variabili d'ambiente e un predicato di
  esistenza dei file (precedenza `JOYPP_*`, errore senza ripiego, ordine dei candidati per sistema).
* WebSocket (codifica con maschera; decodifica di lunghezze a 7/16/64 bit, continuazioni, ping,
  close), base64 (vettori della RFC 4648), parser JSON (escape, surrogati, profondità, input
  malformati → `Err` senza panic), lettura dell'`IHDR` PNG.
* Errori prima dell'avvio dei processi (durata 0, palco non intero, palco troppo grande, troppi
  fotogrammi) tramite l'API di libreria.

**Integrazione** (`tests/render.rs`): se browser o ffmpeg non vengono trovati, il test stampa
`test saltato: <motivo>` e termina con successo. Con `JOYPP_TEST_RENDER=1` la loro assenza è invece
un fallimento (per la CI). Casi:
* palco 64×36, durata 0.5 s, `--fps 10` → `N = 6`. Il `.mp4` esiste e inizia con un box `ftyp`. Se
  `ffprobe` è disponibile si controllano 6 fotogrammi, 64×36, `h264` e `yuv420p`;
* palco 63×35 → video 64×36, con l'avviso stampato;
* precisione di `__joy_seek` via CDP: dopo `__joy_seek(250)`, `getComputedStyle` di un attore
  restituisce la traslazione precalcolata a `t = 0.25 s` (tolleranza 0.01 px);
* nessun file `joypp-render-*` resta nella cartella temporanea e nessun `.joypp-parziale` resta
  accanto all'output, sia dopo un successo sia dopo un errore provocato. Gli errori di ffmpeg sono
  due casi distinti (B.9.7), exit code 1 in entrambi e un eventuale `<uscita>` preesistente intatto:
  * **ffmpeg non eseguibile o non valido:** `JOYPP_FFMPEG` punta a un eseguibile che termina con
    codice ≠ 0 già con `-hide_banner -version`. Lo intercetta la verifica preliminare di B.9.6 con
    ``ffmpeg («X») non funziona: `-version` è terminato con un errore``. Il browser non viene
    avviato e non si crea né la cartella temporanea né il file parziale;
  * **ffmpeg fallisce durante la codifica:** `JOYPP_FFMPEG` punta a un eseguibile di prova che
    termina con codice 0 per `-hide_banner -version` e con codice ≠ 0 (es. 1, dopo aver scritto una
    riga su stderr) con gli argomenti di codifica, cioè dopo l'avvio della pipe. L'errore è
    `ffmpeg è terminato con un errore (codice 1)` seguito dalle righe `  ffmpeg: …`. Qui browser e
    cartella temporanea sono già stati avviati e creati: alla fine non devono restare né la
    cartella né un file parziale.

#### B.9.10 Riparto dei ruoli
| Ruolo | Compito |
|-------|---------|
| `@renderer` | `__joy_seek` in `dom.rs` (sempre emessa), controllo `data-joy-render` nello script di adattamento, regola CSS e guardie del mini-player (B.5) in modalità render. |
| `@exporter` | Sottocomando e aiuto in `main.rs`; in `src/exporter/render/` la ricerca degli eseguibili, i file temporanei, il client CDP minimale (WebSocket, JSON, base64), il campionamento, ffmpeg, il progresso e la rinomina finale. API indicativa: `OpzioniVideo { fps: Option<u32>, crf: u8 }` (`None` = nessun `--fps`: si usa l'intestazione, altrimenti 60, B.9.1), `render_file(ingresso, uscita, &OpzioniVideo, progresso) -> Risultato<EsitoVideo>`. Aggiorna `src/exporter/README.md`. |
| `@security` | Revisione di argomenti dei processi, espressioni CDP, URL, file temporanei, porta di debug e pulizia dello stderr; nuove costanti in `security::limiti` (B.9.7). |
| `@debugger` | Test di B.9.9; file minimi in `tests/repro/` per ogni bug del render. |
| `@compiler` | Nessuna modifica: la durata, le dimensioni del palco e i fotogrammi dichiarati (`fps: Option<u32>`) sono già in `ScenaCompilata`. |
| `@auditor` | `CHANGELOG.md` (`[Unreleased]`: «nuovo comando `joypp render` per esportare MP4»), coerenza con questa sezione, commit. |
