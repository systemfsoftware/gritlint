#![allow(clippy::expect_used, clippy::disallowed_methods)]

use gritlint_core::decode;
use gritlint_core::domain::{EmbeddedFile, Pack, RelPath};
use gritlint_core::lsp::{self, Ending};
use gritlint_core::shell;
use gritlint_core::workspace::WorkspaceInputs;
use lsp_server::{Connection, Message, Notification, Request, RequestId, ResponseError};
use lsp_types::{Diagnostic, NumberOrString, PublishDiagnosticsParams};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(30);

fn fixture(relative: &str) -> PathBuf {
    fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative),
    )
    .expect("a committed fixture")
}

fn bundled_packs() -> Vec<Pack> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let tree = shell::read_tree(&dir).expect("read the bundled packs");
    let files: Vec<EmbeddedFile> = tree
        .files()
        .iter()
        .map(|file| {
            EmbeddedFile::new(
                RelPath::from_root(tree.root(), file.path())
                    .expect("a file below the packs root")
                    .as_str()
                    .to_owned(),
                file.contents().to_owned(),
            )
        })
        .collect();
    decode::embedded_packs(&files).expect("decode the bundled packs")
}

fn uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

struct Session {
    client: Connection,
    server: JoinHandle<Ending>,
}

impl Session {
    fn start(root: &str, rules: &[&str], packs: Vec<Pack>) -> Self {
        let inputs = WorkspaceInputs {
            config: None,
            rules: rules
                .iter()
                .map(|rule| fixture(&format!("lsp/rules/{rule}.md")))
                .collect(),
            root: fixture(root),
        };
        let (server, client) = Connection::memory();
        let server = thread::spawn(move || lsp::serve(&server, &inputs, &packs));
        Self { client, server }
    }

    fn send(&self, message: Message) {
        self.client
            .sender
            .send(message)
            .expect("the server is listening");
    }

    fn notify(&self, method: &str, params: Value) {
        self.send(Notification::new(method.to_owned(), params).into());
    }

    fn initialize(&self, encodings: &[&str]) -> Result<Value, ResponseError> {
        self.send(
            Request::new(
                RequestId::from(1),
                "initialize".to_owned(),
                json!({
                    "processId": null,
                    "rootUri": null,
                    "capabilities": { "general": { "positionEncodings": encodings } }
                }),
            )
            .into(),
        );
        let response = self.response();
        if response.is_ok() {
            self.notify("initialized", json!({}));
        }
        response
    }

    fn response(&self) -> Result<Value, ResponseError> {
        loop {
            match self.client.receiver.recv_timeout(WAIT).expect("a reply") {
                Message::Response(response) => return response.response_result,
                Message::Notification(_) | Message::Request(_) => {}
            }
        }
    }

    fn published(&self) -> PublishDiagnosticsParams {
        loop {
            match self
                .client
                .receiver
                .recv_timeout(WAIT)
                .expect("a notification")
            {
                Message::Notification(notification)
                    if notification.method == "textDocument/publishDiagnostics" =>
                {
                    return serde_json::from_value(notification.params)
                        .expect("publishDiagnostics params");
                }
                Message::Notification(_) | Message::Request(_) | Message::Response(_) => {}
            }
        }
    }

    fn published_for(&self, documents: &[&Path]) -> BTreeMap<String, Vec<Diagnostic>> {
        let wanted: Vec<String> = documents.iter().map(|path| uri(path)).collect();
        let mut latest = BTreeMap::new();
        while !wanted.iter().all(|uri| latest.contains_key(uri)) {
            let published = self.published();
            latest.insert(published.uri.as_str().to_owned(), published.diagnostics);
        }
        latest
    }

    fn open(&self, path: &Path, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri(path), "languageId": "json", "version": 1, "text": text } }),
        );
    }

    fn change(&self, path: &Path, text: &str) {
        self.notify(
            "textDocument/didChange",
            json!({ "textDocument": { "uri": uri(path), "version": 2 }, "contentChanges": [{ "text": text }] }),
        );
    }

    fn close(&self, path: &Path) {
        self.notify(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri(path) } }),
        );
    }

    fn shutdown(self) -> Ending {
        self.send(Request::new(RequestId::from(2), "shutdown".to_owned(), Value::Null).into());
        assert_eq!(self.response().err().map(|error| error.message), None);
        self.notify("exit", Value::Null);
        self.server.join().expect("the server thread ends")
    }

    fn exit(self) -> Ending {
        self.notify("exit", Value::Null);
        self.server.join().expect("the server thread ends")
    }
}

fn summary(diagnostics: &[Diagnostic]) -> Vec<(String, (u32, u32, u32, u32))> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let code = match &diagnostic.code {
                Some(NumberOrString::String(code)) => code.clone(),
                Some(NumberOrString::Number(number)) => number.to_string(),
                None => String::new(),
            };
            let range = diagnostic.range;
            (
                code,
                (
                    range.start.line,
                    range.start.character,
                    range.end.line,
                    range.end.character,
                ),
            )
        })
        .collect()
}

fn single_session() -> (Session, PathBuf) {
    let session = Session::start("lsp/single", &["license"], Vec::new());
    assert_eq!(
        session
            .initialize(&["utf-16"])
            .err()
            .map(|error| error.message),
        None
    );
    (session, fixture("lsp/single/package.json"))
}

#[test]
fn a_finding_in_the_unsaved_buffer_becomes_one_diagnostic() {
    let (session, document) = single_session();

    session.open(&document, "{\n  \"license\": \"UNLICENSED\"\n}\n");
    let opened = session.published();
    assert_eq!(opened.uri.as_str(), uri(&document));
    assert_eq!(
        summary(&opened.diagnostics),
        vec![("unlicensed".to_owned(), (1, 2, 1, 25))]
    );
    let diagnostic = opened.diagnostics.first().expect("one diagnostic");
    assert_eq!(diagnostic.source.as_deref(), Some("gritlint"));
    assert_eq!(
        diagnostic.message,
        "A package does not declare `UNLICENSED`.\nChoose a license."
    );

    assert_eq!(session.shutdown(), Ending::Shutdown);
}

#[test]
fn fixing_the_buffer_or_closing_it_publishes_an_empty_set() {
    let (session, document) = single_session();
    let bad = "{\n  \"license\": \"UNLICENSED\"\n}\n";

    session.open(&document, bad);
    assert_eq!(summary(&session.published().diagnostics).len(), 1);
    session.change(&document, "{\n  \"license\": \"MIT\"\n}\n");
    assert_eq!(summary(&session.published().diagnostics), Vec::new());

    session.change(&document, bad);
    assert_eq!(summary(&session.published().diagnostics).len(), 1);
    session.close(&document);
    let closed = session.published();
    assert_eq!(closed.uri.as_str(), uri(&document));
    assert_eq!(summary(&closed.diagnostics), Vec::new());

    assert_eq!(session.shutdown(), Ending::Shutdown);
}

#[test]
fn saving_publishes_the_buffer_and_documents_outside_the_root_get_an_empty_set() {
    let (session, document) = single_session();
    let bad = "{\n  \"license\": \"UNLICENSED\"\n}\n";
    let outside = fixture("lsp/empty/package.json");

    session.open(&document, bad);
    assert_eq!(summary(&session.published().diagnostics).len(), 1);
    session.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": uri(&document) } }),
    );
    assert_eq!(
        summary(&session.published().diagnostics),
        vec![("unlicensed".to_owned(), (1, 2, 1, 25))]
    );

    session.open(&outside, bad);
    session.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": "untitled:Untitled-1", "languageId": "json", "version": 1, "text": bad } }),
    );
    let mut published = session.published_for(&[&document, &outside]);
    while !published.contains_key("untitled:Untitled-1") {
        let next = session.published();
        published.insert(next.uri.as_str().to_owned(), next.diagnostics);
    }
    let counts: BTreeMap<String, usize> = published
        .into_iter()
        .map(|(uri, diagnostics)| (uri, diagnostics.len()))
        .collect();
    assert_eq!(
        counts,
        BTreeMap::from([
            (uri(&document), 1),
            (uri(&outside), 0),
            ("untitled:Untitled-1".to_owned(), 0),
        ])
    );

    assert_eq!(session.shutdown(), Ending::Shutdown);
}

#[test]
fn two_uris_for_one_file_both_get_its_diagnostics() {
    let (session, document) = single_session();
    let links = tempfile::tempdir().expect("temp dir");
    let link = links.path().join("single");
    std::os::unix::fs::symlink(fixture("lsp/single"), &link).expect("symlink the root");
    let alias = link.join("package.json");
    let bad = "{\n  \"license\": \"UNLICENSED\"\n}\n";

    session.open(&document, bad);
    session.open(&alias, bad);
    let mut published = session.published_for(&[&document, &alias]);
    while published.values().any(Vec::is_empty) {
        let next = session.published();
        published.insert(next.uri.as_str().to_owned(), next.diagnostics);
    }
    let counts: Vec<usize> = published.values().map(Vec::len).collect();
    assert_eq!(counts, vec![1, 1]);

    assert_eq!(session.shutdown(), Ending::Shutdown);
}

#[test]
fn a_bundled_multifile_rule_reports_in_the_open_buffer() {
    let session = Session::start(
        "../../../../packs/npm-provenance/fixtures/repository-url/bad/other-repository",
        &[],
        bundled_packs(),
    );
    assert_eq!(
        session
            .initialize(&["utf-8"])
            .err()
            .map(|error| error.message),
        None
    );
    let document = fixture(
        "../../../../packs/npm-provenance/fixtures/repository-url/bad/other-repository/packages/widget/package.json",
    );
    let text = fs::read_to_string(&document).expect("read the bad case");

    session.open(&document, &text);
    assert_eq!(
        summary(&session.published().diagnostics),
        vec![("repository-url-mismatch".to_owned(), (5, 4, 5, 50))]
    );

    session.change(
        &document,
        &text.replace("acme/other.git", "acme/widgets.git"),
    );
    assert_eq!(summary(&session.published().diagnostics), Vec::new());

    session.change(&document, &text.replace("\"repository\"", "\"homepage\""));
    assert_eq!(
        summary(&session.published().diagnostics),
        vec![("repository-url-mismatch".to_owned(), (0, 0, 0, 0))]
    );

    assert_eq!(session.shutdown(), Ending::Shutdown);
}

#[test]
fn columns_follow_the_negotiated_position_encoding() {
    let text = "{\n  \"café\": 1, \"license\": \"UNLICENSED\"\n}\n";
    for (offered, expected, encoding) in [
        (vec!["utf-8", "utf-16"], (1, 14, 1, 37), "utf-8"),
        (vec!["utf-16"], (1, 13, 1, 36), "utf-16"),
        (Vec::new(), (1, 13, 1, 36), "utf-16"),
    ] {
        let session = Session::start("lsp/single", &["license"], Vec::new());
        let response = session.initialize(&offered);
        assert_eq!(
            response
                .as_ref()
                .ok()
                .and_then(|result| result.pointer("/capabilities/positionEncoding"))
                .and_then(Value::as_str),
            Some(encoding)
        );
        let document = fixture("lsp/single/package.json");

        session.open(&document, text);
        assert_eq!(
            summary(&session.published().diagnostics),
            vec![("unlicensed".to_owned(), expected)],
            "{offered:?}"
        );
        assert_eq!(session.shutdown(), Ending::Shutdown);
    }
}

#[test]
fn a_root_check_refuses_fails_initialize_and_exits_2() {
    let session = Session::start("lsp/empty", &[], Vec::new());
    let error = session.initialize(&[]).expect_err("initialize is refused");
    assert_eq!(error.code, -32803);
    assert_eq!(
        error.message,
        format!(
            "no rules are enabled: `{}` enables no packs and no rule path was given",
            fixture("lsp/empty").join("gritlint.json").display()
        )
    );
    assert_eq!(session.exit(), Ending::Refused);
    assert_eq!(Ending::Refused.exit_code(), 2);

    let session = Session::start("lsp/broken", &[], Vec::new());
    let error = session.initialize(&[]).expect_err("initialize is refused");
    assert!(
        error.message.starts_with(&format!(
            "cannot parse config `{}`: ",
            fixture("lsp/broken").join("gritlint.json").display()
        )),
        "{}",
        error.message
    );
    assert_eq!(session.exit(), Ending::Refused);
}

#[test]
fn exit_without_shutdown_ends_with_exit_1() {
    let session = Session::start("lsp/single", &["license"], Vec::new());
    assert_eq!(
        session.initialize(&[]).err().map(|error| error.message),
        None
    );

    assert_eq!(session.exit(), Ending::Exit);
    assert_eq!(Ending::Exit.exit_code(), 1);
    assert_eq!(Ending::Shutdown.exit_code(), 0);
}

fn refusal_session() -> (Session, PathBuf, PathBuf) {
    let session = Session::start("lsp/refusal", &["license", "directory-read"], Vec::new());
    assert_eq!(
        session
            .initialize(&["utf-8"])
            .err()
            .map(|error| error.message),
        None
    );
    let first = fixture("lsp/refusal/a/package.json");
    let second = fixture("lsp/refusal/b/package.json");
    let text = "{\n  \"license\": \"UNLICENSED\"\n}\n";
    session.open(&first, text);
    session.open(&second, text);
    (session, first, second)
}

#[test]
fn a_closed_sibling_that_does_not_parse_is_named_on_the_open_documents_of_its_batch() {
    let (session, first, second) = refusal_session();

    let published = session.published_for(&[&first, &second]);
    let first_diagnostics = published.get(&uri(&first)).expect("a/package.json");
    assert_eq!(
        summary(first_diagnostics),
        vec![
            ("unlicensed".to_owned(), (1, 2, 1, 25)),
            ("target-does-not-parse".to_owned(), (0, 0, 0, 0)),
        ]
    );
    let refusal = first_diagnostics.get(1).expect("the sibling's refusal");
    assert!(
        refusal.message.starts_with("a/tsconfig.json:"),
        "{}",
        refusal.message
    );
    assert_eq!(
        summary(published.get(&uri(&second)).expect("b/package.json")),
        vec![("unlicensed".to_owned(), (1, 2, 1, 25))]
    );

    assert_eq!(session.shutdown(), Ending::Shutdown);
}

fn stated_position(message: &str) -> Vec<u32> {
    message
        .split(':')
        .skip(1)
        .take(2)
        .map(|number| number.parse::<u32>().expect("a line and column"))
        .collect()
}

#[test]
fn an_open_document_that_does_not_parse_gets_only_its_own_refusal() {
    let (session, first, second) = refusal_session();
    session.published_for(&[&first, &second]);

    session.change(&first, "{\n  \"license\": \n");
    let published = session.published_for(&[&first, &second]);
    let own = published.get(&uri(&first)).expect("a/package.json");
    assert_eq!(
        own.iter()
            .map(|diagnostic| diagnostic.message.split(':').next().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["a/package.json"]
    );
    let refusal = own.first().expect("its own refusal");
    assert_eq!(
        stated_position(&refusal.message),
        vec![
            refusal.range.start.line + 1,
            refusal.range.start.character + 1
        ]
    );
    assert_eq!(
        summary(published.get(&uri(&second)).expect("b/package.json")),
        vec![("unlicensed".to_owned(), (1, 2, 1, 25))]
    );

    assert_eq!(session.shutdown(), Ending::Shutdown);
}
