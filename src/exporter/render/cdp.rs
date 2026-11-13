//! Sessione del Chrome DevTools Protocol su WebSocket (loopback), con la sola libreria standard.
//!
//! Comandi `{"id":n,"method":…,"params":{…}}`, risposte associate per `id`, eventi in coda.
//! Ogni attesa ha una scadenza.

use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use super::json::{self, Valore};
use super::websocket::{Connessione, ErroreWs, handshake};

/// Numero massimo di eventi conservati in coda (i più vecchi vengono scartati).
const MAX_EVENTI: usize = 256;

/// Errori della sessione CDP.
#[derive(Debug)]
pub enum ErroreCdp {
    /// Nessuna risposta entro la scadenza.
    Timeout,
    /// Il browser ha risposto con un campo `error` (messaggio).
    Browser(String),
    /// Risposta o messaggio non valido.
    Protocollo(String),
    /// Connessione chiusa dal browser.
    Chiuso,
    /// Messaggio oltre il limite.
    TroppoGrande,
    /// Altro errore di I/O.
    Io(String),
}

impl From<ErroreWs> for ErroreCdp {
    fn from(e: ErroreWs) -> Self {
        if e.e_timeout() {
            return ErroreCdp::Timeout;
        }
        match e {
            ErroreWs::Chiuso => ErroreCdp::Chiuso,
            ErroreWs::TroppoGrande => ErroreCdp::TroppoGrande,
            ErroreWs::Protocollo(m) => ErroreCdp::Protocollo(m),
            ErroreWs::Io(e) => ErroreCdp::Io(e.to_string()),
        }
    }
}

/// Sessione verso un browser: connessione WebSocket e stato dei comandi.
pub struct Sessione {
    ws: Connessione<TcpStream>,
    prossimo_id: u64,
    id_sessione: Option<String>,
    eventi: VecDeque<Valore>,
}

impl Sessione {
    /// Si collega a `127.0.0.1:porta` (mai `localhost`) e fa l'handshake sul `percorso`.
    ///
    /// # Errori
    /// Connessione rifiutata, timeout, handshake non valido.
    pub fn connetti(porta: u16, percorso: &str, timeout: Duration) -> Result<Self, ErroreCdp> {
        let indirizzo = SocketAddr::from((Ipv4Addr::LOCALHOST, porta));
        let mut flusso = TcpStream::connect_timeout(&indirizzo, timeout)
            .map_err(|e| ErroreCdp::Io(e.to_string()))?;
        flusso.set_read_timeout(Some(timeout)).map_err(|e| ErroreCdp::Io(e.to_string()))?;
        flusso.set_write_timeout(Some(timeout)).map_err(|e| ErroreCdp::Io(e.to_string()))?;
        let _ = flusso.set_nodelay(true);
        handshake(&mut flusso, &format!("127.0.0.1:{porta}"), percorso)?;
        Ok(Self {
            ws: Connessione::nuova(flusso),
            prossimo_id: 1,
            id_sessione: None,
            eventi: VecDeque::new(),
        })
    }

    /// Da ora i comandi includono `"sessionId"`.
    pub fn imposta_sessione(&mut self, id: String) {
        self.id_sessione = Some(id);
    }

    fn scadenza(&self, fine: Instant) -> Result<(), ErroreCdp> {
        let resto = fine.saturating_duration_since(Instant::now());
        if resto.is_zero() {
            return Err(ErroreCdp::Timeout);
        }
        self.ws.flusso().set_read_timeout(Some(resto)).map_err(|e| ErroreCdp::Io(e.to_string()))
    }

    fn ricevi_json(&mut self, fine: Instant) -> Result<Valore, ErroreCdp> {
        self.scadenza(fine)?;
        let testo = self.ws.ricevi()?;
        json::analizza(&testo).map_err(|e| ErroreCdp::Protocollo(format!("JSON non valido: {e}")))
    }

    fn accoda(&mut self, v: Valore) {
        if self.eventi.len() >= MAX_EVENTI {
            self.eventi.pop_front();
        }
        self.eventi.push_back(v);
    }

    /// Scarta gli eventi in coda.
    pub fn svuota_eventi(&mut self) {
        self.eventi.clear();
    }

    /// Invia un comando e attende la risposta; restituisce il campo `result`.
    /// `params` è un oggetto JSON già serializzato (testo costante più numeri).
    ///
    /// # Errori
    /// Timeout, `error` del browser, risposta malformata, connessione chiusa.
    pub fn comando(&mut self, metodo: &str, params: &str, timeout: Duration) -> Result<Valore, ErroreCdp> {
        self.invia_comando(metodo, params, timeout, true)
    }

    /// Come [`Sessione::comando`], ma senza `sessionId`: per i comandi del browser (`Browser.close`).
    ///
    /// # Errori
    /// Come [`Sessione::comando`].
    pub fn comando_globale(&mut self, metodo: &str, params: &str, timeout: Duration) -> Result<Valore, ErroreCdp> {
        self.invia_comando(metodo, params, timeout, false)
    }

    fn invia_comando(
        &mut self,
        metodo: &str,
        params: &str,
        timeout: Duration,
        con_sessione: bool,
    ) -> Result<Valore, ErroreCdp> {
        let id = self.prossimo_id;
        self.prossimo_id += 1;
        let sessione = match (&self.id_sessione, con_sessione) {
            (Some(s), true) => format!(",\"sessionId\":{}", json::stringa(s)),
            _ => String::new(),
        };
        let msg = format!(
            "{{\"id\":{id},\"method\":{},\"params\":{params}{sessione}}}",
            json::stringa(metodo)
        );
        self.ws.invia_testo(&msg)?;
        let fine = Instant::now() + timeout;
        loop {
            let v = self.ricevi_json(fine)?;
            match v.campo("id").and_then(Valore::numero) {
                Some(n) if n == id as f64 => {
                    if let Some(e) = v.campo("error") {
                        let m = e.campo("message").and_then(Valore::testo).unwrap_or("errore sconosciuto");
                        return Err(ErroreCdp::Browser(m.to_string()));
                    }
                    return Ok(v.campo("result").cloned().unwrap_or(Valore::Null));
                }
                Some(_) => {}
                None => self.accoda(v),
            }
        }
    }

    /// Attende un evento con il `metodo` dato (cercando prima in coda).
    ///
    /// # Errori
    /// Timeout o errori di connessione.
    pub fn attendi_evento(&mut self, metodo: &str, timeout: Duration) -> Result<Valore, ErroreCdp> {
        let corrisponde = |v: &Valore| v.campo("method").and_then(Valore::testo) == Some(metodo);
        if let Some(pos) = self.eventi.iter().position(corrisponde) {
            return Ok(self.eventi.remove(pos).unwrap_or(Valore::Null));
        }
        let fine = Instant::now() + timeout;
        loop {
            let v = self.ricevi_json(fine)?;
            if v.campo("id").is_some() {
                continue;
            }
            if corrisponde(&v) {
                return Ok(v);
            }
            self.accoda(v);
        }
    }
}

/// Estrae dall'esito di `Runtime.evaluate` il valore, trasformando `exceptionDetails` in errore.
///
/// # Errori
/// Messaggio dell'eccezione JS, se presente.
pub fn valore_valutato(risultato: &Valore) -> Result<&Valore, String> {
    if let Some(e) = risultato.campo("exceptionDetails") {
        let m = e
            .campo("exception")
            .and_then(|x| x.campo("description"))
            .and_then(Valore::testo)
            .or_else(|| e.campo("text").and_then(Valore::testo))
            .unwrap_or("eccezione JavaScript");
        return Err(m.to_string());
    }
    risultato
        .campo("result")
        .and_then(|r| r.campo("value"))
        .ok_or_else(|| "risultato di Runtime.evaluate senza valore".to_string())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valore_ok_ed_eccezione() {
        let ok = json::analizza(r#"{"result":{"type":"number","value":250}}"#).unwrap();
        assert_eq!(valore_valutato(&ok).unwrap().numero(), Some(250.0));
        let ko = json::analizza(
            r#"{"result":{"type":"object"},"exceptionDetails":{"text":"Uncaught","exception":{"description":"TypeError: x"}}}"#,
        )
        .unwrap();
        assert_eq!(valore_valutato(&ko).unwrap_err(), "TypeError: x");
        let vuoto = json::analizza(r#"{"result":{"type":"undefined"}}"#).unwrap();
        assert!(valore_valutato(&vuoto).is_err());
    }
}
