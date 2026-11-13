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
* `appare sfumando` porta l'opacità al valore dichiarato dall'attore (default 1.0); se è la prima
  azione di opacità dell'attore, l'attore parte invisibile.
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
  rotazione, scala. `appare sfumando` su un gruppo lo fa partire invisibile, come per un attore (A.3).
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

---

### B.7 Questioni aperte
1. **Pivot esplicito dei gruppi** (es. `con centro a X, Y`): escluso dalla v0.2 per non aggiungere
   sintassi. Da valutare se il centro dell'AABB non basta.
2. **Orbita con centro mobile:** nella v0.2 il centro è statico (B.4.1) e il caso si risolve con i
   gruppi. Seguire il centro nella v0.3 richiederebbe di campionare la posizione di ALTRO.
3. **Verso predefinito di `orbita`:** antiorario. Va confermato dall'utente prima di implementarlo.
4. **Coefficienti della stima dei testi** (1.0 / 1.25 em, 1.5 em di altezza): prudenti ma
   sovrastimati del 30-50% per i testi latini. Si possono ridurre solo accettando il rischio che un
   testo resti in parte visibile.

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
