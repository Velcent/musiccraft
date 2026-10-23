//! Live control (`docs/spec/live-control.md`): the desktop app listens on a
//! localhost port and answers the same JSON-RPC as the stdio server, against its
//! open project; `musiccraft-mcp --connect` relays stdio to it.
//!
//! Connections are served on their own threads, but requests are only *handled*
//! where the app calls [`ControlServer::serve`] (its main thread, between
//! frames), so the project is never touched from another thread.

use std::hash::{BuildHasher, RandomState};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};

use crate::Host;

/// How long a connection waits for the window to answer one request.
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(120);

/// A request waiting for the main thread, with where its reply goes.
struct Request {
    message: Value,
    reply: Sender<Option<Value>>,
}

/// The app's end of the control channel.
pub struct ControlServer {
    requests: Receiver<Request>,
    port: u16,
}

/// A fresh 128-bit token as 32 hex digits. `RandomState` keys come from the
/// operating system's random source; the clock is mixed in for good measure.
#[must_use]
pub fn new_token() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let a = RandomState::new().hash_one((now, std::process::id()));
    let b = RandomState::new().hash_one((a, now));
    format!("{a:016x}{b:016x}")
}

/// Writes `token` to `path`, readable only by the user on Unix.
///
/// # Errors
///
/// When the file can't be written.
pub fn write_token(path: &Path, token: &str) -> io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(token.as_bytes())?;
    file.flush()
}

/// Reads a token file written by [`write_token`].
///
/// # Errors
///
/// When the file can't be read.
pub fn read_token(path: &Path) -> io::Result<String> {
    Ok(std::fs::read_to_string(path)?.trim().to_owned())
}

impl ControlServer {
    /// Listens on `127.0.0.1:port` (0 picks a free port). With a `token`, each
    /// connection must send it as its first line. `wake` is called whenever a
    /// request arrives, so the app can schedule a frame to answer it.
    ///
    /// # Errors
    ///
    /// When the port can't be opened.
    pub fn start(
        port: u16,
        token: Option<String>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))?;
        let port = listener.local_addr()?.port();
        let (tx, requests) = channel();
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let token = token.map(Arc::<str>::from);
        std::thread::Builder::new()
            .name("control-accept".into())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    let (tx, wake, token) = (tx.clone(), Arc::clone(&wake), token.clone());
                    let _ = std::thread::Builder::new()
                        .name("control-connection".into())
                        .spawn(move || serve_connection(stream, token.as_deref(), &tx, &*wake));
                }
            })?;
        Ok(Self { requests, port })
    }

    /// The port it listens on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Answers every waiting request against `host`, in arrival order. Returns
    /// how many were handled.
    pub fn serve<H: Host + ?Sized>(&self, host: &mut H) -> usize {
        let mut handled = 0;
        while let Ok(request) = self.requests.try_recv() {
            let _ = request.reply.send(crate::handle(host, &request.message));
            handled += 1;
        }
        handled
    }
}

fn serve_connection(
    stream: TcpStream,
    token: Option<&str>,
    requests: &Sender<Request>,
    wake: &(dyn Fn() + Send + Sync),
) {
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut lines = BufReader::new(read_half).lines();
    let mut out = stream;
    if let Some(token) = token {
        match lines.next() {
            Some(Ok(line)) if line.trim() == token => {}
            _ => return,
        }
    }
    for line in lines {
        let Ok(line) = line else {
            return;
        };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(batch)) => {
                let replies: Vec<Value> = batch
                    .into_iter()
                    .filter_map(|message| ask(requests, wake, message))
                    .collect();
                (!replies.is_empty()).then_some(Value::Array(replies))
            }
            Ok(message) => ask(requests, wake, message),
            Err(err) => Some(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("parse error: {err}") }
            })),
        };
        if let Some(reply) = reply
            && writeln!(out, "{reply}").and_then(|()| out.flush()).is_err()
        {
            return;
        }
    }
}

/// Hands one message to the main thread and waits for its reply.
fn ask(
    requests: &Sender<Request>,
    wake: &(dyn Fn() + Send + Sync),
    message: Value,
) -> Option<Value> {
    let id = message.get("id").cloned();
    let (reply, answer) = channel();
    if requests.send(Request { message, reply }).is_err() {
        return id.map(|id| unanswered(id, "MusicCraft's window has closed"));
    }
    wake();
    match answer.recv_timeout(ANSWER_TIMEOUT) {
        Ok(reply) => reply,
        Err(_) => id.map(|id| unanswered(id, "MusicCraft's window did not answer")),
    }
}

fn unanswered(id: Value, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": message } })
}

/// Relays `input` lines to a control channel at `addr` and its replies to
/// `output`, after sending `token`. Returns when either side closes.
///
/// # Errors
///
/// When the connection can't be made or the token can't be sent.
pub fn relay(
    addr: &str,
    token: Option<&str>,
    input: impl BufRead + Send + 'static,
    mut output: impl Write,
) -> io::Result<()> {
    let stream = TcpStream::connect(addr)?;
    let mut upstream = stream.try_clone()?;
    if let Some(token) = token {
        writeln!(upstream, "{token}")?;
    }
    std::thread::Builder::new()
        .name("control-relay".into())
        .spawn(move || {
            for line in input.lines() {
                let Ok(line) = line else {
                    break;
                };
                if writeln!(upstream, "{line}").is_err() {
                    break;
                }
            }
            // stdin closed: let the app see the end, so the replies stop too
            let _ = upstream.shutdown(std::net::Shutdown::Write);
        })?;
    for line in BufReader::new(stream).lines() {
        writeln!(output, "{}", line?)?;
        output.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_commands::Session;
    use musiccraft_project::Project;
    use std::sync::Mutex;

    /// Starts a server with a session served by a background "main thread".
    fn live(token: Option<&str>) -> (u16, Arc<Mutex<Session>>) {
        let session = Arc::new(Mutex::new(Session::new(
            Project::default(),
            musiccraft_cli::full_registry(),
        )));
        let server = ControlServer::start(0, token.map(str::to_owned), || {}).unwrap();
        let port = server.port();
        let shared = Arc::clone(&session);
        std::thread::spawn(move || {
            loop {
                server.serve(&mut *shared.lock().unwrap());
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        (port, session)
    }

    fn exchange(port: u16, token: Option<&str>, lines: &[&str]) -> Vec<Value> {
        let input = lines.join("\n") + "\n";
        let mut out = Vec::new();
        relay(
            &format!("127.0.0.1:{port}"),
            token,
            io::Cursor::new(input.into_bytes()),
            &mut out,
        )
        .unwrap();
        String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    const SET_TEMPO: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"project_set_tempo","arguments":{"args":["137"]}}}"#;

    #[test]
    fn tools_run_on_the_served_session() {
        let (port, session) = live(Some("secret"));
        let replies = exchange(
            port,
            Some("secret"),
            &[
                r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}"#,
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                SET_TEMPO,
            ],
        );
        assert_eq!(replies.len(), 2, "{replies:?}");
        assert_eq!(replies[0]["result"]["serverInfo"]["name"], "musiccraft");
        assert_eq!(replies[1]["result"]["isError"], false);
        let tempo = session.lock().unwrap().project().tempo_bpm();
        assert!((tempo - 137.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_wrong_token_gets_no_answer_and_changes_nothing() {
        let (port, session) = live(Some("secret"));
        assert!(exchange(port, Some("guess"), &[SET_TEMPO]).is_empty());
        assert!(exchange(port, None, &[SET_TEMPO]).is_empty());
        let tempo = session.lock().unwrap().project().tempo_bpm();
        assert!((tempo - 137.0).abs() > f64::EPSILON);
    }

    #[test]
    fn tokens_are_fresh_and_private() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        let path = std::env::temp_dir().join(format!("mc-token-{}", std::process::id()));
        write_token(&path, &a).unwrap();
        assert_eq!(read_token(&path).unwrap(), a);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let _ = std::fs::remove_file(&path);
    }
}
