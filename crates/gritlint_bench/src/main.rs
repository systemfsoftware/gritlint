mod breakdown;
mod deltas;

use gritlint_core::domain::{EmbeddedFile, RelPath};
use gritlint_core::shell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

struct Args {
    baseline: String,
    candidate: String,
    corpus: String,
    config: String,
    packs: String,
    expected_deltas: String,
    rounds: usize,
}

#[derive(Clone, Copy)]
enum Arm {
    Baseline,
    Candidate,
    Serial,
}

impl Arm {
    const ALL: [Arm; 3] = [Arm::Baseline, Arm::Candidate, Arm::Serial];

    fn label(self) -> &'static str {
        match self {
            Arm::Baseline => "baseline",
            Arm::Candidate => "candidate",
            Arm::Serial => "candidate, `RAYON_NUM_THREADS=1`",
        }
    }
}

struct Report {
    stdout: Vec<u8>,
    code: Option<i32>,
}

#[derive(Default)]
struct Walls {
    baseline: Vec<Duration>,
    candidate: Vec<Duration>,
    serial: Vec<Duration>,
}

impl Walls {
    fn of(&mut self, arm: Arm) -> &mut Vec<Duration> {
        match arm {
            Arm::Baseline => &mut self.baseline,
            Arm::Candidate => &mut self.candidate,
            Arm::Serial => &mut self.serial,
        }
    }
}

fn main() -> ExitCode {
    let (mut sink, text, code): (Box<dyn std::io::Write>, String, ExitCode) =
        match parse(std::env::args().skip(1)).and_then(|args| bench(&args)) {
            Ok(summary) => (Box::new(std::io::stdout()), summary, ExitCode::SUCCESS),
            Err(message) => (
                Box::new(std::io::stderr()),
                format!("gritlint-bench: {message}\n"),
                ExitCode::FAILURE,
            ),
        };
    match sink.write_all(text.as_bytes()).and_then(|()| sink.flush()) {
        Ok(()) => code,
        Err(_) => ExitCode::FAILURE,
    }
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let (mut baseline, mut candidate, mut corpus, mut config, mut packs) =
        (None, None, None, None, None);
    let (mut expected_deltas, mut rounds) = (None, None);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--baseline" => baseline = Some(value),
            "--candidate" => candidate = Some(value),
            "--corpus" => corpus = Some(value),
            "--config" => config = Some(value),
            "--packs" => packs = Some(value),
            "--expected-deltas" => expected_deltas = Some(value),
            "--rounds" => rounds = Some(value),
            _ => return Err(format!("unknown argument `{flag}`")),
        }
    }
    let rounds = rounds
        .ok_or("missing `--rounds`")?
        .parse::<usize>()
        .ok()
        .filter(|rounds| *rounds > 0)
        .ok_or("`--rounds` must be a positive integer")?;
    Ok(Args {
        baseline: baseline.ok_or("missing `--baseline`")?,
        candidate: candidate.ok_or("missing `--candidate`")?,
        corpus: corpus.ok_or("missing `--corpus`")?,
        config: config.ok_or("missing `--config`")?,
        packs: packs.ok_or("missing `--packs`")?,
        expected_deltas: expected_deltas.ok_or("missing `--expected-deltas`")?,
        rounds,
    })
}

fn bench(args: &Args) -> Result<String, String> {
    let (baseline, _) = run(args, Arm::Baseline)?;
    if baseline.code != Some(1) {
        return Err(format!(
            "the baseline exited {:?}, not 1 with findings, so the corpus or config is broken and a comparison would prove nothing",
            baseline.code
        ));
    }
    let (candidate, _) = run(args, Arm::Candidate)?;
    if candidate.code != baseline.code {
        return Err(format!(
            "the candidate exited {:?}; the baseline exited {:?}",
            candidate.code, baseline.code
        ));
    }
    let packs = packs(Path::new(&args.packs))?;
    let deltas = shell::read_text(Path::new(&args.expected_deltas))
        .map_err(|error| error.to_string())
        .and_then(|text| deltas::parse(&text))?;
    let pack_files: Vec<&str> = packs.iter().map(EmbeddedFile::path).collect();
    deltas::check_cases(&deltas, &pack_files)?;
    deltas::compare(&tally(&baseline)?, &tally(&candidate)?, &deltas)?;
    let mut walls = Walls::default();
    for round in 0..args.rounds {
        let mut order = Arm::ALL;
        if round % 2 == 1 {
            order.reverse();
        }
        for arm in order {
            let (report, wall) = run(args, arm)?;
            let reference = match arm {
                Arm::Baseline => &baseline,
                Arm::Candidate | Arm::Serial => &candidate,
            };
            if report.code != reference.code || report.stdout != reference.stdout {
                return Err(format!(
                    "round {}: the {} report differs from its first run (exit {:?} against {:?}, {} stdout bytes against {})",
                    round + 1,
                    arm.label(),
                    report.code,
                    reference.code,
                    report.stdout.len(),
                    reference.stdout.len()
                ));
            }
            walls.of(arm).push(wall);
        }
    }
    let phases = phases(args, &packs, &candidate)?;
    Ok(summary(args.rounds, &mut walls) + &delta_table(&deltas) + &phases)
}

/// Findings per rule id from `--format json`: PR1 names the rule `rule`, the
/// versioned envelope `ruleId`. Messages and locations may differ by design.
fn tally(report: &Report) -> Result<BTreeMap<String, usize>, String> {
    let json: serde_json::Value = serde_json::from_slice(&report.stdout)
        .map_err(|error| format!("a report is not JSON: {error}"))?;
    json.get("findings")
        .and_then(serde_json::Value::as_array)
        .ok_or("a report has no findings array")?
        .iter()
        .map(|finding| {
            finding
                .get("ruleId")
                .or_else(|| finding.get("rule"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("a finding names no rule: {finding}"))
        })
        .try_fold(BTreeMap::new(), |mut counts, rule| {
            *counts.entry(rule?).or_insert(0) += 1;
            Ok(counts)
        })
}

fn phases(args: &Args, packs: &[EmbeddedFile], reference: &Report) -> Result<String, String> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .map_err(|error| format!("cannot build a one-thread pool: {error}"))?;
    let runs = (0..args.rounds)
        .map(|round| {
            let run = pool.install(|| {
                breakdown::run(Path::new(&args.config), Path::new(&args.corpus), packs)
            })?;
            if Some(i32::from(run.code)) == reference.code && run.stdout == reference.stdout {
                Ok(run)
            } else {
                Err(format!(
                    "breakdown round {}: the in-process report differs from the candidate's (exit {} against {:?}, {} stdout bytes against {})",
                    round + 1,
                    run.code,
                    reference.code,
                    run.stdout.len(),
                    reference.stdout.len()
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(phase_table(args.rounds, &runs))
}

fn packs(root: &Path) -> Result<Vec<EmbeddedFile>, String> {
    let tree = shell::read_tree(root).map_err(|error| error.to_string())?;
    tree.files()
        .iter()
        .map(|file| {
            let path = RelPath::from_root(root, file.path())
                .map_err(|error| format!("{}: {error:?}", file.path().display()))?;
            Ok(EmbeddedFile::new(
                path.as_str().to_owned(),
                file.contents().to_owned(),
            ))
        })
        .collect()
}

fn run(args: &Args, arm: Arm) -> Result<(Report, Duration), String> {
    let binary = match arm {
        Arm::Baseline => &args.baseline,
        Arm::Candidate | Arm::Serial => &args.candidate,
    };
    let mut command = Command::new(binary);
    command.args([
        "check",
        "--config",
        &args.config,
        "--format",
        "json",
        &args.corpus,
    ]);
    if matches!(arm, Arm::Serial) {
        command.env("RAYON_NUM_THREADS", "1");
    }
    let start = Instant::now();
    let output = command
        .output()
        .map_err(|error| format!("cannot run `{binary}`: {error}"))?;
    let wall = start.elapsed();
    Ok((
        Report {
            stdout: output.stdout,
            code: output.status.code(),
        },
        wall,
    ))
}

fn median(walls: &mut [Duration]) -> Duration {
    walls.sort_unstable();
    walls.get(walls.len() / 2).copied().unwrap_or_default()
}

fn millis(wall: Duration) -> f64 {
    wall.as_secs_f64() * 1000.0
}

fn summary(rounds: usize, walls: &mut Walls) -> String {
    let [baseline, candidate, serial] = Arm::ALL.map(|arm| median(walls.of(arm)));
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let mut out = String::from("| Binary | Median wall |\n| --- | --- |\n");
    for (arm, wall) in Arm::ALL.into_iter().zip([baseline, candidate, serial]) {
        let _ = writeln!(out, "| {} | {:.0} ms |", arm.label(), millis(wall));
    }
    let _ = writeln!(
        out,
        "\nSpeedup {:.1}x (single-threaded {:.1}x) over {rounds} rounds on {cores} cores. The candidate's reports are byte-identical to each other, and its exit code equals the baseline's. Its findings per rule equal the baseline's, except the expected deltas below.",
        baseline.as_secs_f64() / candidate.as_secs_f64(),
        baseline.as_secs_f64() / serial.as_secs_f64(),
    );
    out
}

fn delta_table(deltas: &BTreeMap<String, deltas::Delta>) -> String {
    let mut out = String::from(
        "\n| Rule | Baseline | Candidate | Reason | Case |\n| --- | --- | --- | --- | --- |\n",
    );
    for (rule, delta) in deltas {
        let _ = writeln!(
            out,
            "| `{rule}` | {} | {} | {} | `{}` |",
            delta.baseline, delta.expected, delta.reason, delta.case
        );
    }
    out
}

fn phase_table(rounds: usize, runs: &[breakdown::Run]) -> String {
    let mut columns: Vec<Vec<Duration>> = vec![Vec::new(); breakdown::PHASES.len()];
    let mut totals = Vec::new();
    for run in runs {
        for (column, wall) in columns.iter_mut().zip(run.walls) {
            column.push(wall);
        }
        totals.push(run.total);
    }
    let total = median(&mut totals);
    let mut out = format!(
        "\n#### Where one thread spends its time\n\nThe candidate's pipeline in process on one thread, median of {rounds} rounds, each report byte-identical to the candidate binary's.\n\n| Phase | Median wall | Share |\n| --- | --- | --- |\n"
    );
    for (phase, column) in breakdown::PHASES.into_iter().zip(&mut columns) {
        let wall = median(column);
        let _ = writeln!(
            out,
            "| {phase} | {:.1} ms | {:.1}% |",
            millis(wall),
            100.0 * wall.as_secs_f64() / total.as_secs_f64()
        );
    }
    let _ = writeln!(out, "| total | {:.1} ms | 100% |", millis(total));
    if let Some(run) = runs.first() {
        let counts = &run.counts;
        let _ = writeln!(
            out,
            "\nFiles: {} walked, {} selected, {} read. Rules: {}. Executions: {} over {} (rule, file) pairs. Parses: {} of {} distinct files, at most {} per file.\n\nParse is timed by parsing each file an execution reads with the engine's parser; match is the scan's wall time minus parse. Locate is the locator pass after the scan, its own parses included. A multifile rule loads only the files its pattern names, so parse and its counts are upper bounds and match a lower bound.",
            counts.walked,
            counts.selected,
            counts.read,
            counts.rules,
            counts.executions,
            counts.pairs,
            counts.parses,
            counts.parsed_files,
            counts.most_parses,
        );
    }
    out
}
