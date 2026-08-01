"""Token-based Rust source policy for production coverage accounting."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import PurePosixPath
class SourcePolicyError(ValueError): pass
@dataclass(frozen=True)
class Token:
    text: str; line: int; end_line: int; literal: bool = False
@dataclass(frozen=True)
class SourcePolicy:
    test_only_lines: frozenset[int]; coverage_control: bool
OPENERS = {"(": ")", "[": "]", "{": "}"}
CLOSERS = {value: key for key, value in OPENERS.items()}
CONTINUE_AFTER_GROUP = {".", "?", "+", "-", "*", "/", "%", "&", "|", "||", "^", "!", "=", "<", ">", ":"}
AFTER_GENERIC = set("()[]{}.,;:?+-*/%&|^!=<>") | {"as", "for", "where", "||"}
ITEM_TARGETS = {"fn", "mod", "struct", "enum", "union", "impl", "trait", "type", "static", "use", "macro", "macro_rules"}
GENERIC_DECLARATIONS = {"fn", "struct", "enum", "union", "trait", "type"}
COVERAGE_NAMES = {"coverage", "coverage_nightly"}
SUPPRESSION_NAMES = COVERAGE_NAMES | {"automatically_derived", "naked"}
NON_PRODUCTION_PATH_PARTS = frozenset({"test", "tests", "example", "examples", "bench", "benches"})
RUST_KEYWORDS = {"abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield"}
def is_production_rust_path(path: str) -> bool:
    source = PurePosixPath(path); parts = source.parts
    if not path.endswith(".rs") or source.stem in NON_PRODUCTION_PATH_PARTS or any(part in NON_PRODUCTION_PATH_PARTS for part in parts): return False
    return len(parts) >= 2 and parts[0] == "src" or len(parts) >= 4 and parts[0] == "crates" and parts[2] == "src"
def _identifier(text: str) -> str:
    return text[2:] if text.startswith("r#") else text
def _identifier_end(source: str, start: int) -> int:
    if start >= len(source) or not (source[start] == "_" or source[start].isalpha()):
        return start
    cursor = start + 1
    while cursor < len(source) and (source[cursor] == "_" or source[cursor].isalnum()):
        cursor += 1
    return cursor
def _literal_with_suffix(source: str, end: int) -> int:
    return _identifier_end(source, end)
def _literal(source: str, start: int, line: int) -> tuple[int, int] | None:
    index = start
    raw = source.startswith(("br", "cr"), index)
    if raw:
        index += 2
    elif index < len(source) and source[index] == "r":
        raw = True
        index += 1
    if raw:
        hashes = 0
        while index < len(source) and source[index] == "#":
            hashes += 1
            index += 1
        if index >= len(source) or source[index] != '"':
            return None
        marker = '"' + "#" * hashes
        stop = source.find(marker, index + 1)
        if stop < 0:
            raise SourcePolicyError("unterminated raw string")
        end = stop + len(marker)
        return _literal_with_suffix(source, end), line + source[start:end].count("\n")
    if index < len(source) and source[index] in "bc":
        index += 1
    if index < len(source) and source[index] == '"':
        quote = '"'
    elif index < len(source) and source[index] == "'":
        quote = "'"
    else:
        return None
    cursor = index + 1
    if quote == "'":
        if cursor >= len(source) or source[cursor] == "\n":
            return None
        if source[cursor] != "\\":
            cursor += 1
        elif source.startswith("\\u{", cursor):
            close = source.find("}", cursor + 3)
            if close < 0:
                return None
            cursor = close + 1
        elif source.startswith("\\x", cursor):
            cursor += 4
        else:
            cursor += 2
        return (
            _literal_with_suffix(source, cursor + 1), line
        ) if cursor < len(source) and source[cursor] == "'" else None
    end_line = line
    while cursor < len(source):
        character = source[cursor]
        if character == "\n":
            end_line += 1
            cursor += 1
            continue
        if character == "\\":
            end_line += cursor + 1 < len(source) and source[cursor + 1] == "\n"
            cursor += 2
            continue
        if character == quote:
            return _literal_with_suffix(source, cursor + 1), end_line
        cursor += 1
    raise SourcePolicyError("unterminated string literal")
def _numeric_literal(source: str, start: int, line: int) -> tuple[int, int] | None:
    if start >= len(source) or not source[start].isdigit():
        return None
    cursor = start
    if source.startswith(("0x", "0X"), start):
        cursor += 2
        while cursor < len(source) and (source[cursor] in "0123456789abcdefABCDEF_"):
            cursor += 1
    elif source.startswith(("0o", "0O"), start):
        cursor += 2
        while cursor < len(source) and source[cursor] in "01234567_":
            cursor += 1
    elif source.startswith(("0b", "0B"), start):
        cursor += 2
        while cursor < len(source) and source[cursor] in "01_":
            cursor += 1
    else:
        while cursor < len(source) and (source[cursor].isdigit() or source[cursor] == "_"):
            cursor += 1
        if (
            cursor + 1 < len(source)
            and source[cursor] == "."
            and source[cursor + 1].isdigit()
        ):
            cursor += 1
            while cursor < len(source) and (source[cursor].isdigit() or source[cursor] == "_"):
                cursor += 1
        exponent = cursor
        if cursor < len(source) and source[cursor] in "eE":
            cursor += 1
            if cursor < len(source) and source[cursor] in "+-":
                cursor += 1
            digits = cursor
            while cursor < len(source) and (source[cursor].isdigit() or source[cursor] == "_"):
                cursor += 1
            if cursor == digits:
                cursor = exponent
    return _literal_with_suffix(source, cursor), line
def _tokens(data: bytes) -> list[Token]:
    try:
        source = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise SourcePolicyError("Rust source is not UTF-8") from error
    if source.startswith("\ufeff"): source = source[1:]
    result: list[Token] = []
    index = 0
    line = 1
    if source.startswith("#!") and not source.startswith("#!["):
        stop = source.find("\n")
        index, line = (len(source), 1) if stop < 0 else (stop + 1, 2)
    while index < len(source):
        character = source[index]
        if character.isspace() or character in "\u200e\u200f":
            line += character == "\n"
            index += 1
            continue
        if source.startswith("//", index):
            stop = source.find("\n", index + 2)
            index = len(source) if stop < 0 else stop
            continue
        if source.startswith("/*", index):
            depth = 1
            cursor = index + 2
            while cursor < len(source) and depth:
                if source.startswith("/*", cursor):
                    depth += 1
                    cursor += 2
                elif source.startswith("*/", cursor):
                    depth -= 1
                    cursor += 2
                else:
                    line += source[cursor] == "\n"
                    cursor += 1
            if depth:
                raise SourcePolicyError("unterminated block comment")
            index = cursor
            continue
        parsed = _literal(source, index, line)
        if parsed is None:
            parsed = _numeric_literal(source, index, line)
        if parsed is not None:
            end, end_line = parsed
            result.append(Token(source[index:end], line, end_line, True))
            index, line = end, end_line
            continue
        if character == "r" and source.startswith("r#", index) and index + 2 < len(source) and (source[index + 2] == "_" or source[index + 2].isalpha()):
            cursor = index + 3
            while cursor < len(source) and (source[cursor] == "_" or source[cursor].isalnum()):
                cursor += 1
            result.append(Token(source[index:cursor], line, line))
            index = cursor
            continue
        if character == "_" or character.isalpha():
            cursor = index + 1
            while cursor < len(source) and (source[cursor] == "_" or source[cursor].isalnum()):
                cursor += 1
            result.append(Token(source[index:cursor], line, line))
            index = cursor
            continue
        if source.startswith("||", index): result.append(Token("||", line, line)); index += 2; continue
        result.append(Token(character, line, line))
        index += 1
    return result
def _pairs(tokens: list[Token]) -> dict[int, int]:
    stack: list[int] = []
    result: dict[int, int] = {}
    for index, token in enumerate(tokens):
        if token.text in OPENERS:
            stack.append(index)
        elif token.text in CLOSERS:
            if not stack or tokens[stack[-1]].text != CLOSERS[token.text]:
                raise SourcePolicyError("unbalanced Rust delimiters")
            opener = stack.pop()
            result[opener] = index
            result[index] = opener
    if stack:
        raise SourcePolicyError("unbalanced Rust delimiters")
    return result
def _attribute(tokens: list[Token], pairs: dict[int, int], index: int) -> tuple[int, int, bool] | None:
    if tokens[index].text != "#":
        return None
    cursor = index + 1
    inner = cursor < len(tokens) and tokens[cursor].text == "!"
    cursor += inner
    if cursor >= len(tokens) or tokens[cursor].text != "[":
        return None
    return cursor + 1, pairs[cursor], inner
def _following_group(
    tokens: list[Token], pairs: dict[int, int], index: int, stop: int,
) -> tuple[int, int] | None:
    opener = index + 1
    if opener >= stop or tokens[opener].text != "(":
        return None
    close = pairs[opener]
    return (opener, close) if close < stop else None
def _has_coverage_name(tokens: list[Token], start: int, stop: int) -> bool:
    return any(
        not token.literal and _identifier(token.text) in COVERAGE_NAMES
        for token in tokens[start:stop]
    )
def _first_top_level_comma(
    tokens: list[Token], pairs: dict[int, int], start: int, stop: int,
) -> int:
    cursor = start
    while cursor < stop:
        if tokens[cursor].text in OPENERS:
            cursor = pairs[cursor] + 1
        elif tokens[cursor].text == ",":
            return cursor
        else:
            cursor += 1
    return stop
def _top_level_comma_segments(
    tokens: list[Token], pairs: dict[int, int], start: int, stop: int,
) -> list[tuple[int, int]]:
    segments: list[tuple[int, int]] = []
    segment_start = start
    cursor = start
    while cursor < stop:
        if tokens[cursor].text in OPENERS:
            cursor = pairs[cursor] + 1
            continue
        if tokens[cursor].text != ",":
            cursor += 1
            continue
        if segment_start < cursor:
            segments.append((segment_start, cursor))
        cursor += 1
        segment_start = cursor
    if segment_start < stop:
        segments.append((segment_start, stop))
    return segments
def _simple_string_literal(token: Token) -> str | None:
    if not token.literal:
        return None
    text = token.text
    if text.startswith('"'):
        close = text.find('"', 1)
        if close < 0 or "\\" in text[1:close]:
            return None
        return text[1:close]
    if not text.startswith("r"):
        return None
    cursor = 1
    while cursor < len(text) and text[cursor] == "#":
        cursor += 1
    if cursor >= len(text) or text[cursor] != '"':
        return None
    hashes = cursor - 1
    marker = '"' + "#" * hashes
    close = text.find(marker, cursor + 1)
    return None if close < 0 else text[cursor + 1:close]
def _safe_rust_module_path(
    tokens: list[Token],
    start: int,
    stop: int,
    allow_non_production: bool,
    inline_module_path_safe: bool,
) -> bool:
    if start + 3 != stop or tokens[start + 1].text != "=":
        return False
    value = _simple_string_literal(tokens[start + 2])
    if value is None or "\\" in value or "\x00" in value:
        return False
    path = PurePosixPath(value)
    return (
        not path.is_absolute()
        and not (len(value) >= 2 and value[0].isalpha() and value[1] == ":")
        and ".." not in path.parts
        and path.suffix == ".rs"
        and (
            allow_non_production
            or (
                inline_module_path_safe
                and path.stem not in NON_PRODUCTION_PATH_PARTS
                and not any(part in NON_PRODUCTION_PATH_PARTS for part in path.parts)
            )
        )
    )
def _meta_coverage_control(
    tokens: list[Token],
    pairs: dict[int, int],
    start: int,
    stop: int,
    allow_non_production_path: bool,
    inline_module_path_safe: bool,
) -> bool:
    work = [(start, stop)]
    while work:
        begin, end = work.pop()
        if begin >= end or tokens[begin].literal:
            continue
        name = _identifier(tokens[begin].text)
        if name == "path":
            if not _safe_rust_module_path(
                tokens, begin, end, allow_non_production_path, inline_module_path_safe,
            ):
                return True
            continue
        if name in SUPPRESSION_NAMES:
            return True
        group = _following_group(tokens, pairs, begin, end)
        if group is None:
            continue
        opener, close = group
        if name == "cfg":
            if _has_coverage_name(tokens, opener + 1, close):
                return True
            continue
        segments = _top_level_comma_segments(tokens, pairs, opener + 1, close)
        if name == "unsafe":
            work.extend(segments)
        elif name == "cfg_attr" and segments:
            if _has_coverage_name(tokens, *segments[0]):
                return True
            work.extend(segments[1:])
    return False
def _macro_ranges(tokens: list[Token], pairs: dict[int, int]) -> list[tuple[int, int, bool]]:
    result: list[tuple[int, int, bool]] = []
    for index, token in enumerate(tokens):
        if token.text != "!" or not index or tokens[index - 1].literal:
            continue
        name = _identifier(tokens[index - 1].text)
        if not name.isidentifier() or not tokens[index - 1].text.startswith("r#") and name in RUST_KEYWORDS:
            continue
        definition = tokens[index - 1].text == "macro_rules" and index + 2 < len(tokens) and not tokens[index + 1].literal and _identifier(tokens[index + 1].text).isidentifier() and tokens[index + 2].text in OPENERS
        cursor = index + 2 if definition else index + 1
        if cursor < len(tokens) and tokens[cursor].text in OPENERS:
            result.append((cursor, pairs[cursor], definition))
    return result
def _module_has_path_override(
    tokens: list[Token], pairs: dict[int, int], index: int,
) -> bool:
    cursor = index - 1
    while cursor >= 0 and tokens[cursor].text not in ";{}":
        if tokens[cursor].text == "]":
            opener = pairs[cursor]
            marker = opener - 1
            attribute = _attribute(tokens, pairs, marker) if marker >= 0 else None
            if (
                attribute is not None
                and not attribute[2]
                and _identifier(tokens[attribute[0]].text) == "path"
            ):
                return True
            cursor = marker
        cursor -= 1
    return False
def _coverage_control(
    tokens: list[Token],
    pairs: dict[int, int],
    test_only_spans: list[tuple[int, int]],
    source_path: str,
) -> bool:
    source_is_production = is_production_rust_path(source_path)
    macro_ranges = _macro_ranges(tokens, pairs)
    macro_stops = {start: (stop, definition) for start, stop, definition in macro_ranges}
    macro_depth = [0] * (len(tokens) + 1)
    for start, stop, _definition in macro_ranges: macro_depth[start + 1] += 1; macro_depth[stop] -= 1
    for index in range(1, len(tokens)): macro_depth[index] += macro_depth[index - 1]
    always, production, dollars, safe_tests = [0], [0], [0], [0]
    for token_index, item in enumerate(tokens):
        name = "" if item.literal else _identifier(item.text)
        suppressed = name in SUPPRESSION_NAMES
        safe_test_attribute = (
            name in ("test", "bench")
            and token_index >= 2
            and tokens[token_index - 2].text == "#"
            and tokens[token_index - 1].text == "["
            and token_index + 1 < len(tokens)
            and tokens[token_index + 1].text == "]"
        )
        always.append(always[-1] + suppressed)
        production.append(production[-1] + (suppressed or name == "path" or name in NON_PRODUCTION_PATH_PARTS))
        dollars.append(dollars[-1] + (item.text == "$"))
        safe_tests.append(safe_tests[-1] + safe_test_attribute)
    excluded_inline_spans = sorted([
        (opener, close)
        for opener, close in pairs.items()
        if tokens[opener].text == "{"
        and opener >= 2
        and tokens[opener - 2].text == "mod"
        and _identifier(tokens[opener - 1].text) in NON_PRODUCTION_PATH_PARTS
    ])
    test_span_index = inline_span_index = 0
    use_end = -1
    for index, token in enumerate(tokens):
        while test_span_index < len(test_only_spans) and test_only_spans[test_span_index][1] < index:
            test_span_index += 1
        while inline_span_index < len(excluded_inline_spans) and excluded_inline_spans[inline_span_index][1] < index:
            inline_span_index += 1
        test_only = (
            test_span_index < len(test_only_spans)
            and test_only_spans[test_span_index][0] <= index
        )
        inline_path_safe = not (
            inline_span_index < len(excluded_inline_spans)
            and excluded_inline_spans[inline_span_index][0] < index
        )
        name = "" if token.literal else _identifier(token.text)
        if token.text == "use" and not macro_depth[index] and index > use_end and (index + 1 == len(tokens) or tokens[index + 1].text != "<"): use_end = _target_end(tokens, pairs, index, None)
        if name == "include" and (
            macro_depth[index] or index <= use_end
            or index + 1 < len(tokens) and tokens[index + 1].text == "!"
        ):
            return True
        if (
            token.text == "["
            and index >= 2
            and tokens[index - 2].text == "$"
            and not tokens[index - 1].literal
            and dollars[pairs[index]] > dollars[index + 1]
        ):
            return True
        attribute = _attribute(tokens, pairs, index)
        if attribute is not None:
            start, stop, _inner = attribute
            if (
                dollars[stop] > dollars[start] and _identifier(tokens[start].text) != "doc"
            ) or _meta_coverage_control(
                tokens,
                pairs,
                start,
                stop,
                test_only or not source_is_production,
                inline_path_safe,
            ):
                return True
        if (
            source_is_production
            and not test_only
            and not token.literal
            and name == "mod"
            and index + 2 < len(tokens)
            and not tokens[index + 1].literal
            and _identifier(tokens[index + 1].text).isidentifier()
            and tokens[index + 2].text == ";"
            and not _module_has_path_override(tokens, pairs, index)
        ):
            if (
                _identifier(tokens[index + 1].text)
                in NON_PRODUCTION_PATH_PARTS
                or not inline_path_safe
            ):
                return True
        if _identifier(token.text) == "cfg" and index + 2 < len(tokens):
            if tokens[index + 1].text == "!" and tokens[index + 2].text in OPENERS:
                if _has_coverage_name(tokens, index + 3, pairs[index + 2]):
                    return True
        macro = macro_stops.get(index)
        prefix = production if source_is_production and not test_only else always
        if macro is not None:
            macro_stop, definition = macro
            dangerous = prefix[macro_stop] - prefix[index + 1]
            if definition and prefix is production: dangerous -= safe_tests[macro_stop] - safe_tests[index + 1]
            if dangerous: return True
    return False
def detect_coverage_control(
    data: bytes, source_path: str = "src/lib.rs",
) -> bool:
    """Detect suppression and source-exclusion controls."""
    tokens = _tokens(data)
    pairs = _pairs(tokens)
    spans = _test_only_token_spans(tokens, pairs, strict=False)
    return _coverage_control(tokens, pairs, spans, source_path)
def _possibilities(tokens: list[Token], pairs: dict[int, int], start: int, stop: int) -> tuple[bool, bool]:
    work: list[tuple[int, int] | tuple[str, int]] = [(start, stop)]
    values: list[tuple[bool, bool]] = []
    while work:
        frame = work.pop()
        if isinstance(frame[0], str):
            name, count = frame
            arguments = values[-count:] if count else []
            if count: del values[-count:]
            if name == "all":
                value = any(item[0] for item in arguments), all(item[1] for item in arguments)
            elif name == "any":
                value = all(item[0] for item in arguments), any(item[1] for item in arguments)
            elif name == "not" and count == 1:
                value = arguments[0][1], arguments[0][0]
            else:
                value = True, True
            values.append(value)
            continue
        begin, end = frame
        if begin >= end or tokens[begin].literal:
            values.append((True, True))
            continue
        name = _identifier(tokens[begin].text)
        cursor = begin + 1
        if cursor < end and tokens[cursor].text == "=":
            values.append((True, True))
            continue
        if cursor >= end or tokens[cursor].text != "(" or pairs[cursor] >= end:
            values.append((True, False) if name == "test" else (True, True))
            continue
        segments = _top_level_comma_segments(tokens, pairs, cursor + 1, pairs[cursor])
        work.append((name, len(segments)))
        work.extend(reversed(segments))
    return values.pop()
def _checked_possibilities(tokens: list[Token], pairs: dict[int, int], start: int, stop: int) -> tuple[bool, bool]:
    value = _possibilities(tokens, pairs, start, stop)
    if value == (True, True) and any(_identifier(token.text) == "test" for token in tokens[start:stop]):
        raise SourcePolicyError("nontrivial cfg involving test is unsupported")
    return value
def _meta_test_only(tokens: list[Token], pairs: dict[int, int], start: int, stop: int) -> bool:
    work: list[tuple[int, int] | tuple[tuple[bool, bool], int]] = [(start, stop)]
    values: list[bool] = []
    while work:
        frame = work.pop()
        if isinstance(frame[0], tuple):
            condition, count = frame
            nested = any(values[-count:]) if count else False
            if count: del values[-count:]
            if nested and condition == (True, True):
                raise SourcePolicyError("cfg_attr conditionally applies a test-only attribute")
            values.append(nested and not condition[0])
            continue
        begin, end = frame
        if begin >= end:
            values.append(False)
            continue
        name = _identifier(tokens[begin].text)
        path_end = begin
        while path_end < end and tokens[path_end].text not in ("(", "="): path_end += 1
        if path_end > begin and _identifier(tokens[path_end - 1].text) in ("test", "bench"):
            if path_end != begin + 1:
                raise SourcePolicyError("namespaced test attributes are unsupported")
            values.append(True)
            continue
        if begin + 1 >= end or tokens[begin + 1].text != "(" or pairs[begin + 1] >= end:
            values.append(False)
            continue
        close = pairs[begin + 1]
        if name == "cfg":
            values.append(not _checked_possibilities(tokens, pairs, begin + 2, close)[1])
            continue
        comma = _first_top_level_comma(tokens, pairs, begin + 2, close)
        if name != "cfg_attr" or comma >= close:
            values.append(False)
            continue
        condition = _checked_possibilities(tokens, pairs, begin + 2, comma)
        if not condition[1]:
            values.append(False)
            continue
        segments = _top_level_comma_segments(tokens, pairs, comma + 1, close)
        work.append((condition, len(segments)))
        work.extend(reversed(segments))
    return values.pop()
def _item_target(tokens: list[Token], pairs: dict[int, int], start: int) -> bool:
    cursor = start
    if cursor < len(tokens) and tokens[cursor].text == "pub":
        cursor += 1
        if cursor < len(tokens) and tokens[cursor].text == "(": cursor = pairs[cursor] + 1
    while cursor < len(tokens) and tokens[cursor].text in ("async", "unsafe", "default", "auto"):
        cursor += 1
    if cursor < len(tokens) and tokens[cursor].text == "extern":
        cursor += 1
        if cursor < len(tokens) and tokens[cursor].literal: cursor += 1
        return cursor < len(tokens) and tokens[cursor].text in ("fn", "crate", "{")
    return cursor < len(tokens) and tokens[cursor].text in ITEM_TARGETS
def _generic_close(
    tokens: list[Token], pairs: dict[int, int], start: int, allow_comma: bool, trusted: bool = False,
) -> tuple[int | None, int]:
    depth = 1
    cursor = start + 1
    while cursor < len(tokens):
        text = tokens[cursor].text
        if text in OPENERS:
            cursor = pairs[cursor] + 1
            continue
        if text in CLOSERS or text == ";" or (
            text == "=" and cursor + 1 < len(tokens) and tokens[cursor + 1].text == ">"
        ):
            return None, cursor
        if text == "," and depth == 1 and not allow_comma:
            return None, cursor
        if text == "<":
            depth += 1
        elif text == ">" and tokens[cursor - 1].text not in ("=", "-"):
            depth -= 1
            if not depth:
                following = tokens[cursor + 1].text if cursor + 1 < len(tokens) else ";"
                return (cursor, cursor + 1) if trusted or following in AFTER_GENERIC else (None, cursor + 1)
        cursor += 1
    return None, cursor
def _generic_ranges(tokens: list[Token], pairs: dict[int, int]) -> list[tuple[int, int]]:
    result: list[tuple[int, int]] = []
    for index, token in enumerate(tokens):
        if token.text != "<" or not index:
            continue
        declaration = tokens[index - 1].text in ("impl", "for") or (
            index >= 2 and tokens[index - 2].text in GENERIC_DECLARATIONS
        )
        if declaration:
            close, _retry = _generic_close(tokens, pairs, index, True, True)
            if close is not None: result.append((index, close))
    return result
def _target_end(
    tokens: list[Token], pairs: dict[int, int], start: int, generic_end: int | None,
) -> int:
    item_target = _item_target(tokens, pairs, start)
    statement_target, const_target = start < len(tokens) and tokens[start].text == "let", start < len(tokens) and tokens[start].text == "const"
    assigned, retry_generic, cursor = False, start, start
    while cursor < len(tokens):
        text = tokens[cursor].text
        if cursor == generic_end:
            if cursor == start: raise SourcePolicyError("attribute has no syntactic target")
            return cursor - 1
        if text in CLOSERS:
            if cursor == start:
                raise SourcePolicyError("attribute has no syntactic target")
            return cursor - 1
        turbofish = (
            cursor >= start + 2 and tokens[cursor - 1].text == tokens[cursor - 2].text == ":"
        )
        if text == "<" and (cursor >= retry_generic or turbofish):
            close, retry_generic = _generic_close(
                tokens, pairs, cursor,
                turbofish or generic_end is not None and (not const_target or not assigned),
            )
            if close is not None:
                cursor = close + 1
                continue
        if text == "=" and (cursor + 1 >= len(tokens) or tokens[cursor + 1].text != ">"):
            assigned = True
        if text in OPENERS:
            close = pairs[cursor]
            if text == "{":
                following = close + 1
                if following < len(tokens) and tokens[following].text == "else":
                    cursor = following + 1
                    continue
                if following < len(tokens) and tokens[following].text in (";", ","):
                    return following
                starts_statement = following < len(tokens) and (
                    tokens[following].text in {"-", "*", "&", "!", "|", "||", "<"}
                    or tokens[following].text in (".", ":") and following + 1 < len(tokens)
                    and tokens[following + 1].text == tokens[following].text
                )
                if starts_statement or following >= len(tokens) or tokens[following].text not in CONTINUE_AFTER_GROUP:
                    return close
            cursor = close + 1
            continue
        if text == ";" or text == "," and not item_target and not statement_target:
            return cursor
        cursor += 1
    raise SourcePolicyError("attribute target is unterminated")
def _test_only_token_spans(
    tokens: list[Token], pairs: dict[int, int], strict: bool,
) -> list[tuple[int, int]]:
    macro_ranges, generic_ranges = _macro_ranges(tokens, pairs), _generic_ranges(tokens, pairs); macro_delta = [0] * (len(tokens) + 1)
    spans: list[tuple[int, int]] = []; active_generic_ends: list[int] = []
    macro_range_index = generic_range_index = index = 0
    for start, stop, _definition in macro_ranges: macro_delta[start + 1] += 1; macro_delta[stop] -= 1
    closure_starts: dict[int, int] = {}; closure_ends: dict[int, int] = {}; active: dict[int, tuple[int, bool]] = {}; pending: dict[int, list[tuple[int, int]]] = {}; attribute_stops: list[int] = []; depth = macro_level = 0
    for cursor, token in enumerate(tokens):
        macro_level += macro_delta[cursor]; previous = tokens[cursor - 1] if cursor else None; depth -= token.text in CLOSERS
        if token.text == "[" and cursor and (tokens[cursor - 1].text == "#" or cursor >= 2 and tokens[cursor - 1].text == "!" and tokens[cursor - 2].text == "#"): attribute_stops.append(pairs[cursor])
        if cursor > 0 and tokens[cursor - 1].text == "=" and token.text == ">" and not macro_level and not attribute_stops: active.pop(depth, None); pending.pop(depth, None)
        if token.text == "|" and not macro_level and not attribute_stops and depth in active:
            start, ambiguous = active.pop(depth)
            if ambiguous: pending.setdefault(depth, []).append((start, cursor))
            else: closure_starts[start + 1] = cursor
        elif token.text == "|" and not macro_level and not attribute_stops and (not cursor or previous.text not in (")", "]") and previous.text != "?" and not previous.literal and (not _identifier(previous.text).isidentifier() or previous.text in ("async", "become", "box", "break", "const", "match", "move", "return", "unsafe", "yield") or cursor >= 3 and tokens[cursor - 2].text == "'" and tokens[cursor - 3].text == "break")):
            active[depth] = (cursor, previous is not None and (previous.text in OPENERS or previous.text in (",", "let", "for")))
        if token.text == ";":
            for accepted in pending.pop(depth, ()): closure_starts[accepted[0] + 1] = accepted[1]
            active.pop(depth, None)
        if token.text in CLOSERS:
            for accepted in pending.pop(depth + 1, ()): closure_starts[accepted[0] + 1] = accepted[1]
            active.pop(depth + 1, None)
        if attribute_stops and cursor == attribute_stops[-1]: attribute_stops.pop()
        depth += token.text in OPENERS
    active_closure_ends: list[int] = []
    for cursor, token in enumerate(tokens):
        while active_closure_ends and active_closure_ends[-1] <= cursor: active_closure_ends.pop()
        if cursor in closure_starts: active_closure_ends.append(closure_starts[cursor])
        if token.text == "#" and active_closure_ends: closure_ends[cursor] = active_closure_ends[-1]
    while index < len(tokens):
        attribute = _attribute(tokens, pairs, index)
        if attribute is None:
            index += 1
            continue
        span_start = index
        cursor = index
        test_only = False
        macro_safe = True
        while attribute is not None:
            start, stop, inner = attribute
            current = False
            if not test_only:
                try:
                    current = _meta_test_only(tokens, pairs, start, stop)
                except SourcePolicyError:
                    if strict:
                        raise
            if current: macro_safe &= stop == start + 1 and _identifier(tokens[start].text) in ("test", "bench")
            if current and inner:
                if strict: raise SourcePolicyError("test-only inner cfg attribute is unsupported")
                current = False
            test_only |= current
            cursor = stop + 1
            attribute = None if inner or cursor >= len(tokens) else _attribute(tokens, pairs, cursor)
        index = cursor
        if not test_only:
            continue
        while active_generic_ends and active_generic_ends[-1] < span_start:
            active_generic_ends.pop()
        while (
            generic_range_index < len(generic_ranges)
            and generic_ranges[generic_range_index][0] < span_start
        ):
            generic_end = generic_ranges[generic_range_index][1]
            if generic_end > span_start: active_generic_ends.append(generic_end)
            generic_range_index += 1
        while macro_range_index < len(macro_ranges) and macro_ranges[macro_range_index][1] < span_start:
            macro_range_index += 1
        if macro_range_index < len(macro_ranges) and macro_ranges[macro_range_index][0] < span_start:
            if strict and not (macro_safe and macro_ranges[macro_range_index][2]): raise SourcePolicyError("test attributes inside macros are unsupported")
            continue
        try:
            target_end = _target_end(
                tokens, pairs, cursor,
                closure_ends.get(span_start, active_generic_ends[-1] if active_generic_ends else None),
            )
        except SourcePolicyError:
            if strict: raise
            continue
        spans.append((span_start, target_end))
        index = target_end + 1
    return spans
def analyze_rust_source(
    data: bytes, source_path: str = "src/lib.rs",
) -> SourcePolicy:
    tokens = _tokens(data)
    pairs = _pairs(tokens)
    merged = _test_only_token_spans(tokens, pairs, strict=True)
    candidates: set[int] = set()
    active: set[int] = set()
    for start, stop in merged:
        candidates.update(range(tokens[start].line, tokens[stop].end_line + 1))
    span_index = 0
    for index, token in enumerate(tokens):
        while span_index < len(merged) and merged[span_index][1] < index:
            span_index += 1
        if token.literal:
            lines = range(token.line, token.end_line + 1)
        else:
            lines = (token.line,)
        if span_index >= len(merged) or not merged[span_index][0] <= index <= merged[span_index][1]:
            active.update(lines)
    if candidates & active:
        raise SourcePolicyError("test-only and production tokens share one source line")
    return SourcePolicy(
        frozenset(candidates - active),
        _coverage_control(tokens, pairs, merged, source_path),
    )
