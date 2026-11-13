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