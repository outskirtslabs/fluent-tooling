/**
 * @file Fluent grammar for tree-sitter
 * @author David Rios <david.rios.gomes@gmail.com>
 * @author Casey Link <casey@outskirtslabs.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const HEX_DIGITS = "[0-9a-fA-F]";

module.exports = grammar({
  name: "fluent",

  extras: () => [],
  externals: ($) => [
    $._pattern_start,
    $.pure_text,
    $._pattern_end,
    $._pattern_skip,
    $._blank_lines,
    $.unfinished_line,
    $._close_comment_block,
    $._end_positional_args,
    $._error_sentinel,
  ],

  rules: {
    fluent_file: ($) =>
      repeat(
        choice(
          $.comment_block,
          $.group_comment,
          $.file_comment,
          $.message,
          $.term,
          $.doc_commented,
          $._blank_lines,
          $.unfinished_line,
        ),
      ),

    _base_comment: () =>
      token(
        prec(
          1,
          seq(
            "#",
            optional(seq(" ", optional(/[^\r\n]+/))),
            optional(choice("\r\n", "\n", "\r")),
          ),
        ),
      ),
    _comment_block: ($) => repeat1($._base_comment),
    comment_block: ($) => seq($._comment_block, $._close_comment_block),

    _group_comment_line: () =>
      token(
        prec(
          2,
          seq(
            "##",
            optional(seq(" ", optional(/[^\r\n]+/))),
            optional(choice("\r\n", "\n", "\r")),
          ),
        ),
      ),
    group_comment: ($) => prec.right(repeat1($._group_comment_line)),
    _file_comment_line: () =>
      token(
        prec(
          3,
          seq(
            "###",
            optional(seq(" ", optional(/[^\r\n]+/))),
            optional(choice("\r\n", "\n", "\r")),
          ),
        ),
      ),
    file_comment: ($) => prec.right(repeat1($._file_comment_line)),

    doc_commented: ($) =>
      seq(
        alias($._comment_block, $.doc_comment_block),
        choice($.message, $.term),
      ),

    message: ($) =>
      seq(
        field("id", $.identifier),
        optional($._s),
        "=",
        choice(field("value", $.pattern), $._pattern_skip),
        field("attributes", alias(repeat($.attribute), $.attributes)),
      ),

    attribute: ($) =>
      seq(
        ".",
        field("id", $.identifier),
        optional($._s),
        "=",
        choice(field("value", $.pattern), $._pattern_skip),
      ),

    identifier: () => /[a-zA-Z][a-zA-Z0-9_-]*/,

    term: ($) =>
      seq(
        field("id", $.term_identifier),
        optional($._s),
        "=",
        field("value", $.pattern),
        field("attributes", alias(repeat($.attribute), $.attributes)),
      ),

    term_identifier: ($) => seq("-", alias($.identifier, "identifier")),

    variable: ($) => seq("$", alias($.identifier, "identifier")),

    pattern: ($) =>
      seq(
        $._pattern_start,
        repeat1(choice($.pure_text, $.placeable)),
        $._pattern_end,
      ),

    positional_arguments: ($) =>
      seq(
        $._expression,
        repeat(
          seq(alias(token(prec(1, /[ \r\n]*,[ \r\n]*/)), ","), $._expression),
        ),
      ),

    named_argument: ($) =>
      seq(
        field("id", $.identifier),
        alias(token(prec(1, /[ \r\n]*:[ \r\n]*/)), ":"),
        field("value", choice($.number_literal, $.string_literal)),
      ),

    named_arguments: ($) =>
      seq(
        $.named_argument,
        repeat(
          seq(
            alias(token(prec(1, /[ \r\n]*,[ \r\n]*/)), ","),
            $.named_argument,
          ),
        ),
      ),

    function_call: ($) =>
      seq(
        alias(/[ \r\n]*\([ \r\n]*/, "("),
        optional(
          seq(
            optional(
              seq($.positional_arguments, alias($._end_positional_args, ",")),
            ),
            optional($.named_arguments),
          ),
        ),
        alias(/[ \r\n]*\)[ \r\n]*/, ")"),
      ),

    function_reference: ($) =>
      seq(
        alias(field("id", token(prec(1, /[A-Z][A-Z0-9_-]*/))), $.function_name),
        optional($._s),
        $.function_call,
      ),

    message_reference: ($) =>
      seq(
        field("id", $.identifier),
        optional(seq(".", field("attribute", $.identifier))),
      ),

    term_reference: ($) =>
      seq(
        field("id", $.term_identifier),
        optional(seq(".", field("attribute", $.identifier))),
        optional($._ws),
        optional($.function_call),
      ),

    _expression: ($) =>
      choice(
        alias($.placeable, $.inline_placeable),
        $.number_literal,
        $.string_literal,
        $.message_reference,
        $.term_reference,
        $.variable,
        prec(1, $.function_reference),
      ),

    selector_expression: ($) =>
      choice(
        $.number_literal,
        $.string_literal,
        $.variable,
        $.term_reference,
        prec(1, $.function_reference),
      ),

    _selector_key: ($) => choice($.identifier, $.number_literal),

    selector_variant: ($) =>
      seq(
        alias(/\[[ \r\n]*/, "["),
        field("key", $._selector_key),
        alias(/[\r\n ]*\]/, "]"),
        field("value", $.pattern),
      ),

    default_variant: ($) =>
      seq(
        "*",
        alias(/\[[ \r\n]*/, "["),
        field("key", $._selector_key),
        alias(/[\r\n ]*\]/, "]"),
        field("value", $.pattern),
      ),

    selectors: ($) =>
      seq(
        alias(/[ \r\n]*-> *(\r\n|\r|\n)[ \r\n]*/, "->"),
        repeat1(choice($.selector_variant, $.default_variant)),
        "}",
      ),

    placeable: ($) =>
      seq(
        alias(/\{[ \r\n]*/, "{"),
        choice(
          seq($._expression, alias(/[ \r\n]*\}/, "}")),
          seq($.selector_expression, $.selectors),
        ),
      ),

    number_literal: () => token(/-?\d+(\.\d+)?/),

    string_literal: ($) =>
      seq('"', repeat(choice($.escaped_literal, /[^\\"\r\n]+/)), '"'),

    escaped_literal: () =>
      token(
        choice(
          new RegExp(`\\\\u${HEX_DIGITS}{4}`),
          new RegExp(`\\\\U${HEX_DIGITS}{6}`),
          '\\"',
          "\\\\",
        ),
      ),

    _s: () => / +/,
    _ws: () => /[ \r\n]+/,
  },
});
