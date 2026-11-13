# Backlog v0.2.2: Performance & Reliability

**Data:** 2026-11-14

## Stato del ciclo

- **Unità 1, diagnostica del render** (`--profilo`, `--profilo-json`, SPECIFICA C.1): fatta. Commit
  `f2ade8f`, `5928623`, `7294965`. I rilievi R1, R2, R3 e A1 restano aperti (vedi Fase 4).
- **Fase 3, cattura degli screenshot** (V1, `optimizeForSpeed`, SPECIFICA C.2): fatta. Commit `2c1d3ba`
  e `5dc1461`. Circa -33% di tempo totale a 4K e -3% a 720p, con pixel identici a parità di superficie.
- **Seek** (`__joy_seek`, B.9.2): sospeso. Resta invariato fino alla decisione dell'utente su P1.
- **Preset ffmpeg**: non toccato. Le misure mostrano che l'encoder non è il collo di bottiglia
  (`benches/README.md`, Fase 3).

## P1: non determinazione del rendering tra esecuzioni (priorità alta, correttezza)

**Fatti misurati** (`benches/README.md`, «Non determinazione» e «Conclusione sulla correttezza di V1»):
- due render dello stesso binario non sono identici bit a bit;
- lo scostamento è intermittente: fino a 95 livelli su circa 8400 pixel a 720p, con fase inferiore a un
  fotogramma;
- colpisce V0 e V1 allo stesso modo: non è stato introdotto dalla cattura veloce.

**Ipotesi non verificate:**
- **A.** race tra lo stato del compositor e `__joy_seek`: il doppio `requestAnimationFrame` non garantisce
  che il compositor abbia applicato l'ultimo stato delle animazioni;
- **B.** dipendenza dalla storia dei seek: stato residuo tra chiamate successive di `__joy_seek`.

**Ruoli:** `@renderer` per il seek (B.9.2); `@debugger` per la riproduzione (ripetizioni, seek in ordine
diverso, confronto a livello di pixel).

**Stato:** il seek è congelato fino a decisione dell'utente.

**Criterio di chiusura:** due render dello stesso binario identici bit a bit a 720p e a 4K, oppure una
spiegazione verificata dello scostamento residuo.

## P2: pavimento di cattura a 720p

Circa 45-60 ms per screenshot a 720p. V1 non lo tocca. Il prossimo esperimento è il controllo dei
BeginFrame in modalità headless. Ipotesi, non verificata.

## Fase 4: robustezza

- Il nome `<file>.joypp-parziale` è deterministico: due render concorrenti sullo stesso `<file>` possono
  collidere.
- `Ffmpeg::finisci` non ha timeout: un ffmpeg bloccato blocca il render.
- **R1** (SPECIFICA C.1.8): il parziale del JSON segue i symlink e può troncare un file omonimo.
- **R2** (C.1.8): `NUL` e `CON` non sono intercettati prima del render.
- **R3** (C.1.8): i permessi del percorso JSON non sono verificati prima del render.
- **A1** (C.1.8): `json::stringa` non applica l'escape a `<` né a U+2028/U+2029.
- Flake raro: una cartella `joypp-render-*` può restare nella temp dopo un render.
- `rustfmt` non è installato: il formato del codice non è verificato.
