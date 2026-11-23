"""Apply gritlint's grammar changes to the resolved grammar.json of both parsers.

Run from this directory, then regenerate each parser:

    python3 patch-grammar.py
    (cd typescript && tree-sitter generate --abi 14 src/grammar.json)
    (cd tsx && tree-sitter generate --abi 14 src/grammar.json)

The edits are idempotent: a rule that already carries a change is left alone, and
`import_type` is always rewritten to the definition below.
"""

import copy
import json
from pathlib import Path


def sym(name):
    return {"type": "SYMBOL", "name": name}


def string(value):
    return {"type": "STRING", "value": value}


def seq(*members):
    return {"type": "SEQ", "members": list(members)}


def choice(*members):
    return {"type": "CHOICE", "members": list(members)}


def field(name, content):
    return {"type": "FIELD", "name": name, "content": content}


BLANK = {"type": "BLANK"}

VARIANCE = choice(string("in"), string("out"), seq(string("in"), string("out")), BLANK)

EXPORT_TYPE_STAR = [
    seq(string("export"), sym("type"), string("*"), sym("_from_clause"), sym("_semicolon")),
    seq(
        string("export"),
        sym("type"),
        field("export", sym("namespace_export")),
        sym("_from_clause"),
        sym("_semicolon"),
    ),
]

# import('x') or import('x', { with: ... }), then an optional qualified name and type arguments.
IMPORT_TYPE = {
    "type": "PREC_RIGHT",
    "value": 0,
    "content": seq(
        sym("import"),
        string("("),
        field("argument", sym("string")),
        choice(seq(string(","), sym("object"), choice(string(","), BLANK)), BLANK),
        string(")"),
        choice(
            seq(
                string("."),
                field("name", sym("_type_identifier")),
                {
                    "type": "REPEAT",
                    "content": seq(string("."), field("name", sym("_type_identifier"))),
                },
                choice(sym("type_arguments"), BLANK),
            ),
            BLANK,
        ),
    ),
}

AMPERSAND_TEXT = {"type": "PATTERN", "value": "&"}
AMPERSAND_FRAGMENT = {
    "type": "ALIAS",
    "content": {"type": "IMMEDIATE_TOKEN", "content": string("&")},
    "named": True,
    "value": "string_fragment",
}


def add_once(members, item):
    if item not in members:
        members.append(item)


def patch(path, tsx):
    grammar = json.loads(path.read_text())
    rules = grammar["rules"]

    parameter = rules["type_parameter"]["members"]
    if VARIANCE not in parameter:
        parameter.insert(1, copy.deepcopy(VARIANCE))

    for alternative in EXPORT_TYPE_STAR:
        add_once(rules["export_statement"]["members"], alternative)

    rules["import_type"] = IMPORT_TYPE
    add_once(rules["_primary_type"]["members"], sym("import_type"))
    add_once(grammar["conflicts"], ["primary_expression", "import_type"])

    if tsx:
        add_once(rules["jsx_text"]["members"], AMPERSAND_TEXT)
        for variant in rules["_jsx_string"]["members"]:
            add_once(variant["members"][1]["content"]["members"], AMPERSAND_FRAGMENT)

    path.write_text(json.dumps(grammar, indent=2) + "\n")


here = Path(__file__).parent
patch(here / "typescript/src/grammar.json", tsx=False)
patch(here / "tsx/src/grammar.json", tsx=True)
