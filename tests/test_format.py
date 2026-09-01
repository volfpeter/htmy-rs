from datetime import date, datetime
from xml.sax.saxutils import quoteattr as py_quoteattr

import pytest
from htmy import Formatter, SafeStr, XBool, xml_format_string
from htmy_rs._native import format_attr, format_name, quoteattr, xml_escape_text


@pytest.mark.parametrize(
    ("name", "expected"),
    (
        ("class_", "class"),
        ("hx_boost", "hx-boost"),
        ("_foo_", "foo"),
        ("_foo", "foo"),
        ("foo_", "foo"),
        ("id", "id"),
        ("data_value", "data-value"),
        ("hx_on__click", "hx-on--click"),
    ),
)
def test_format_name(name: str, expected: str) -> None:
    assert format_name(name) == expected
    assert format_name(name) == Formatter().format_name(name)


@pytest.mark.parametrize(
    "value",
    (
        "",
        "hello",
        "a<b>c&d",
        'he said "hi"',
        "it's",
        "he said \"hi\" and 'bye'",
        "line\nbreak\ttab\rcr",
        "a&b",
    ),
)
def test_quoteattr(value: str) -> None:
    assert quoteattr(value) == py_quoteattr(value)


@pytest.mark.parametrize(
    "value",
    ("", "hello", "a<b>c&d", "a&b", "plain"),
)
def test_xml_escape_text(value: str) -> None:
    assert xml_escape_text(value) == xml_format_string(value)
    assert xml_format_string(SafeStr(value)) == value


@pytest.mark.parametrize(
    ("name", "value"),
    (
        ("class_", "btn"),
        ("hx_boost", True),
        ("checked", XBool.true),
        ("checked", XBool.false),
        ("hidden", None),
        ("flag", False),
        ("count", 3),
        ("when", date(2024, 1, 2)),
        ("when", datetime(2024, 1, 2, 3, 4, 5)),
        ("data", {"a": 1, "b": "x"}),
        ("items", [1, "a"]),
        ("items", (1, "a")),
        ("empty", {}),
        ("empty", []),
        ("empty", ()),
    ),
)
def test_format_attr_matches_formatter(name: str, value: object) -> None:
    assert format_attr(name, value) == Formatter().format(name, value)


def test_format_attr_set() -> None:
    # Set iteration order is not stable; compare via Formatter on the same value.
    value = {"c0ff33"}
    rust = format_attr("items", value)
    py = Formatter().format("items", value)
    assert rust == py
