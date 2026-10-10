use gritlint_core::decode::{self, GritlintConfig};
use gritlint_core::domain::{EmbeddedFile, Execution, RelPath};
use gritlint_core::{decide, encode, engine, shell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{Duration, Instant};

pub const PHASES: [&str; 9] = [
    "config decode",
    "rule compile",
    "file walk",
    "plan",
    "file read",
    "execution: parse",
    "execution: match",
    "execution: locate",
    "report output",
];

pub struct Run {
    pub walls: [Duration; PHASES.len()],
    pub total: Duration,
    pub counts: Counts,
    pub stdout: Vec<u8>,
    pub code: u8,
}

pub struct Counts {
    pub walked: usize,
    pub selected: usize,
    pub read: usize,
    pub rules: usize,
    pub executions: usize,
    pub pairs: usize,
    pub parses: usize,
    pub parsed_files: usize,
    pub most_parses: usize,
}

struct Census {
    wall: Duration,
    executions: usize,
    pairs: usize,
    parses: usize,
    per_file: BTreeMap<String, usize>,
}

pub fn run(config_path: &Path, corpus: &Path, packs: &[EmbeddedFile]) -> Result<Run, String> {
    let clock = Instant::now();
    let text = shell::read_text(config_path).map_err(message)?;
    let config = GritlintConfig::parse(&text).map_err(message)?;
    let packs = decode::embedded_packs(packs).map_err(message)?;
    let enabled = decode::enabled_packs(&config, config_path).map_err(message)?;
    let ignore = decode::ignore_globs(&config.ignore).map_err(message)?;
    let decode_wall = clock.elapsed();

    let clock = Instant::now();
    let base = config_path.parent().unwrap_or_else(|| Path::new("."));
    let consumer: Vec<_> = config.rules.iter().map(|path| base.join(path)).collect();
    let consumer = shell::read_rule_sources(&consumer).map_err(message)?;
    let rules = decode::load_rules(&packs, &enabled, consumer, config_path).map_err(message)?;
    let compile_wall = clock.elapsed();

    let clock = Instant::now();
    let walked = shell::walk(corpus, &ignore);
    let walk_wall = clock.elapsed();
    let walked_count = walked.len();

    let clock = Instant::now();
    let files = decode::select_files(walked, corpus).map_err(message)?;
    let plan = decide::plan(&rules, &files);
    let needed = decide::sources_needed(&plan);
    let plan_wall = clock.elapsed();

    let clock = Instant::now();
    let sources = shell::read_sources(corpus, &needed).map_err(message)?;
    let read_wall = clock.elapsed();

    let clock = Instant::now();
    let matched = decide::match_rules(&plan, &sources).map_err(message)?;
    let scan_wall = clock.elapsed();

    let clock = Instant::now();
    let outcome = decide::locate(matched, &sources).map_err(message)?;
    let locate_wall = clock.elapsed();

    let clock = Instant::now();
    let rendered = encode::render_json(&outcome).map_err(message)?;
    let render_wall = clock.elapsed();

    let census = census(&decide::executions(&plan, &sources).map_err(message)?)?;
    let read: BTreeSet<&str> = needed.iter().map(RelPath::as_str).collect();
    if !census
        .per_file
        .keys()
        .map(String::as_str)
        .eq(read.iter().copied())
    {
        return Err(format!(
            "the executions read {} distinct files, but the plan read {}",
            census.per_file.len(),
            read.len()
        ));
    }
    let Some(match_wall) = scan_wall.checked_sub(census.wall) else {
        return Err(format!(
            "parsing alone took {:?}, longer than the whole scan's {scan_wall:?}, so the parse and match split is meaningless",
            census.wall
        ));
    };
    Ok(Run {
        walls: [
            decode_wall,
            compile_wall,
            walk_wall,
            plan_wall,
            read_wall,
            census.wall,
            match_wall,
            locate_wall,
            render_wall,
        ],
        total: decode_wall
            + compile_wall
            + walk_wall
            + plan_wall
            + read_wall
            + scan_wall
            + locate_wall
            + render_wall,
        counts: Counts {
            walked: walked_count,
            selected: plan.files_selected(),
            read: needed.len(),
            rules: plan.rule_count(),
            executions: census.executions,
            pairs: census.pairs,
            parses: census.parses,
            parsed_files: census.per_file.len(),
            most_parses: census.per_file.values().copied().max().unwrap_or(0),
        },
        stdout: rendered.into_bytes(),
        code: decide::exit_code(outcome.verdict()),
    })
}

fn census(executions: &[Execution]) -> Result<Census, String> {
    let mut wall = Duration::ZERO;
    let mut per_file = BTreeMap::<String, usize>::new();
    let mut parses = 0;
    for execution in executions {
        for file in execution.read() {
            let clock = Instant::now();
            let tree = engine::parse(execution.language(), file);
            wall += clock.elapsed();
            if !tree {
                return Err(format!(
                    "the engine's parser produced no tree for {}",
                    file.path
                ));
            }
            parses += 1;
            *per_file.entry(file.path.clone()).or_default() += 1;
        }
    }
    Ok(Census {
        wall,
        executions: executions.len(),
        pairs: executions
            .iter()
            .map(|execution| execution.files().count())
            .sum(),
        parses,
        per_file,
    })
}

fn message(error: impl std::fmt::Display) -> String {
    error.to_string()
}
