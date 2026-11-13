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
