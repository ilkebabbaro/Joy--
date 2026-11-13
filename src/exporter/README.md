# `src/exporter` — Packaging, I/O e CLI (`@exporter`)

Collega la pipeline al file system.

- `compila_con_avvisi`: punto unico della pipeline (limite di dimensione, `parser → compiler →
  codegen`). Restituisce una `Compilazione { html, avvisi }`.
- `compila_sorgente`: wrapper che delega a `compila_con_avvisi` e restituisce solo l'HTML
  (firma stabile, usata dai test; gli avvisi vengono scartati).
- `compila_file`: legge un `.joy`, compila con `compila_con_avvisi` e scrive il `.html`.
  Restituisce `Risultato<Esito>`, dove `Esito { byte, avvisi }` contiene i byte scritti e gli avvisi.
- `osserva`: modalità *watch*, ricompila quando il sorgente cambia (polling dell'orario di
  modifica, senza dipendenze esterne). La callback riceve `Risultato<Esito>` a ogni compilazione.
- Opzioni: percorso di output (`-o`), output leggibile (`--pretty`, default: compatto),
  `--watch`.

**Avvisi (SPECIFICA B.6).** Gli avvisi non bloccanti del compilatore non cambiano l'esito:
la compilazione riuscita scrive l'HTML e termina con exit code 0. La CLI li stampa su
**stderr**, in ordine di riga, prima del messaggio di successo `✔ …` su stdout. Se la
compilazione fallisce si stampa solo l'errore. In `--watch` gli avvisi sono ristampati a
ogni ricompilazione riuscita.

La CLI vera e propria (`src/main.rs`) si limita a interpretare gli argomenti, a stampare
avvisi ed esito e a delegare a questo modulo.
