# `src/compiler` — Risoluzione, Timeline e Keyframe (`@compiler`)

Trasforma il `Programma` (AST) in una `ScenaCompilata`: una rappresentazione già
"appiattita" che il generatore può emettere senza fare calcoli.

## Fasi

1. **Validazione attori**: nomi duplicati, limiti anti-DoS (vedi `src/security`).
2. **Risoluzione spaziale**: ogni attore ottiene le coordinate assolute del proprio
   **centro**. Le posizioni relative (`a destra di "sole" di 200`) sono risolte
   ricorsivamente con rilevamento dei cicli.
3. **Risoluzione temporale**: i blocchi relativi (`Poi per 2 secondi:`) vengono ancorati
   alla fine del blocco precedente; si calcola la durata totale della scena.
4. **Canali di animazione**: ogni azione diventa un *segmento* su uno dei canali
   indipendenti `opacita`, `traslazione`, `scala`, `rotazione`. Segmenti sovrapposti sullo
   stesso canale dello stesso attore producono un errore leggibile.
5. **Pre-calcolo keyframe**: i segmenti diventano keyframe percentuali.
   Le coreografie (es. `pulsa`) sono espanse in Rust in keyframe espliciti.
6. **Easing** (`easing.rs`): le curve `elastico` e `rimbalzo` sono campionate
   matematicamente e convertite nella funzione CSS `linear(...)`.

Perché canali separati: le proprietà CSS individuali `translate`, `rotate`, `scale` e
`opacity` si compongono tra loro, quindi animazioni diverse sullo stesso attore non si
sovrascrivono (cosa che invece accadrebbe con un'unica proprietà `transform`).
