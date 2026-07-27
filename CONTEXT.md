# Fluent Tooling

Fluent Tooling parses and checks Fluent Translation List (FTL) resources for localization and editor workflows. This glossary defines shared language for Fluent source and diagnostics independently of any parser, linter, or editor implementation.

## Resources and entries

**FTL resource**:
A complete unit of Fluent source, whether stored in a file, held in memory, or sent through a stream. It contains an ordered sequence of entries.
_Avoid_: FTL file when the source is not specifically file-backed

**Entry**:
A top-level unit in an FTL resource. An entry may be a message, term, comment, or malformed source retained for diagnosis.
_Avoid_: Record, node

**Message**:
A localizable entry identified by an unprefixed identifier. It has a value, attributes, or both.
_Avoid_: Public message, translation string, key-value pair

**Term**:
A reusable localizable entry identified with a leading `-`. It always has a value and may have attributes.
_Avoid_: Private message

**Identifier**:
The name of a message, term, attribute, variable, or function. Prefix sigils and attribute separators express how the name is used.
_Avoid_: Key when naming a message or term

**Attribute**:
A named secondary value attached to a message or term. It describes a facet of that entry rather than a separate top-level entry.
_Avoid_: Property, field

**Comment**:
A non-localized annotation in an FTL resource. A comment may describe one entry, a group of entries, or the resource as a whole.

**Resource comment**:
A `###` comment that describes an FTL resource as a whole rather than one entry or group.
_Avoid_: File comment in domain prose

## Values and expressions

**Pattern**:
An ordered sequence of literal text and placeables that forms a localized value.
_Avoid_: String, template

**Placeable**:
A braced expression embedded in a pattern. Its result contributes text or selects a pattern.
_Avoid_: Placeholder, interpolation

**Expression**:
A value-producing or choice-making construct inside a placeable. Literals, references, variables, function calls, and select expressions are expressions.

**Reference**:
An expression that reuses a message, term, or attribute by identifier.
_Avoid_: Link

**Variable**:
A named runtime input referenced with `$`. A consumer or term argument supplies it rather than an FTL entry.
_Avoid_: Placeholder

**Select expression**:
An expression that chooses one variant by matching a selector against variant keys.
_Avoid_: Plural expression, switch

**Selector**:
The expression whose value drives variant matching. Numeric selectors can invoke locale plural categories without a special plural operation.
_Avoid_: Condition, plural function

**Variant**:
A keyed pattern within a select expression.
_Avoid_: Case, branch

**Default variant**:
The variant marked with `*` and chosen when no other key matches. Every select expression has exactly one.
_Avoid_: Else branch, fallback branch

## Diagnostics

**Diagnostic**:
A source-related finding that reports a Fluent syntax, structural, or project lint problem. It carries a stable code, severity, and explanation.
_Avoid_: Error when the severity may be a warning

**Fluent error**:
A diagnostic defined by Project Fluent syntax or structural rules and identified by a stable `E` code.
_Avoid_: Project lint

**Project lint**:
A Fluent Tooling-specific diagnostic for valid but redundant or discouraged FTL source. It remains distinct from a Project Fluent syntax error.
_Avoid_: Syntax error

**Source label**:
An explanation anchored to a specific source range. A primary label identifies the core problem; secondary labels provide related context.
_Avoid_: Line number
