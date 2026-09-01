from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from htmy import ErrorBoundary, Fragment, SafeStr, WithContext, component, html
from htmy.renderer.baseline import Renderer as BaselineRenderer
from htmy.renderer.default import Renderer as PythonRenderer
from htmy_rs.renderer import Renderer as RustRenderer
from htmy_rs.tag import TagImpl, TagWithPropsImpl

if TYPE_CHECKING:
    from htmy import Component, ComponentType, Context


class WrapAsync:
    def __init__(self, *children: ComponentType) -> None:
        self.children = children

    async def htmy(self, context: Context) -> Component:
        return self.children


class Nested:
    def __init__(self, *children: ComponentType) -> None:
        self.children = children

    def htmy(self, context: Context) -> Component:
        return html.div(
            "Foo",
            html.div("bar"),
            Fragment(
                html.div(
                    WrapAsync("Before error", html.div(*self.children), "After error"),
                )
            ),
        )


def sync_async_divs(i: int) -> Fragment:
    return Fragment(html.div(f"Sync {i}", " ", "end"), WrapAsync(html.div(f"Async {i}", " ", "end")))


class SyncReturnsNone:
    def htmy(self, context: Context) -> Component:
        return None


class AsyncReturnsNone:
    async def htmy(self, context: Context) -> Component:
        return None


@component
def page(content: ComponentType, context: Context) -> Component:
    return (
        html.DOCTYPE.html,
        html.html(
            html.head(
                html.title("Test page"),
                html.Meta.charset(),
                None,
                SyncReturnsNone(),
                html.Meta.viewport(),
                None,
                None,
                html.script(src="https://cdn.tailwindcss.com"),
                SyncReturnsNone(),
                AsyncReturnsNone(),
                html.Link.css("https://cdn.jsdelivr.net/npm/daisyui@4.12.11/dist/full.min.css"),
            ),
            html.body(
                content,
                class_="h-screen w-screen",
            ),
            lang="en",
        ),
    )


class SyncError:
    def htmy(self, context: Context) -> Component:
        raise ValueError("sync-error-component")


class AsyncError:
    async def htmy(self, context: Context) -> Component:
        raise ValueError("async-error-component")


class SyncContextProvider:
    def __init__(self, *children: ComponentType) -> None:
        self.children = children

    def htmy_context(self) -> Context:
        return {"marker": "sync-provider"}

    def htmy(self, context: Context) -> Component:
        return (html.p("sync-provider", data_marker=context["marker"]), *self.children)


class AsyncContextProvider:
    def __init__(self, *children: ComponentType) -> None:
        self.children = children

    async def htmy_context(self) -> Context:
        return {"marker": "async-provider"}

    def htmy(self, context: Context) -> Component:
        return (html.p("async-provider", data_marker=context["marker"]), *self.children)


@component.context_only
def context_marker(context: Context) -> Component:
    return html.span("context-marker", data_marker=context.get("marker"))


async def _agree(component: Component) -> None:
    rust = await RustRenderer().render(component)
    python = await PythonRenderer().render(component)
    baseline = await BaselineRenderer().render(component)
    assert python == baseline
    assert rust == baseline


@pytest.mark.anyio
@pytest.mark.parametrize(
    ("component",),
    (
        ([Nested(sync_async_divs(i)) for i in range(20)],),
        (page(Fragment(*[Nested(sync_async_divs(i)) for i in range(20)])),),
        (Nested(ErrorBoundary(Nested(SyncError()), fallback="Fallback to sync error.")),),
        (Nested(ErrorBoundary(Nested(AsyncError()), fallback="Fallback to async error.")),),
        (
            Nested(
                WithContext(
                    SyncContextProvider(context_marker()),
                    AsyncContextProvider(context_marker()),
                    context_marker(),
                    context={"marker": "wrapped"},
                ),
                "escaped < text &",
                SafeStr("safe < text &"),
                None,
                Fragment(),
                WrapAsync(None, html.em("async child")),
                SyncReturnsNone(),
                AsyncReturnsNone(),
            ),
        ),
    ),
)
async def test_renderers_agree(component: Component) -> None:
    await _agree(component)


@pytest.mark.anyio
@pytest.mark.parametrize(
    ("component", "expected"),
    (
        (None, ""),
        (SyncReturnsNone(), ""),
        (AsyncReturnsNone(), ""),
        ((SyncReturnsNone(), "text", AsyncReturnsNone()), "text"),
    ),
)
async def test_none_components_render_nothing(component: Component, expected: str) -> None:
    for renderer in (RustRenderer(), PythonRenderer(), BaselineRenderer()):
        assert await renderer.render(component) == expected


@pytest.mark.anyio
async def test_native_tag_tree_agrees() -> None:
    tree = TagImpl(
        "div",
        {"class_": "wrap"},
        (
            TagImpl("span", {}, ("hello <",), None),
            TagWithPropsImpl("img", {"src": "x.png"}),
            None,
            "world",
            SafeStr("<br>"),
        ),
        "\n",
    )
    await _agree(tree)


@pytest.mark.anyio
async def test_sequence_child_is_invalid() -> None:
    # Sequences are only valid components at the top level or as component results;
    # every non-string child of a tag must be a component.
    tree = TagImpl("div", {}, (["a", "b"],), None)  # type: ignore[arg-type]
    for renderer in (RustRenderer(), PythonRenderer()):
        with pytest.raises(ValueError, match="Invalid component type"):
            await renderer.render(tree)


@pytest.mark.anyio
async def test_custom_string_formatter() -> None:
    def upper(s: str) -> str:
        return s.upper()

    component = TagImpl("span", {}, ("hello",), None)
    rust = await RustRenderer(string_formatter=upper).render(component)
    python = await PythonRenderer(string_formatter=upper).render(component)
    assert rust == python


@pytest.mark.anyio
async def test_invalid_type() -> None:
    with pytest.raises(ValueError, match="Invalid component type"):
        await RustRenderer().render(123)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="Invalid component type"):
        await PythonRenderer().render(123)  # type: ignore[arg-type]
