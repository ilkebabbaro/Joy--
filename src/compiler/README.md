# `src/compiler` — Risoluzione, Timeline e Keyframe (`@compiler`)

Trasforma il `Programma` (AST) in una `ScenaCompilata`: una rappresentazione già
"appiattita" che il generatore può emettere senza fare calcoli.

## Fasi

1. **Validazione attori**: nomi duplicati, limiti anti-DoS (vedi `src/security`), compreso
   lo stato iniziale `scala` (0 ≤ s0 ≤ `MAX_SCALA`) e `rotazione` (|r0| ≤ `MAX_GRADI`) di
   SPECIFICA B.1. Gli errori citano la riga di `Crea`.
2. **Risoluzione spaziale**: ogni attore ottiene le coordinate assolute del proprio
   **centro**. Le posizioni relative (`a destra di "sole" di 200`) sono risolte
   in modo iterativo con rilevamento dei cicli. Se il riferimento non esiste, il
   suggerimento «forse intendevi …?» non propone mai l'attore che si sta posizionando
   (i candidati sono ordinati, così il suggerimento è deterministico).
3. **Risoluzione temporale**: i blocchi relativi (`Poi per 2 secondi:`) vengono ancorati
   alla fine del blocco precedente; si calcola la durata totale della scena.
4. **Canali di animazione**: ogni azione diventa un *segmento* su uno dei canali
   indipendenti `opacita`, `traslazione`, `scala`, `rotazione`. Segmenti sovrapposti sullo
   stesso canale dello stesso attore producono un errore leggibile.
5. **Pre-calcolo keyframe**: i segmenti diventano keyframe percentuali.
   Le coreografie (es. `pulsa`) sono espanse in Rust in keyframe espliciti.
6. **Avvisi** (SPECIFICA B.6): problemi non bloccanti raccolti in `ScenaCompilata::avvisi`
   (`Vec<Avviso>`, tipo definito in `src/errore.rs`), ordinati per riga e deduplicati
   (stessa riga + stesso messaggio = uno solo). Il compilatore non li stampa: lo fa
   `@exporter` su stderr. Avvisi attuali: «rotazione invisibile su un cerchio», per
   `ruota di` su un `cerchio` (riga dell'azione) e per `rotazione` ≠ 0 alla creazione di un
   `cerchio` (riga di `Crea`).
7. **Easing** (`easing.rs`): le curve `elastico` e `rimbalzo` sono campionate
   matematicamente e convertite nella funzione CSS `linear(...)`.

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

`AttoreCompilato` espone `scala_iniziale` e `rotazione_iniziale`: il generatore li emette
come `scale` / `rotate` nella regola statica del wrapper `#aN` quando differiscono da 1 e 0,
anche se l'attore ha animazioni.
