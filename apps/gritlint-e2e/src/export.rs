use std::fs;
use std::path::Path;
use std::process::Command;

const PLANTED: [(&str, &str); 2] = [
    (
        "gritlint.json",
        r#"{
  "packs": {
    "npm-provenance": { "repositoryUrl": "`\"git+https://github.com/acme/widgets.git\"`" }
  }
}
"#,
    ),
    (
        "rules/license.md",
        "---
reasonCode: unlicensed
nextAction: Choose a license.
---

# A package is licensed

A package does not declare `UNLICENSED`.

```grit
language json
`\"license\": \"UNLICENSED\"`
```
",
    ),
];

const PLUGIN: &str = "language json

pattern gritlint_exported_rule() {
  `\"license\": \"UNLICENSED\"`
}

gritlint_exported_rule() as $gritlint_match where {
  register_diagnostic(span = $gritlint_match, message = \"A package does not declare `UNLICENSED`.\", severity = \"error\")
}
";

struct Run {
    exit: i32,
    stdout: String,
    stderr: String,
}

pub fn journey(gritlint: &Path, root: &Path) -> Result<(), String> {
    let directory = root.join("export");
    for (path, contents) in PLANTED {
        let target = directory.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("create {path}: {error}"))?;
        }
        fs::write(&target, contents).map_err(|error| format!("write {path}: {error}"))?;
    }
    lists_every_enabled_rule(gritlint, &directory)?;
    prints_an_exportable_rule(gritlint, &directory)?;
    refuses(
        gritlint,
        &directory,
        "npm-provenance/repository-url",
        "export-multifile",
    )?;
    refuses(gritlint, &directory, "local/nope", "export-unknown-rule")
}

fn export(gritlint: &Path, directory: &Path, rule: Option<&str>) -> Result<Run, String> {
    let mut command = Command::new(gritlint);
    command
        .args(["export", "biome", "--rules", "rules"])
        .current_dir(directory);
    if let Some(rule) = rule {
        command.args(["--rule", rule]);
    }
    let output = command
        .output()
        .map_err(|error| format!("run gritlint export biome: {error}"))?;
    Ok(Run {
        exit: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn lists_every_enabled_rule(gritlint: &Path, directory: &Path) -> Result<(), String> {
    let run = export(gritlint, directory, None)?;
    let lines: Vec<&str> = run.stdout.lines().collect();
    let refused = lines
        .iter()
        .skip(1)
        .all(|line| line.contains(" refused export-multifile: "));
    if run.exit != 0
        || lines.first() != Some(&"local/license exportable")
        || lines.len() != 4
        || !refused
    {
        return Err(format!(
            "the listing exited {} with stdout {:?} and stderr {:?}",
            run.exit, run.stdout, run.stderr
        ));
    }
    Ok(())
}

fn prints_an_exportable_rule(gritlint: &Path, directory: &Path) -> Result<(), String> {
    let run = export(gritlint, directory, Some("local/license"))?;
    if run.exit != 0 || run.stdout != PLUGIN {
        return Err(format!(
            "exporting local/license exited {} with stdout {:?} and stderr {:?}",
            run.exit, run.stdout, run.stderr
        ));
    }
    Ok(())
}

fn refuses(gritlint: &Path, directory: &Path, rule: &str, reason_code: &str) -> Result<(), String> {
    let run = export(gritlint, directory, Some(rule))?;
    let prefix = format!("ERROR gritlint: {rule}: {reason_code}: ");
    let lines: Vec<&str> = run.stderr.lines().collect();
    let one_line = matches!(lines.as_slice(), [line] if line.starts_with(&prefix) && line.contains(" Next action: "));
    if run.exit != 2 || !run.stdout.is_empty() || !one_line {
        return Err(format!(
            "exporting {rule} exited {} with stdout {:?} and stderr {:?}",
            run.exit, run.stdout, run.stderr
        ));
    }
    Ok(())
}
