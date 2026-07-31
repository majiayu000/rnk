"""Token-based Rust source policy for production coverage accounting."""

from __future__ import annotations

from dataclasses import dataclass


class SourcePolicyError(ValueError):
    """The immutable Rust source cannot be classified safely."""


@dataclass(frozen=True)
class Token:
    text: str
    line: int
    end_line: int
    literal: bool = False


@dataclass(frozen=True)
class SourcePolicy:
    test_only_lines: frozenset[int]
    coverage_control: bool


OPENERS = {"(": ")", "[": "]", "{": "}"}
CLOSERS = {value: key for key, value in OPENERS.items()}
CONTINUE_AFTER_GROUP = {
    ".", "?", "+", "-", "*", "/", "%", "&", "|", "^", "!", "=", "<", ">", ":",
}
ITEM_TARGETS = {"fn", "mod", "struct", "enum", "union", "impl", "trait", "type", "static", "use", "macro", "macro_rules"}


def _identifier(text: str) -> str:
    return text[2:] if text.startswith("r#") else text


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
        return end, line + source[start:end].count("\n")
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
        return (cursor + 1, line) if cursor < len(source) and source[cursor] == "'" else None
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
            return cursor + 1, end_line
        cursor += 1
    raise SourcePolicyError("unterminated string literal")


def _tokens(data: bytes) -> list[Token]:
    try:
        source = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise SourcePolicyError("Rust source is not UTF-8") from error
    result: list[Token] = []
    index = 0
    line = 1
    while index < len(source):
        character = source[index]
        if character.isspace():
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
        if parsed is not None:
            end, end_line = parsed
            result.append(Token("<literal>", line, end_line, True))
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


def _coverage_control(tokens: list[Token], pairs: dict[int, int]) -> bool:
    for index, token in enumerate(tokens):
        if _identifier(token.text) in ("coverage", "coverage_nightly"):
            return True
        attribute = _attribute(tokens, pairs, index)
        if attribute is not None and any(_identifier(item.text) in ("automatically_derived", "naked") for item in tokens[attribute[0]:attribute[1]]):
            return True
    return False


def detect_coverage_control(data: bytes) -> bool:
    """Detect suppression tokens without classifying cfg(test) target spans."""
    tokens = _tokens(data)
    return _coverage_control(tokens, _pairs(tokens))


def _possibilities(tokens: list[Token], pairs: dict[int, int], start: int, stop: int) -> tuple[bool, bool]:
    if start >= stop or tokens[start].literal:
        return True, True
    name = _identifier(tokens[start].text)
    cursor = start + 1
    if cursor < stop and tokens[cursor].text == "=":
        return True, True
    if cursor >= stop or tokens[cursor].text != "(" or pairs[cursor] >= stop:
        return (True, False) if name == "test" else (True, True)
    close = pairs[cursor]
    arguments: list[tuple[bool, bool]] = []
    item = cursor + 1
    while item < close:
        end = item
        while end < close:
            if tokens[end].text in OPENERS:
                end = pairs[end] + 1
                continue
            if tokens[end].text == ",":
                break
            end += 1
        arguments.append(_possibilities(tokens, pairs, item, end))
        item = end + 1
    if name == "all":
        return any(value[0] for value in arguments), all(value[1] for value in arguments)
    if name == "any":
        return all(value[0] for value in arguments), any(value[1] for value in arguments)
    if name == "not" and len(arguments) == 1:
        return arguments[0][1], arguments[0][0]
    return True, True


def _checked_possibilities(tokens: list[Token], pairs: dict[int, int], start: int, stop: int) -> tuple[bool, bool]:
    value = _possibilities(tokens, pairs, start, stop)
    if value == (True, True) and any(_identifier(token.text) == "test" for token in tokens[start:stop]):
        raise SourcePolicyError("nontrivial cfg involving test is unsupported")
    return value


def _meta_test_only(tokens: list[Token], pairs: dict[int, int], start: int, stop: int) -> bool:
    if start >= stop:
        return False
    name = _identifier(tokens[start].text)
    path_end = start
    while path_end < stop and tokens[path_end].text not in ("(", "="):
        path_end += 1
    if path_end > start and _identifier(tokens[path_end - 1].text) in ("test", "bench"):
        if path_end == start + 1:
            return True
        raise SourcePolicyError("namespaced test attributes are unsupported")
    if start + 1 >= stop or tokens[start + 1].text != "(":
        return False
    close = pairs[start + 1]
    if close >= stop:
        return False
    if name == "cfg":
        return not _checked_possibilities(tokens, pairs, start + 2, close)[1]
    if name != "cfg_attr":
        return False
    cursor = start + 2
    comma = cursor
    while comma < close:
        if tokens[comma].text in OPENERS:
            comma = pairs[comma] + 1
            continue
        if tokens[comma].text == ",":
            break
        comma += 1
    condition = _checked_possibilities(tokens, pairs, cursor, comma)
    if condition[0] or comma >= close:
        return False
    item = comma + 1
    while item < close:
        end = item
        while end < close:
            if tokens[end].text in OPENERS:
                end = pairs[end] + 1
                continue
            if tokens[end].text == ",":
                break
            end += 1
        if _meta_test_only(tokens, pairs, item, end):
            return True
        item = end + 1
    return False


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
    if cursor < len(tokens) and tokens[cursor].text == "const":
        return True
    return cursor < len(tokens) and tokens[cursor].text in ITEM_TARGETS


def _target_end(tokens: list[Token], pairs: dict[int, int], start: int) -> int:
    item_target = _item_target(tokens, pairs, start)
    statement_target = start < len(tokens) and tokens[start].text == "let"
    angles = 0
    cursor = start
    while cursor < len(tokens):
        text = tokens[cursor].text
        if text in CLOSERS:
            if angles: raise SourcePolicyError("ambiguous angle brackets in attribute target")
            if cursor == start:
                raise SourcePolicyError("attribute has no syntactic target")
            return cursor - 1
        if text == "<":
            angles += 1
            cursor += 1
            continue
        if text == ">" and angles:
            angles -= 1
            cursor += 1
            continue
        if text in OPENERS:
            close = pairs[cursor]
            if text == "{":
                following = close + 1
                if following < len(tokens) and tokens[following].text == "else":
                    cursor = following + 1
                    continue
                if following < len(tokens) and tokens[following].text in (";", ","):
                    return following
                if following >= len(tokens) or tokens[following].text not in CONTINUE_AFTER_GROUP:
                    return close
            cursor = close + 1
            continue
        if text == ";" or text == "," and not item_target and not statement_target and not angles:
            return cursor
        cursor += 1
    raise SourcePolicyError("attribute target is unterminated")


def analyze_rust_source(data: bytes) -> SourcePolicy:
    tokens = _tokens(data)
    pairs = _pairs(tokens)
    macro_ranges: list[tuple[int, int]] = []
    for index, token in enumerate(tokens):
        if token.text != "!" or not index or not (_identifier(tokens[index - 1].text)[:1].isalpha() or tokens[index - 1].text.startswith(("_", "r#"))):
            continue
        cursor = index + 1
        if cursor < len(tokens) and tokens[cursor].text not in OPENERS: cursor += 1
        if cursor < len(tokens) and tokens[cursor].text in OPENERS: macro_ranges.append((cursor, pairs[cursor]))
    spans: list[tuple[int, int]] = []
    for index, token in enumerate(tokens):
        attribute = _attribute(tokens, pairs, index)
        if attribute is not None:
            start, stop, inner = attribute
            if _meta_test_only(tokens, pairs, start, stop):
                if any(begin < index < end for begin, end in macro_ranges):
                    raise SourcePolicyError("test attributes inside macros are unsupported")
                if inner:
                    raise SourcePolicyError("test-only inner cfg attribute is unsupported")
                cursor = stop + 1
                while cursor < len(tokens):
                    stacked = _attribute(tokens, pairs, cursor)
                    if stacked is None or stacked[2]:
                        break
                    cursor = stacked[1] + 1
                spans.append((index, _target_end(tokens, pairs, cursor)))
    merged: list[tuple[int, int]] = []
    for start, stop in sorted(spans):
        if merged and start <= merged[-1][1] + 1:
            merged[-1] = (merged[-1][0], max(stop, merged[-1][1]))
        else:
            merged.append((start, stop))
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
    return SourcePolicy(frozenset(candidates - active), _coverage_control(tokens, pairs))
