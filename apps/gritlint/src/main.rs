use clap::{Args, Parser, Subcommand, ValueEnum};
use gritlint_core::decode::{self, GritlintConfig};
use gritlint_core::domain::{CompiledRule, EmbeddedFile, Pack, ParsedRule, Verdict};
use gritlint_core::error::GritlintError;
use gritlint_core::workspace::WorkspaceInputs;
use gritlint_core::{decide, encode, export, lsp, shell};
use include_dir::{Dir, include_dir};
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

static PACKS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../packs");

#[derive(Debug, Parser)]
#[command(
    name = "gritlint",
    version,
    about = "Run GritQL rule packs over the config files of a repository.",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Scan a tree and report findings.")]
    Check(CheckArgs),
    #[command(about = "Run a pack's fixtures against its rules.")]
    Test(TestArgs),
    #[command(about = "Serve findings to an editor over the Language Server Protocol on stdio.")]
    Lsp(InputArgs),
    #[command(about = "Export rules for another linter.")]
    Export(ExportArgs),
    #[command(hide = true, about = "Print the gritlint.json JSON schema.")]
    Schema(SchemaArgs),
}

#[derive(Debug, Args)]
struct InputArgs {
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    #[arg(long = "rules", value_name = "PATH")]
    rules: Vec<PathBuf>,
    #[arg(default_value = ".")]
    root: PathBuf,
}

#[derive(Debug, Args)]
struct CheckArgs {
    #[command(flatten)]
    inputs: InputArgs,
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    #[value(help = "Every finding, its reason code and next action, and per-rule counts.")]
    Human,
    #[value(help = "The versioned report; `gritlint schema --report` prints its schema.")]
    Json,
    #[value(
        help = "At most 20 findings of at most 200 bytes a line, the count of the rest, and the verdict: at most 22 lines and 4,500 bytes."
    )]
    Concise,
    #[value(help = "One GitHub Actions `::error` annotation per finding.")]
    Github,
    #[value(help = "Markdown for `$GITHUB_STEP_SUMMARY`: the verdict and at most 100 findings.")]
    GithubSummary,
}

#[derive(Debug, Args)]
struct SchemaArgs {
    #[arg(
        long,
        help = "Print the `--format json` report schema instead of gritlint.json's."
    )]
    report: bool,
}

#[derive(Debug, Args)]
struct TestArgs {
    #[arg(default_value = ".")]
    dir: PathBuf,
}

#[derive(Debug, Args)]
struct ExportArgs {
    #[command(subcommand)]
    target: ExportTarget,
}

#[derive(Debug, Subcommand)]
enum ExportTarget {
    #[command(about = "Print a rule as a Biome GritQL plugin, or list which rules export.")]
    Biome(ExportBiomeArgs),
}

#[derive(Debug, Args)]
struct ExportBiomeArgs {
    #[command(flatten)]
    inputs: InputArgs,
    #[arg(
        long,
        value_name = "ID",
        help = "The rule to print; without it, list every enabled rule."
    )]
    rule: Option<String>,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_target(false)
        .without_time()
        .with_max_level(tracing::Level::WARN)
        .init();

    match Cli::parse().command {
        Command::Check(args) => report_exit(check(&args)),
        Command::Test(args) => report_exit(test(&args)),
        Command::Schema(args) => report_exit(schema(&args)),
        Command::Lsp(args) => report_exit(lsp(&args)),
        Command::Export(ExportArgs {
            target: ExportTarget::Biome(args),
        }) => report_exit(export_biome(&args)),
    }
}

fn report_exit(result: Result<ExitCode, GritlintError>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(error) => {
            tracing::error!("gritlint: {error}");
            ExitCode::from(decide::exit_code(Verdict::Refused))
        }
    }
}

fn lsp(args: &InputArgs) -> Result<ExitCode, GritlintError> {
    let packs = embedded_packs()?;
    let (connection, io_threads) = lsp_server::Connection::stdio();
    let ending = lsp::serve(&connection, &shell::Disk, &args.workspace(), &packs);
    drop(connection);
    let _ = io_threads.join();
    Ok(ExitCode::from(ending.exit_code()))
}

fn export_biome(args: &ExportBiomeArgs) -> Result<ExitCode, GritlintError> {
    let workspace = shell::load_workspace(&args.inputs.workspace(), &embedded_packs()?)?;
    let rules: Vec<&ParsedRule> = workspace.rules().iter().map(CompiledRule::rule).collect();
    let text = match &args.rule {
        None => export::listing(&rules),
        Some(id) => export::find(&rules, id)
            .and_then(|rule| export::biome(rule, &rules))
            .map_err(GritlintError::ExportRefused)?,
    };
    emit(&text)?;
    Ok(ExitCode::SUCCESS)
}

fn schema(args: &SchemaArgs) -> Result<ExitCode, GritlintError> {
    let schema = if args.report {
        encode::report_schema()
    } else {
        GritlintConfig::schema_for_packs(&embedded_packs()?)
    };
    let text = serde_json::to_string_pretty(&schema).map_err(|error| GritlintError::Render {
        message: error.to_string(),
    })?;
    emit(&format!("{text}\n"))?;
    Ok(ExitCode::SUCCESS)
}

fn check(args: &CheckArgs) -> Result<ExitCode, GritlintError> {
    let workspace = shell::load_workspace(&args.inputs.workspace(), &embedded_packs()?)?;
    let files = shell::select(&workspace)?;
    let plan = decide::plan(workspace.rules(), &files);
    let sources = shell::read_sources(workspace.root(), &decide::sources_needed(&plan))?;
    let outcome = decide::scan(&plan, &sources)?;

    let rendered = match args.format {
        Format::Human => encode::render_human(&outcome),
        Format::Json => encode::render_json(&outcome).map_err(|error| GritlintError::Render {
            message: error.to_string(),
        })?,
        Format::Concise => encode::render_concise(&outcome),
        Format::Github => encode::render_github(&outcome),
        Format::GithubSummary => encode::render_github_summary(&outcome),
    };
    emit(&rendered)?;
    Ok(ExitCode::from(decide::exit_code(outcome.verdict())))
}

fn test(args: &TestArgs) -> Result<ExitCode, GritlintError> {
    let tree = shell::read_tree(&args.dir)?;
    let layout = decode::fixture_layout(&tree)?;
    let outcome = decide::run_fixtures(&layout)?;
    emit(&encode::render_fixtures(&outcome))?;
    Ok(ExitCode::from(decide::exit_code(outcome.verdict())))
}

fn emit(text: &str) -> Result<(), GritlintError> {
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
        .map_err(|source| GritlintError::Io {
            path: PathBuf::from("<stdout>"),
            source,
        })
}

fn embedded_packs() -> Result<Vec<Pack>, GritlintError> {
    let mut files = Vec::new();
    collect_embedded(&PACKS, &mut files);
    decode::embedded_packs(&files)
}

fn collect_embedded(dir: &Dir<'_>, files: &mut Vec<EmbeddedFile>) {
    files.extend(dir.files().map(|file| {
        EmbeddedFile::new(
            file.path().to_string_lossy().replace('\\', "/"),
            file.contents_utf8()
                .map_or_else(String::new, ToOwned::to_owned),
        )
    }));
    for sub in dir.dirs() {
        collect_embedded(sub, files);
    }
}

impl InputArgs {
    fn workspace(&self) -> WorkspaceInputs {
        WorkspaceInputs {
            config: self.config.clone(),
            rules: self.rules.clone(),
            root: self.root.clone(),
        }
    }
}
