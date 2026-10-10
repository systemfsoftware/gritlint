---
reasonCode: service-exports-layer
nextAction: Move the driver import and every Layer built from it into a module beside the service that only composition roots and tests import, keep at most a pure `static readonly layer` on the Service class, and delete every other Layer export from the service module.
locate: pattern driver_specifier() { r"['\"](?:node:[A-Za-z0-9_/.-]+|(?:fs|fs/promises|child_process|worker_threads|cluster|net|tls|dgram|dns|http|https|http2|inspector|vm|os|crypto|readline|zlib|stream|process|perf_hooks|async_hooks|v8)|@effect/platform-[a-z0-9-]+(?:/[A-Za-z0-9_/.-]+)?|@effect/sql-[a-z0-9-]+(?:/[A-Za-z0-9_/.-]+)?|@effect/ai-[a-z0-9-]+(?:/[A-Za-z0-9_/.-]+)?|(?:pg|postgres|mysql2|drizzle-orm|kysely|memfs|@prisma/client|@electric-sql/pglite|mongoose|better-sqlite3|ioredis|redis|mongodb|openai|stripe|@anthropic-ai/sdk|@aws-sdk/[a-z0-9-]+|@google-cloud/[a-z0-9-]+|@azure/[a-z0-9-]+)(?:/[A-Za-z0-9_/.-]+)?)['\"]" } or { import_statement(source=$source) where { $source <: driver_specifier() }, export_statement(source=$source) where { $source <: driver_specifier() }, `import($specifier)` where { $specifier <: driver_specifier() }, import_statement(import=$clause) where { $clause <: import_require_clause(source=$source) where { $source <: driver_specifier() } }, import_specifier() as $specifier where { $specifier <: r"(?:layer|[A-Za-z0-9_$]+Layer|[A-Za-z0-9_$]*Live)\s+as\s+[A-Za-z0-9_$]+", $specifier <: not within import_statement() as $type_only where { $type_only <: r"import\s+type\b[\s\S]*" } }, `Layer.$method($...)` as $call where { $call <: within export_statement(), $call <: not within class_body() }, `export const $live = $_` where { $live <: r".*Live" }, export_statement() as $statement where { $statement <: r"export\s+(?:(?:const|let|var)\s+[A-Za-z0-9_$]+\s*(?::[\s\S]+?)?=|default\b)\s*(?:[A-Za-z0-9_$]+\s*\.\s*)*(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)\s*(?:(?:as|satisfies)\b[\s\S]*)?;?" }, export_specifier(name=$exported) as $specifier where { $specifier <: not r"type\s[\s\S]*", $specifier <: not within export_statement() as $type_only where { $type_only <: r"export\s+type\b[\s\S]*" }, $exported <: r"(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)" }, public_field_definition(name=$field) where { $field <: r"(?:[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)" }, public_field_definition(value=$value) where { $value <: r"(?:[A-Za-z0-9_$]+\s*\.\s*)*(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)", $value <: not r"this\s*\.[\s\S]*" } }
---

# A service module imports no driver and exports no Layer

A `*.service.ts` module in this directory imports a driver or platform runtime
(an `@effect/platform-*`, `@effect/sql-*` or `@effect/ai-*` package, a database
client, a Node built-in, or a vendor SDK), statically, through `import()`, or
through `import x = require(...)`, or hands out a Layer outside a pure member
of its Service class: a module-level exported Layer value or factory, a `*Live`
export, a class field named `*Layer` or `*Live`, a re-export or alias of a
Layer, an import that renames a `*Layer` or `*Live` binding, or an export
bound to a driver's Layer. A `static readonly layer`,
`layerTest` or `layerConfig` on the Service class is allowed when the module
imports no driver. One finding covers the directory and points at the first
driver import or Layer in its first offending service module; search every
`*.service.ts` in the directory for the rest.

```grit
language js

multifile {
  file($name, $body) where {
    $name <: r".*\.service\.ts",
    $body <: contains or {
      import_statement(source=$source) where { $source <: driver_specifier() },
      export_statement(source=$source) where { $source <: driver_specifier() },
      `import($specifier)` where { $specifier <: driver_specifier() },
      import_require_clause(source=$source) where { $source <: driver_specifier() },
      import_statement() as $statement where {
        $statement <: not r"import\s+type\b[\s\S]*",
        $statement <: contains import_specifier() as $specifier where { $specifier <: r"(?:layer|[A-Za-z0-9_$]+Layer|[A-Za-z0-9_$]*Live)\s+as\s+[A-Za-z0-9_$]+" }
      },
      export_statement() as $statement where {
        $statement <: contains `Layer.$method($...)` as $call where { $call <: not within class_body() }
      },
      `export const $live = $_` where { $live <: r".*Live" },
      export_statement() as $statement where {
        $statement <: r"export\s+(?:(?:const|let|var)\s+[A-Za-z0-9_$]+\s*(?::[\s\S]+?)?=|default\b)\s*(?:[A-Za-z0-9_$]+\s*\.\s*)*(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)\s*(?:(?:as|satisfies)\b[\s\S]*)?;?"
      },
      export_statement() as $statement where {
        $statement <: not r"export\s+type\b[\s\S]*",
        $statement <: contains export_specifier(name=$exported) as $specifier where {
          $specifier <: not r"type\s[\s\S]*",
          $exported <: r"(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)"
        }
      },
      public_field_definition(name=$field) where { $field <: r"(?:[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)" },
      public_field_definition(value=$value) where {
        $value <: r"(?:[A-Za-z0-9_$]+\s*\.\s*)*(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)",
        $value <: not r"this\s*\.[\s\S]*"
      }
    }
  }
}

pattern driver_specifier() {
  r"['\"](?:node:[A-Za-z0-9_/.-]+|(?:fs|fs/promises|child_process|worker_threads|cluster|net|tls|dgram|dns|http|https|http2|inspector|vm|os|crypto|readline|zlib|stream|process|perf_hooks|async_hooks|v8)|@effect/platform-[a-z0-9-]+(?:/[A-Za-z0-9_/.-]+)?|@effect/sql-[a-z0-9-]+(?:/[A-Za-z0-9_/.-]+)?|@effect/ai-[a-z0-9-]+(?:/[A-Za-z0-9_/.-]+)?|(?:pg|postgres|mysql2|drizzle-orm|kysely|memfs|@prisma/client|@electric-sql/pglite|mongoose|better-sqlite3|ioredis|redis|mongodb|openai|stripe|@anthropic-ai/sdk|@aws-sdk/[a-z0-9-]+|@google-cloud/[a-z0-9-]+|@azure/[a-z0-9-]+)(?:/[A-Za-z0-9_/.-]+)?)['\"]"
}
```

## Why

A service module is the contract every consumer imports. A driver import there
puts the driver into every consumer's import graph, whether or not a Layer
uses it, so a consumer cannot take the contract without the technology behind
it. A pure Layer on the Service class (`static readonly layer =
Layer.effect(this, this.make)`) imports nothing beyond Effect and other
contracts; it may require a platform service such as `FileSystem` in its `R`
channel, and the composition root provides it. A Layer built from a driver,
and any second name for a Layer (a module-level export, a `*Live`, a
re-export, an alias), belongs in a module beside the service that only
composition roots and tests import. That module sits flat beside the service,
in no folder, and its name is free.

## A tripwire, not the control

gritlint reads one file at a time and does not resolve imports, so this rule
sees a service module's own import specifiers and Layer shapes, never what an
imported module reaches. The control is the package graph: a contract package
whose manifest declares no driver cannot import one, and pnpm and `tsc` refuse
the import. Within one package, where the graph imposes no seam, this rule is
the deterministic signal. The driver specifiers are a reviewed preset in this
rule (`driver_specifier`); a technology missing from it passes until the preset
names it.
