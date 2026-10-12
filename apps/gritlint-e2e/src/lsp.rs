use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const WIDGET: &str = "packages/widget/package.json";

struct Server {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Server {
    fn start(gritlint: &Path, directory: &Path) -> Result<Self, String> {
        let mut child = Command::new(gritlint)
            .arg("lsp")
            .current_dir(directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("start gritlint lsp: {error}"))?;
        let input = child.stdin.take().ok_or("gritlint lsp has no stdin")?;
        let output = child.stdout.take().ok_or("gritlint lsp has no stdout")?;
        Ok(Self {
            child,
            input,
            output: BufReader::new(output),
        })
    }

    fn send(&mut self, message: &Value) -> Result<(), String> {
        let body = message.to_string();
        write!(self.input, "Content-Length: {}\r\n\r\n{body}", body.len())
            .and_then(|()| self.input.flush())
            .map_err(|error| format!("write to gritlint lsp: {error}"))
    }

    fn receive(&mut self) -> Result<Value, String> {
        let mut length = None;
        loop {
            let mut line = String::new();
            self.output
                .read_line(&mut line)
                .map_err(|error| format!("read from gritlint lsp: {error}"))?;
            if line.is_empty() {
                return Err("gritlint lsp closed its output".to_owned());
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length: ") {
                length = value.parse::<usize>().ok();
            }
        }
        let mut body = vec![0; length.ok_or("a message without Content-Length")?];
        self.output
            .read_exact(&mut body)
            .map_err(|error| format!("read a message body: {error}"))?;
        serde_json::from_slice(&body)
            .map_err(|error| format!("a message that is not JSON: {error}"))
    }

    fn response(&mut self, id: u64) -> Result<Value, String> {
        loop {
            let message = self.receive()?;
            if message.get("id").and_then(Value::as_u64) == Some(id) {
                return Ok(message);
            }
        }
    }

    fn published(&mut self, uri: &str) -> Result<Value, String> {
        loop {
            let message = self.receive()?;
            let params = message.get("params").cloned().unwrap_or(Value::Null);
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
                && params.get("uri").and_then(Value::as_str) == Some(uri)
            {
                return Ok(params);
            }
        }
    }

    fn initialize(&mut self) -> Result<Value, String> {
        self.send(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "processId": null, "rootUri": null, "capabilities": {} }
        }))?;
        self.response(1)
    }

    fn notify(&mut self, method: &str, params: &Value) -> Result<(), String> {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    fn exit(mut self) -> Result<i32, String> {
        self.notify("exit", &Value::Null)?;
        drop(self.input);
        self.child
            .wait()
            .map_err(|error| format!("wait for gritlint lsp: {error}"))?
            .code()
            .ok_or_else(|| "gritlint lsp was killed by a signal".to_owned())
    }
}

pub fn journey(gritlint: &Path, root: &Path, findings: &[Value]) -> Result<(), String> {
    serves_a_finding(gritlint, root, findings)?;
    exits_1_without_shutdown(gritlint, root)?;
    refuses_a_directory_with_no_rules(gritlint, root)
}

fn expected_range(findings: &[Value]) -> Result<Value, String> {
    let range = findings
        .iter()
        .filter(|finding| finding.pointer("/location/path").and_then(Value::as_str) == Some(WIDGET))
        .find_map(|finding| finding.pointer("/location/range"))
        .ok_or_else(|| format!("check reported no ranged finding in {WIDGET}"))?;
    let zero_based = |pointer: &str| {
        range
            .pointer(pointer)
            .and_then(Value::as_u64)
            .and_then(|value| value.checked_sub(1))
            .ok_or_else(|| format!("no one-based {pointer} in {range}"))
    };
    Ok(json!({
        "start": { "line": zero_based("/start/line")?, "character": zero_based("/start/column")? },
        "end": { "line": zero_based("/end/line")?, "character": zero_based("/end/column")? }
    }))
}

fn serves_a_finding(gritlint: &Path, root: &Path, findings: &[Value]) -> Result<(), String> {
    let expected = expected_range(findings)?;
    let mut server = Server::start(gritlint, root)?;
    let initialized = server.initialize()?;
    if initialized.get("error").is_some() {
        return Err(format!("initialize failed: {initialized}"));
    }
    server.notify("initialized", &json!({}))?;
    let path = root.join(WIDGET);
    let uri = format!("file://{}", path.display());
    let text = fs::read_to_string(&path).map_err(|error| format!("read {WIDGET}: {error}"))?;
    server.notify(
        "textDocument/didOpen",
        &json!({ "textDocument": { "uri": uri, "languageId": "json", "version": 1, "text": text } }),
    )?;
    let published = server.published(&uri)?;
    let diagnostics = published
        .get("diagnostics")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("no diagnostics array in {published}"))?;
    let seen: Vec<(Option<&str>, Option<&Value>)> = diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.get("code").and_then(Value::as_str),
                diagnostic.get("range"),
            )
        })
        .collect();
    if seen != vec![(Some("repository-url-mismatch"), Some(&expected))] {
        return Err(format!(
            "expected one repository-url-mismatch at {expected}, read {published}"
        ));
    }
    server.send(&json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }))?;
    server.response(2)?;
    match server.exit()? {
        0 => Ok(()),
        code => Err(format!("`shutdown` then `exit` ended with {code}, not 0")),
    }
}

fn exits_1_without_shutdown(gritlint: &Path, root: &Path) -> Result<(), String> {
    let mut server = Server::start(gritlint, root)?;
    server.initialize()?;
    server.notify("initialized", &json!({}))?;
    match server.exit()? {
        1 => Ok(()),
        code => Err(format!(
            "`exit` without `shutdown` ended with {code}, not 1"
        )),
    }
}

fn refuses_a_directory_with_no_rules(gritlint: &Path, root: &Path) -> Result<(), String> {
    let empty = root.join("lsp-empty");
    fs::create_dir_all(&empty).map_err(|error| format!("create lsp-empty: {error}"))?;
    let mut server = Server::start(gritlint, &empty)?;
    let refused = server.initialize()?;
    let code = refused.pointer("/error/code").and_then(Value::as_i64);
    let message = refused
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if code != Some(-32803) || !message.starts_with("no rules are enabled: ") {
        return Err(format!("a directory with no rules answered {refused}"));
    }
    match server.exit()? {
        2 => Ok(()),
        code => Err(format!("a refused initialize ended with {code}, not 2")),
    }
}
