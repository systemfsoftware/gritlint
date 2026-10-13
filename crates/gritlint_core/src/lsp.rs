use crate::domain::{Finding, Outcome, Pack, RelPath, SelectedFiles, Sources};
use crate::error::{GritlintError, TargetReason};
use crate::workspace::{Workspace, WorkspaceInputs};
use crate::{decide, encode};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit,
    LogMessage, Notification as _, PublishDiagnostics,
};
use lsp_types::request::{Request as _, Shutdown};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, InitializeResult, LogMessageParams, MessageType,
    NumberOrString, Position, PositionEncodingKind, PublishDiagnosticsParams, Range,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Uri,
};
use serde::de::DeserializeOwned;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const INVALID_REQUEST: i32 = -32600;
const SERVER_NOT_INITIALIZED: i32 = -32002;
const METHOD_NOT_FOUND: i32 = -32601;
const REQUEST_FAILED: i32 = -32803;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    Shutdown,
    Exit,
    Refused,
}

impl Ending {
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Shutdown => 0,
            Self::Exit => 1,
            Self::Refused => 2,
        }
    }
}

pub trait Files {
    fn load(&self, inputs: &WorkspaceInputs, packs: &[Pack]) -> Result<Workspace, GritlintError>;
    fn select(&self, workspace: &Workspace) -> Result<SelectedFiles, GritlintError>;
    fn read(&self, root: &Path, paths: &[RelPath]) -> Result<Sources, GritlintError>;
    fn absolute(&self, path: &Path) -> Result<PathBuf, GritlintError>;
}

#[must_use]
pub fn serve(
    connection: &Connection,
    files: &dyn Files,
    inputs: &WorkspaceInputs,
    packs: &[Pack],
) -> Ending {
    let Ok((id, params)) = connection.initialize_start() else {
        return Ending::Exit;
    };
    let encoding = Encoding::negotiate(params);
    let loaded = files.load(inputs, packs).map(|workspace| {
        let root = files
            .absolute(workspace.root())
            .unwrap_or_else(|_| workspace.root().to_owned());
        (workspace, root)
    });
    match loaded {
        Err(error) => {
            refuse(connection, id, &error);
            Ending::Refused
        }
        Ok((workspace, root)) => {
            if connection
                .initialize_finish(id, initialize_result(encoding))
                .is_err()
            {
                return Ending::Exit;
            }
            let server = Server {
                connection,
                files,
                workspace,
                root,
                encoding,
                documents: BTreeMap::new(),
                edits: 0,
                closed: Cell::new(false),
            };
            server.announce();
            server.run()
        }
    }
}

fn initialize_result(encoding: Encoding) -> serde_json::Value {
    let result = InitializeResult {
        capabilities: ServerCapabilities {
            position_encoding: Some(encoding.kind()),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::FULL),
                    save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                    ..TextDocumentSyncOptions::default()
                },
            )),
            ..ServerCapabilities::default()
        },
        server_info: Some(ServerInfo {
            name: "gritlint".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    };
    serde_json::to_value(result).unwrap_or_default()
}

fn refuse(connection: &Connection, id: RequestId, error: &GritlintError) {
    send(
        connection,
        Response::new_err(id, REQUEST_FAILED, error.to_string()).into(),
    );
    until_exit(
        connection,
        SERVER_NOT_INITIALIZED,
        "gritlint refused to initialize",
    );
}

fn until_exit(connection: &Connection, code: i32, message: &str) -> bool {
    for incoming in &connection.receiver {
        match incoming {
            Message::Request(request) => {
                send(
                    connection,
                    Response::new_err(request.id, code, message.to_owned()).into(),
                );
            }
            Message::Notification(notification) if notification.method == Exit::METHOD => {
                return true;
            }
            Message::Notification(_) | Message::Response(_) => {}
        }
    }
    false
}

fn send(connection: &Connection, message: Message) -> bool {
    connection.sender.send(message).is_ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Utf8,
    Utf16,
}

impl Encoding {
    fn negotiate(params: serde_json::Value) -> Self {
        let offered = serde_json::from_value::<InitializeParams>(params)
            .ok()
            .and_then(|params| params.capabilities.general)
            .and_then(|general| general.position_encodings)
            .unwrap_or_default();
        if offered.contains(&PositionEncodingKind::UTF8) {
            Self::Utf8
        } else {
            Self::Utf16
        }
    }

    fn kind(self) -> PositionEncodingKind {
        match self {
            Self::Utf8 => PositionEncodingKind::UTF8,
            Self::Utf16 => PositionEncodingKind::UTF16,
        }
    }

    fn position(self, lines: &[&str], line: u32, column: u32) -> Position {
        let line = line.saturating_sub(1);
        let byte = column.saturating_sub(1);
        let character = match self {
            Self::Utf8 => byte,
            Self::Utf16 => utf16_column(lines, line, byte),
        };
        Position::new(line, character)
    }
}

fn utf16_column(lines: &[&str], line: u32, byte: u32) -> u32 {
    let line_text = usize::try_from(line)
        .ok()
        .and_then(|line| lines.get(line))
        .copied()
        .unwrap_or_default();
    let byte = usize::try_from(byte).unwrap_or(usize::MAX);
    let units: usize = line_text
        .char_indices()
        .take_while(|(index, _)| *index < byte)
        .map(|(_, character)| character.len_utf16())
        .sum();
    u32::try_from(units).unwrap_or(u32::MAX)
}
struct Document {
    text: String,
    path: Option<RelPath>,
    edited: u64,
}

struct Refusal {
    file: RelPath,
    reason: TargetReason,
    message: String,
    batch_mates: BTreeSet<RelPath>,
}

enum Handled {
    Nothing,
    Changed,
    End(Ending),
}

struct Server<'a> {
    connection: &'a Connection,
    files: &'a dyn Files,
    workspace: Workspace,
    root: PathBuf,
    encoding: Encoding,
    documents: BTreeMap<Uri, Document>,
    edits: u64,
    closed: Cell<bool>,
}

impl Server<'_> {
    fn run(mut self) -> Ending {
        while let Ok(first) = self.connection.receiver.recv() {
            let mut changed = false;
            let mut next = Some(first);
            while let Some(message) = next {
                match self.handle(message) {
                    Handled::Nothing => {}
                    Handled::Changed => changed = true,
                    Handled::End(ending) => return ending,
                }
                next = self.connection.receiver.try_recv().ok();
            }
            if changed {
                self.publish();
            }
            if self.closed.get() {
                return Ending::Exit;
            }
        }
        Ending::Exit
    }

    fn handle(&mut self, message: Message) -> Handled {
        match message {
            Message::Request(request) => self.request(&request),
            Message::Notification(notification) => self.notification(notification),
            Message::Response(_) => Handled::Nothing,
        }
    }

    fn request(&self, request: &Request) -> Handled {
        if request.method == Shutdown::METHOD {
            self.send(Response::new_ok(request.id.clone(), ()).into());
            let exited = until_exit(
                self.connection,
                INVALID_REQUEST,
                "gritlint is shutting down",
            );
            return Handled::End(if exited {
                Ending::Shutdown
            } else {
                Ending::Exit
            });
        }
        self.send(
            Response::new_err(
                request.id.clone(),
                METHOD_NOT_FOUND,
                format!("gritlint does not serve `{}`", request.method),
            )
            .into(),
        );
        Handled::Nothing
    }

    fn send(&self, message: Message) {
        if !send(self.connection, message) {
            self.closed.set(true);
        }
    }

    fn notify<N: lsp_types::notification::Notification>(&self, params: N::Params) {
        self.send(Notification::new(N::METHOD.to_owned(), params).into());
    }

    fn notification(&mut self, notification: Notification) -> Handled {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => params::<DidOpenTextDocumentParams>(notification)
                .map_or(Handled::Nothing, |open| {
                    self.open(open.text_document.uri, open.text_document.text)
                }),
            DidChangeTextDocument::METHOD => params::<DidChangeTextDocumentParams>(notification)
                .and_then(|change| {
                    let text = change.content_changes.into_iter().last()?.text;
                    Some(self.open(change.text_document.uri, text))
                })
                .unwrap_or(Handled::Nothing),
            DidSaveTextDocument::METHOD => Handled::Changed,
            DidCloseTextDocument::METHOD => params::<DidCloseTextDocumentParams>(notification)
                .map_or(Handled::Nothing, |close| {
                    self.close(close.text_document.uri)
                }),
            Exit::METHOD => Handled::End(Ending::Exit),
            _ => Handled::Nothing,
        }
    }

    fn open(&mut self, uri: Uri, text: String) -> Handled {
        let path = match self.documents.get(&uri) {
            Some(document) => document.path.clone(),
            None => self.relative(&uri),
        };
        self.edits += 1;
        let edited = self.edits;
        self.documents.insert(uri, Document { text, path, edited });
        Handled::Changed
    }

    fn close(&mut self, uri: Uri) -> Handled {
        self.documents.remove(&uri);
        self.send_diagnostics(uri, Vec::new());
        Handled::Changed
    }

    fn relative(&self, uri: &Uri) -> Option<RelPath> {
        if uri.scheme().is_none_or(|scheme| scheme.as_str() != "file") {
            return None;
        }
        let decoded = uri.path().as_estr().decode().into_string().ok()?;
        let path = std::path::Path::new(decoded.as_ref());
        RelPath::from_root(&self.root, path)
            .ok()
            .or_else(|| RelPath::from_root(&self.root, &self.files.absolute(path).ok()?).ok())
    }

    fn publish(&self) {
        let diagnostics = match self.diagnose() {
            Ok(diagnostics) => diagnostics,
            Err(error) => {
                self.log(&error);
                return;
            }
        };
        for (uri, document) in &self.documents {
            let found = document
                .path
                .as_ref()
                .and_then(|path| diagnostics.get(path).cloned())
                .unwrap_or_default();
            self.send_diagnostics(uri.clone(), found);
        }
    }

    fn diagnose(&self) -> Result<BTreeMap<RelPath, Vec<Diagnostic>>, GritlintError> {
        let mut by_edit: Vec<&Document> = self.documents.values().collect();
        by_edit.sort_by_key(|document| document.edited);
        let open: BTreeMap<RelPath, &Document> = by_edit
            .into_iter()
            .filter_map(|document| document.path.clone().map(|path| (path, document)))
            .collect();
        let files = match self.files.select(&self.workspace) {
            Ok(files) => files,
            Err(GritlintError::NoFilesSelected { .. }) => return Ok(BTreeMap::new()),
            Err(error) => return Err(error),
        };
        let selected = selected_open(&files, &open);
        let plan = decide::restrict(decide::plan(self.workspace.rules(), &files), &selected);
        let (outcome, refusals) = self.scan(plan, &open)?;
        Ok(selected
            .iter()
            .filter_map(|path| open.get(path).map(|document| (path, *document)))
            .map(|(path, document)| {
                let diagnostics = self.diagnostics(path, document, &outcome, &refusals);
                (path.clone(), diagnostics)
            })
            .collect())
    }

    fn scan(
        &self,
        mut plan: crate::domain::ScanPlan,
        open: &BTreeMap<RelPath, &Document>,
    ) -> Result<(Outcome, Vec<Refusal>), GritlintError> {
        let mut refusals = Vec::new();
        let mut needed = decide::sources_needed(&plan);
        let sources = loop {
            let error = match self.sources(&needed, open) {
                Ok(sources) => break sources,
                Err(error) => error,
            };
            (plan, needed) = drop_refused_target(plan, &needed, error, &mut refusals)?;
        };
        loop {
            let error = match decide::scan(&plan, &sources) {
                Ok(outcome) => return Ok((outcome, refusals)),
                Err(error) => error,
            };
            (plan, needed) = drop_refused_target(plan, &needed, error, &mut refusals)?;
        }
    }

    fn sources(
        &self,
        needed: &[RelPath],
        open: &BTreeMap<RelPath, &Document>,
    ) -> Result<Sources, GritlintError> {
        let (overlay, disk): (Vec<&RelPath>, Vec<&RelPath>) =
            needed.iter().partition(|path| open.contains_key(*path));
        let disk: Vec<RelPath> = disk.into_iter().cloned().collect();
        let read = self.files.read(&self.root, &disk).map_err(|error| {
            if let GritlintError::Io { path, source } = &error
                && let Ok(file) = RelPath::from_root(&self.root, path)
            {
                return GritlintError::TargetUnreadable {
                    file: file.as_str().to_owned(),
                    message: source.to_string(),
                };
            }
            error
        })?;
        Ok(read.overlaid(overlay.into_iter().filter_map(|path| {
            open.get(path)
                .map(|document| (path.clone(), document.text.clone()))
        })))
    }

    fn diagnostics(
        &self,
        path: &RelPath,
        document: &Document,
        outcome: &Outcome,
        refusals: &[Refusal],
    ) -> Vec<Diagnostic> {
        let own: Vec<&Refusal> = refusals
            .iter()
            .filter(|refusal| refusal.file == *path)
            .collect();
        let related: Vec<&Refusal> = if own.is_empty() {
            refusals
                .iter()
                .filter(|refusal| refusal.batch_mates.contains(path))
                .collect()
        } else {
            own
        };
        let lines: Vec<&str> = document.text.split('\n').collect();
        outcome
            .findings()
            .iter()
            .filter(|finding| finding.location().path() == path)
            .map(|finding| self.finding(finding, &lines))
            .chain(
                related
                    .into_iter()
                    .map(|refusal| self.refusal(refusal, path, &lines)),
            )
            .collect()
    }

    fn finding(&self, finding: &Finding, lines: &[&str]) -> Diagnostic {
        let range = finding
            .location()
            .range()
            .map_or_else(Range::default, |range| {
                Range::new(
                    self.encoding
                        .position(lines, range.start.line, range.start.column),
                    self.encoding
                        .position(lines, range.end.line, range.end.column),
                )
            });
        diagnostic(
            range,
            finding.guidance().reason_code().as_str(),
            encode::message_and_next_action(finding),
        )
    }

    fn refusal(&self, refusal: &Refusal, path: &RelPath, lines: &[&str]) -> Diagnostic {
        let start = match refusal.reason.position() {
            Some((line, column)) if refusal.file == *path => {
                self.encoding.position(lines, line, column)
            }
            Some(_) | None => Position::default(),
        };
        diagnostic(
            Range::new(start, start),
            refusal.reason.code(),
            refusal.message.clone(),
        )
    }

    fn send_diagnostics(&self, uri: Uri, diagnostics: Vec<Diagnostic>) {
        self.notify::<PublishDiagnostics>(PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        });
    }

    fn announce(&self) {
        self.notify::<LogMessage>(LogMessageParams {
            typ: MessageType::INFO,
            message: format!(
                "gritlint: serving {}; documents outside it get no diagnostics",
                self.root.display()
            ),
        });
    }

    fn log(&self, error: &GritlintError) {
        self.notify::<LogMessage>(LogMessageParams {
            typ: MessageType::ERROR,
            message: format!("gritlint: {error}"),
        });
    }
}

fn selected_open(files: &SelectedFiles, open: &BTreeMap<RelPath, &Document>) -> BTreeSet<RelPath> {
    files
        .language_keys()
        .flat_map(|key| files.batch(key))
        .map(crate::domain::SelectedFile::path)
        .filter(|path| open.contains_key(*path))
        .cloned()
        .collect()
}

fn drop_refused_target(
    plan: crate::domain::ScanPlan,
    needed: &[RelPath],
    error: GritlintError,
    refusals: &mut Vec<Refusal>,
) -> Result<(crate::domain::ScanPlan, Vec<RelPath>), GritlintError> {
    let Some(refusal) = refusal_of(&error, &plan) else {
        return Err(error);
    };
    let narrowed = decide::without(plan, &refusal.file);
    let narrowed_needed = decide::sources_needed(&narrowed);
    if narrowed_needed == needed {
        return Err(error);
    }
    refusals.push(refusal);
    Ok((narrowed, narrowed_needed))
}

fn refusal_of(error: &GritlintError, plan: &crate::domain::ScanPlan) -> Option<Refusal> {
    let target = error.target()?;
    let file = RelPath::new(target.file()).ok()?;
    let batch_mates = decide::batch_mates(plan, &file);
    Some(Refusal {
        file,
        reason: target.reason(),
        message: error.to_string(),
        batch_mates,
    })
}

fn diagnostic(range: Range, code: &str, message: String) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(code.to_owned())),
        source: Some("gritlint".to_owned()),
        message,
        ..Diagnostic::default()
    }
}

fn params<P: DeserializeOwned>(notification: Notification) -> Option<P> {
    serde_json::from_value(notification.params).ok()
}
