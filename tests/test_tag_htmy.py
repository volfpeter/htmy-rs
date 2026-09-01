from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from htmy import Formatter, SafeStr, XBool
from htmy.renderer.default import Renderer as PythonRenderer
from htmy_rs.tag import TagImpl, TagWithPropsImpl

from tests.py_tag import TagImpl as PyTagImpl
from tests.py_tag import TagWithPropsImpl as PyTagWithPropsImpl

if TYPE_CHECKING:
    from htmy.typing import Component, ComponentType, PropertyValue


def _void(name: str, **props: PropertyValue) -> tuple[TagWithPropsImpl, PyTagWithPropsImpl]:
    return TagWithPropsImpl(name, props), PyTagWithPropsImpl(name, props)


def _tag(
    name: str,
    *children: ComponentType,
    child_separator: ComponentType = "\n",
    **props: PropertyValue,
) -> tuple[TagImpl, PyTagImpl]:
    return (
        TagImpl(name, props, children, child_separator),
        PyTagImpl(name, props, children, child_separator),
    )


@pytest.mark.anyio
@pytest.mark.parametrize(
    "pair",
    (
        _void("img"),
        _void("img", src="x.png"),
        _void("input", type="checkbox", checked=XBool.true),
        _void("input", type="checkbox", checked=XBool.false),
        _void("meta", charset="utf-8", unused=None),
        _tag("div"),
        _tag("div", "hello"),
        _tag("div", "hello", class_="x"),
        _tag("div", "a", "b"),
        _tag("div", "a", None, "b"),
        _tag("span", "hello", child_separator=None),
        _tag("span", "a", "b", child_separator=None),
        _tag("p", SafeStr("<em>x</em>"), child_separator=None),
        _tag("p", "a<b>", child_separator=None),
        _tag("div", _tag("span", "x", child_separator=None)[0]),
    ),
)
async def test_htmy_fallback_matches_python_tag(pair: tuple[Component, Component]) -> None:
    rust, py = pair
    renderer = PythonRenderer()
    assert await renderer.render(rust) == await renderer.render(py)


@pytest.mark.anyio
async def test_nested_rust_tags_via_python_renderer() -> None:
    inner, _ = _tag("span", "x", child_separator=None, class_="i")
    outer, py_outer = _tag("div", inner, class_="o")
    py_inner = PyTagImpl("span", {"class_": "i"}, ("x",), None)
    py_outer = PyTagImpl("div", {"class_": "o"}, (py_inner,), "\n")
    renderer = PythonRenderer()
    assert await renderer.render(outer) == await renderer.render(py_outer)


@pytest.mark.anyio
async def test_custom_formatter_in_context() -> None:
    fmt = Formatter().add(int, lambda v: f"n{v}")
    rust, py = _tag("div", data_n=3)
    renderer = PythonRenderer()
    ctx = fmt.to_context()
    assert await renderer.render(rust, ctx) == await renderer.render(py, ctx)


@pytest.mark.anyio
async def test_empty_props_space() -> None:
    renderer = PythonRenderer()
    rust, py = _tag("div", "x", child_separator=None)
    assert await renderer.render(rust) == await renderer.render(py)
    assert "<div >" in await renderer.render(rust)
    void_r, void_p = _void("img")
    assert await renderer.render(void_r) == await renderer.render(void_p)
    assert await renderer.render(void_r) == "<img />"
