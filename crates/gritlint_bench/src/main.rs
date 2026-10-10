use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

struct Args {
    baseline: String,
    candidate: String,
    corpus: String,
    config: String,
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
    let (mut baseline, mut candidate, mut corpus, mut config, mut rounds) =
        (None, None, None, None, None);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--baseline" => baseline = Some(value),
            "--candidate" => candidate = Some(value),
            "--corpus" => corpus = Some(value),
            "--config" => config = Some(value),
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
        rounds,
    })
}

fn bench(args: &Args) -> Result<String, String> {
    let (reference, _) = run(args, Arm::Baseline)?;
    if !matches!(reference.code, Some(0 | 1)) {
        return Err(format!(
            "the baseline exited {:?}, so the corpus or config is broken and a comparison would prove nothing",
            reference.code
        ));
    }
    let mut walls = Walls::default();
    for round in 0..args.rounds {
        let mut order = Arm::ALL;
        if round % 2 == 1 {
            order.reverse();
        }
        for arm in order {
            let (report, wall) = run(args, arm)?;
            if report.code != reference.code || report.stdout != reference.stdout {
                return Err(format!(
                    "round {}: the {} report differs from the baseline's (exit {:?} against {:?}, {} stdout bytes against {})",
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
    Ok(summary(args.rounds, &mut walls))
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
        "\nSpeedup {:.1}x (single-threaded {:.1}x) over {rounds} rounds on {cores} cores; all three reports byte-identical.",
        baseline.as_secs_f64() / candidate.as_secs_f64(),
        baseline.as_secs_f64() / serial.as_secs_f64(),
    );
    out
}
