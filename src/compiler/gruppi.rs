//! Albero dei gruppi (SPECIFICA B.3): appartenenza, controlli di B.3.8, profondità e
//! ordine di emissione (pre-ordine con l'ordine dei fratelli di B.3.5).

use std::collections::HashMap;

use crate::ast::{Attore, Forma};
use crate::errore::{Errore, Risultato};
use crate::parser::suggerisci;
use crate::security::limiti;

/// Struttura ad albero degli attori. Tutti gli indici sono quelli di `Programma::attori`
/// (ordine di creazione).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Albero {
    /// Gruppo genitore diretto di ogni attore (`None` = palco).
    pub genitore: Vec<Option<usize>>,
    /// Figli diretti di ogni gruppo, nell'ordine in cui sono elencati nella frase del gruppo
    /// (vuoto per gli attori semplici).
    pub figli: Vec<Vec<usize>>,
    /// Numero di antenati di ogni attore (0 per i figli del palco).
    pub livello: Vec<usize>,
    /// Ordine di emissione: pre-ordine dell'albero, con i fratelli ordinati per il loro
    /// discendente creato per primo (B.3.5).
    pub preordine: Vec<usize>,
}

/// Costruisce l'albero dei gruppi e applica i controlli di B.3.8.
///
/// # Errori
/// Membri inesistenti, ripetuti, creati dopo il gruppo, già in un altro gruppo, un gruppo
/// che contiene sé stesso o vuoto, annidamento oltre `MAX_PROFONDITA_GRUPPI`. Tutti gli
/// errori citano la riga del gruppo.
pub(super) fn costruisci(attori: &[Attore], indici: &HashMap<&str, usize>) -> Risultato<Albero> {
    let n = attori.len();
    let mut genitore: Vec<Option<usize>> = vec![None; n];
    let mut figli: Vec<Vec<usize>> = vec![Vec::new(); n];
    // Altezza del sottoalbero in gruppi: 0 per un attore semplice, 1 per un gruppo di soli
    // attori semplici, ... È la profondità massima di annidamento che il gruppo introduce.
    let mut altezza = vec![0usize; n];

    for (g, gruppo) in attori.iter().enumerate() {
        let Forma::Gruppo { membri } = &gruppo.forma else {
            continue;
        };
        let riga = gruppo.riga;
        if membri.is_empty() {
            return Err(gruppo_vuoto(&gruppo.nome, riga));
        }
        let mut lista = Vec::with_capacity(membri.len());
        for nome in membri {
            let Some(&m) = indici.get(nome.as_str()) else {
                return Err(membro_inesistente(nome, indici, riga, &gruppo.nome));
            };
            if m == g {
                return Err(Errore::alla_riga(riga, "un gruppo non può contenere sé stesso")
                    .con_suggerimento("elenca solo altri attori o gruppi, creati prima del gruppo"));
            }
            if lista.contains(&m) {
                return Err(Errore::alla_riga(
                    riga,
                    format!("\"{nome}\" compare due volte nel gruppo \"{}\"", gruppo.nome),
                )
                .con_suggerimento("elenca ogni membro una volta sola"));
            }
            if m > g {
                return Err(Errore::alla_riga(
                    riga,
                    format!("\"{nome}\" è creato alla riga {}, dopo il gruppo \"{}\"", attori[m].riga, gruppo.nome),
                )
                .con_suggerimento("crea il gruppo dopo tutti i suoi membri"));
            }
            if let Some(altro) = genitore[m] {
                return Err(Errore::alla_riga(
                    riga,
                    format!(
                        "\"{nome}\" fa già parte del gruppo \"{}\" (riga {})",
                        attori[altro].nome, attori[altro].riga
                    ),
                )
                .con_suggerimento(format!(
                    "un attore può appartenere a un solo gruppo: raggruppa i gruppi, es. «Crea un gruppo chiamato \"tutto\" con \"{}\" e \"{}\".»",
                    attori[altro].nome, gruppo.nome
                )));
            }
            genitore[m] = Some(g);
            lista.push(m);
        }
        altezza[g] = 1 + lista.iter().map(|&m| altezza[m]).max().unwrap_or(0);
        if altezza[g] > limiti::MAX_PROFONDITA_GRUPPI {
            return Err(Errore::alla_riga(
                riga,
                format!("gruppi annidati troppo in profondità (massimo {} livelli)", limiti::MAX_PROFONDITA_GRUPPI),
            )
            .con_suggerimento("riduci i livelli di gruppi uno dentro l'altro"));
        }
        figli[g] = lista;
    }

    // Il genitore è sempre creato dopo i figli: scorrendo all'indietro è già noto.
    let mut livello = vec![0usize; n];
    for i in (0..n).rev() {
        if let Some(g) = genitore[i] {
            livello[i] = livello[g] + 1;
        }
    }

    // Chiave d'ordine: il discendente creato per primo (i figli precedono il gruppo).
    let mut chiave: Vec<usize> = (0..n).collect();
    for g in 0..n {
        if let Some(min) = figli[g].iter().map(|&m| chiave[m]).min() {
            chiave[g] = chiave[g].min(min);
        }
    }
    let ordina = |lista: &mut Vec<usize>| lista.sort_by_key(|&i| chiave[i]);

    let mut radici: Vec<usize> = (0..n).filter(|&i| genitore[i].is_none()).collect();
    ordina(&mut radici);
    let mut preordine = Vec::with_capacity(n);
    let mut pila: Vec<usize> = radici.into_iter().rev().collect();
    while let Some(i) = pila.pop() {
        preordine.push(i);
        let mut f = figli[i].clone();
        ordina(&mut f);
        pila.extend(f.into_iter().rev());
    }

    Ok(Albero { genitore, figli, livello, preordine })
}

fn gruppo_vuoto(nome: &str, riga: usize) -> Errore {
    Errore::alla_riga(riga, format!("il gruppo \"{nome}\" deve contenere almeno un membro"))
        .con_suggerimento("es. «Crea un gruppo chiamato \"g\" con \"a\" e \"b\".»")
}

fn membro_inesistente(nome: &str, indici: &HashMap<&str, usize>, riga: usize, gruppo: &str) -> Errore {
    let mut nomi: Vec<&str> = indici.keys().copied().filter(|n| *n != gruppo).collect();
    nomi.sort_unstable();
    let e = Errore::alla_riga(riga, format!("non esiste nessun attore o gruppo chiamato \"{nome}\""));
    match suggerisci(nome, &nomi) {
        Some(s) => e.con_suggerimento(format!("forse intendevi \"{s}\"?")),
        None => e.con_suggerimento("crea prima i membri, poi il gruppo"),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ast::{Ancora, Posizione, Stile};

    fn attore(nome: &str, riga: usize, membri: Option<&[&str]>) -> Attore {
        Attore {
            nome: nome.into(),
            forma: match membri {
                Some(m) => Forma::Gruppo { membri: m.iter().map(|s| s.to_string()).collect() },
                None => Forma::Cerchio { raggio: 10.0 },
            },
            stile: Stile::default(),
            posizione: Posizione::Palco(Ancora::Centro),
            riga,
        }
    }

    fn albero(attori: &[Attore]) -> Risultato<Albero> {
        let indici = attori.iter().enumerate().map(|(i, a)| (a.nome.as_str(), i)).collect();
        costruisci(attori, &indici)
    }

    #[test]
    fn preordine_e_ordine_dei_fratelli() {
        // a, b, c, g1 = {c, a}, d, g2 = {g1, d}
        let attori = [
            attore("a", 1, None),
            attore("b", 2, None),
            attore("c", 3, None),
            attore("g1", 4, Some(&["c", "a"])),
            attore("d", 5, None),
            attore("g2", 6, Some(&["d", "g1"])),
        ];
        let t = albero(&attori).unwrap();
        assert_eq!(t.genitore, [Some(3), None, Some(3), Some(5), Some(5), None]);
        assert_eq!(t.livello, [2, 0, 2, 1, 1, 0]);
        // g2 prende il posto di "a" (riga 1), quindi precede "b"; dentro g1 "a" precede "c".
        assert_eq!(t.preordine, [5, 3, 0, 2, 4, 1]);
    }

    #[test]
    fn senza_gruppi_l_ordine_non_cambia() {
        let attori = [attore("a", 1, None), attore("b", 2, None)];
        let t = albero(&attori).unwrap();
        assert_eq!(t.preordine, [0, 1]);
        assert_eq!(t.genitore, [None, None]);
    }

    fn messaggio(attori: &[Attore]) -> String {
        albero(attori).unwrap_err().messaggio
    }

    #[test]
    fn controlli_dei_gruppi() {
        assert_eq!(
            messaggio(&[attore("a", 1, None), attore("g", 2, Some(&["ab"]))]),
            "non esiste nessun attore o gruppo chiamato \"ab\""
        );
        assert_eq!(messaggio(&[attore("g", 1, Some(&["g"]))]), "un gruppo non può contenere sé stesso");
        assert_eq!(
            messaggio(&[attore("a", 1, None), attore("g", 2, Some(&["a", "a"]))]),
            "\"a\" compare due volte nel gruppo \"g\""
        );
        assert_eq!(
            messaggio(&[attore("g", 1, Some(&["a"])), attore("a", 2, None)]),
            "\"a\" è creato alla riga 2, dopo il gruppo \"g\""
        );
        assert_eq!(
            messaggio(&[attore("a", 1, None), attore("g1", 2, Some(&["a"])), attore("g2", 3, Some(&["a"]))]),
            "\"a\" fa già parte del gruppo \"g1\" (riga 2)"
        );
        assert_eq!(messaggio(&[attore("g", 7, Some(&[]))]), "il gruppo \"g\" deve contenere almeno un membro");
        let e = albero(&[attore("a", 1, None), attore("g", 2, Some(&["ab"]))]).unwrap_err();
        assert_eq!(e.riga, Some(2));
        assert_eq!(e.suggerimento.as_deref(), Some("forse intendevi \"a\"?"));
    }

    #[test]
    fn profondita_massima() {
        let nomi: Vec<String> = (0..=limiti::MAX_PROFONDITA_GRUPPI + 1).map(|i| format!("n{i}")).collect();
        let costruisci_catena = |livelli: usize| -> Vec<Attore> {
            let mut v = vec![attore(&nomi[0], 1, None)];
            for (i, coppia) in nomi.windows(2).take(livelli).enumerate() {
                v.push(attore(&coppia[1], i + 2, Some(&[coppia[0].as_str()])));
            }
            v
        };
        assert!(albero(&costruisci_catena(limiti::MAX_PROFONDITA_GRUPPI)).is_ok());
        assert_eq!(
            messaggio(&costruisci_catena(limiti::MAX_PROFONDITA_GRUPPI + 1)),
            "gruppi annidati troppo in profondità (massimo 16 livelli)"
        );
    }
}
