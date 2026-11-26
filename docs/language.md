# kaalang language

The RFC paragraphs in force, by subject.

## Flows

- Overview and terms: [RFC 0001 §1](rfcs/0001-language.md#1-overview),
  [§2](rfcs/0001-language.md#2-terminology), with deciding selections by
  [RFC 0007 §4.1](rfcs/0007-language-refinements.md#41-deciding-selections).
- The attribute and its annotations:
  [RFC 0009 §2.5](rfcs/0009-types.md#25-the-kaa-namespace), superseding the
  spelling of RFCs 0001–0008.
- Supported edition:
  [RFC 0009 §8](rfcs/0009-types.md#8-compatibility-and-exclusions).
- Inputs and outputs:
  [RFC 0001 §5](rfcs/0001-language.md#5-flow-inputs-and-outputs) and
  [§5.1](rfcs/0001-language.md#51-zero-computation-flow).
- Block statements: [RFC 0001 §3](rfcs/0001-language.md#3-block-statements),
  with initializers by
  [RFC 0007 §3](rfcs/0007-language-refinements.md#3-block-initializers-without-captures)
  and terminators by
  [§3.1](rfcs/0007-language-refinements.md#31-statement-terminators).
- Block kinds: [RFC 0001 §4](rfcs/0001-language.md#4-block-kinds), §4.1–4.4 and
  [§4.7](rfcs/0001-language.md#47-return). `break` no longer exists:
  [RFC 0007 §7](rfcs/0007-language-refinements.md#7-changes-to-earlier-rfcs)
  keeps its restrictions for `return` alone.
- Wires, producers, and captures:
  [RFC 0001 §6](rfcs/0001-language.md#6-wires-producers-and-captures).
- Execution and convergence:
  [RFC 0001 §7](rfcs/0001-language.md#7-execution-and-implicit-convergence),
  with convergence groups by
  [RFC 0007 §4.2](rfcs/0007-language-refinements.md#42-convergence-groups).
- Grammar: [RFC 0001 §8](rfcs/0001-language.md#8-grammar), with stage
  declarations by
  [RFC 0006 §2](rfcs/0006-stages.md#2-preparation-and-stage-declarations), block
  statements by
  [RFC 0007 §2.2](rfcs/0007-language-refinements.md#22-alternative-outputs),
  `continue` by
  [§2.3](rfcs/0007-language-refinements.md#23-explicit-repetition-with-continue),
  initializers by
  [§3](rfcs/0007-language-refinements.md#3-block-initializers-without-captures),
  and cycles by [RFC 0008 §2.3](rfcs/0008-cycle-forms.md#23-grammar).

## Cycles

- Terms: [RFC 0007 §1.1](rfcs/0007-language-refinements.md#11-terms) and
  [RFC 0008 §1.1](rfcs/0008-cycle-forms.md#11-terms).
- Scope and interfaces:
  [RFC 0007 §2](rfcs/0007-language-refinements.md#2-cycle-scope-and-interfaces),
  superseding RFC 0001 §§4.5–4.6. Its gate, outputs, and `continue` belong to
  loop cycles.
- Forms: [RFC 0008 §2](rfcs/0008-cycle-forms.md#2-syntax), loop cycles in
  [§2.1](rfcs/0008-cycle-forms.md#21-loop-cycles) and for cycles in
  [§2.2](rfcs/0008-cycle-forms.md#22-for-cycles).
- For cycle execution and validation:
  [RFC 0008 §3](rfcs/0008-cycle-forms.md#3-execution-of-a-for-cycle) and
  [§4](rfcs/0008-cycle-forms.md#4-validation).

## Stages

- Terms: [RFC 0006 §1.1](rfcs/0006-stages.md#11-terms).
- Declarations, data, and signals:
  [RFC 0006 §2](rfcs/0006-stages.md#2-preparation-and-stage-declarations) and
  [§3](rfcs/0006-stages.md#3-data-scope-and-signals).
- Execution and completion:
  [RFC 0006 §4](rfcs/0006-stages.md#4-execution-and-completion).
- Cycles inside stages: the cycle rules above, as
  [RFC 0007 §7](rfcs/0007-language-refinements.md#7-changes-to-earlier-rfcs) and
  [RFC 0008 §8](rfcs/0008-cycle-forms.md#8-changes-to-earlier-rfcs) apply them,
  superseding RFC 0006 §5.

## Descriptions

- Markdown in descriptions:
  [RFC 0005 §2](rfcs/0005-markdown.md#2-where-formatting-applies) to
  [§5](rfcs/0005-markdown.md#5-semantic-and-processing-boundary), with oversized
  formulas by
  [RFC 0007 §6.4](rfcs/0007-language-refinements.md#64-oversized-formulas).

## Types

- Terms: [RFC 0009 §1.1](rfcs/0009-types.md#11-terms).
- Annotations: [RFC 0009 §2](rfcs/0009-types.md#2-annotations).
- Relationship meaning:
  [RFC 0009 §3](rfcs/0009-types.md#3-relationship-meaning).
- Representation labels:
  [RFC 0009 §5](rfcs/0009-types.md#5-representation-labels).
- Compatibility and exclusions:
  [RFC 0009 §8](rfcs/0009-types.md#8-compatibility-and-exclusions).

## Validation

- Finite analysis:
  [RFC 0007 §5.1](rfcs/0007-language-refinements.md#51-finite-cycle-analysis),
  for stages with [RFC 0006 §6.1](rfcs/0006-stages.md#61-finite-analysis).
- Types:
  [RFC 0009 §7](rfcs/0009-types.md#7-validation-and-acceptance-scenarios).

## Rust lowering

- Flows: [RFC 0004](rfcs/0004-rust-lowering.md) §§1–6, with parameter bindings
  by
  [RFC 0007 §5.3](rfcs/0007-language-refinements.md#53-hygienic-flow-parameters).
- Cycles: [RFC 0007 §5.2](rfcs/0007-language-refinements.md#52-rust-lowering),
  superseding RFC 0004 §7 where they differ; for cycles by
  [RFC 0008 §5](rfcs/0008-cycle-forms.md#5-rust-lowering).
- Stages: [RFC 0006 §6.2](rfcs/0006-stages.md#62-rust-lowering).

## Flow diagrams

- Visual language: [RFC 0002](rfcs/0002-visual-language.md); result junctions
  replace the break of §4.9.
- Cycles:
  [RFC 0007 §6.1](rfcs/0007-language-refinements.md#61-cycles-and-continue) and
  [§6.2](rfcs/0007-language-refinements.md#62-result-junctions-and-construction);
  for cycles by [RFC 0008 §6](rfcs/0008-cycle-forms.md#6-visual-representation).
- Row heights:
  [RFC 0007 §6.3](rfcs/0007-language-refinements.md#63-aligning-node-heights).
- Stages: [RFC 0006 §7](rfcs/0006-stages.md#7-visual-representation).
- Layout: [RFC 0003 §2](rfcs/0003-svg-renderer.md#2-layout).
- SVG output and failures: [RFC 0003 §3](rfcs/0003-svg-renderer.md#3-svg-output)
  and [§4](rfcs/0003-svg-renderer.md#4-validation-and-failures), with
  description styling by
  [RFC 0005 §4](rfcs/0005-markdown.md#4-presentation-and-accessibility).
- Limits: [RFC 0003 §6](rfcs/0003-svg-renderer.md#6-scope-and-limits), whose
  limits on searching a crate, external modules, and conditional compilation
  [RFC 0009 §6](rfcs/0009-types.md#6-command-line-interface) supersedes.

## Type diagrams

- Scope, structure, and arrangement:
  [RFC 0009 §4](rfcs/0009-types.md#4-type-diagrams).
- Card paths: [RFC 0010 §2](rfcs/0010-type-card-paths.md#2-displayed-paths),
  superseding the crate-root path rule of RFC 0009 §4.2.

## Command-line interface

- [RFC 0009 §6](rfcs/0009-types.md#6-command-line-interface), superseding RFC
  0003 §5.

## Acceptance scenarios

- [RFC 0006 §10](rfcs/0006-stages.md#10-acceptance-scenarios),
  [RFC 0007 §8](rfcs/0007-language-refinements.md#8-acceptance-scenarios),
  [RFC 0008 §9](rfcs/0008-cycle-forms.md#9-acceptance-scenarios),
  [RFC 0009 §7](rfcs/0009-types.md#7-validation-and-acceptance-scenarios), and
  [RFC 0010 §3](rfcs/0010-type-card-paths.md#3-acceptance-scenarios).
