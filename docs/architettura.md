# Architettura del compilatore Joy++

```
 sorgente .joy
      │
      ▼
 ┌────────┐ token ┌────────┐  AST  ┌──────────┐ ScenaCompilata ┌─────────┐ String ┌──────────┐
 │ lexer  │──────►│ parser │──────►│ compiler │───────────────►│ codegen │───────►│ exporter │──► .html
 └────────┘       └────────┘       └──────────┘                └─────────┘        └──────────┘
 src/parser       src/parser        src/compiler                src/codegen        src/exporter
                       │  (src/ast)       │                          │
                       └──────────────────┴──── src/security ────────┘
                                   (limiti, escape, validazione)
```

Ogni fase restituisce `Result<_, Errore>` (`src/errore.rs`): il primo errore interrompe la
pipeline e viene mostrato all'utente con riga e suggerimento.
