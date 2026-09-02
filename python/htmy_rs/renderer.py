from __future__ import annotations

from collections import ChainMap
from typing import TYPE_CHECKING

from anyio import create_task_group

from htmy_rs._native import RenderSession as RenderSession

# This module must not import `htmy` at module level: `htmy` optionally imports the
# default renderer from here, so a module level `htmy` import would deadlock the
# two packages' initialization when `htmy_rs` is imported first.


def _default_string_formatter(value: str) -> str:
    """Placeholder default `string_formatter` that resolves to the real default lazily."""
    return value


if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

    from htmy.typing import Component, Context


def _cancel_pending(pending: list[Awaitable[object]]) -> None:
    for awaitable in pending:
        close = getattr(awaitable, "close", None)
        if close is not None:
            close()


async def _store(results: list[object], i: int, awaitable: Awaitable[object]) -> None:
    results[i] = await awaitable


class Renderer:
    """
    Native default renderer.

    Same public API as `htmy.renderer.default.Renderer`.
    """

    __slots__ = ("_default_context", "_string_formatter")

    def __init__(
        self,
        default_context: Context | None = None,
        *,
        string_formatter: Callable[[str], str] = _default_string_formatter,
    ) -> None:
        if string_formatter is _default_string_formatter:
            from htmy.core import xml_format_string

            string_formatter = xml_format_string
        self._default_context: Context = {} if default_context is None else default_context
        self._string_formatter = string_formatter

    async def render(self, component: Component, context: Context | None = None) -> str:
        from htmy.renderer.context import RendererContext

        default_context = {**self._default_context, RendererContext: self}
        context = default_context if context is None else ChainMap(context, default_context)  # type: ignore[arg-type]

        session = RenderSession(component, context, self._string_formatter)
        try:
            pending = session.pending()
            while pending:
                results: list[object] = [None] * len(pending)
                try:
                    async with create_task_group() as tg:
                        for i, awaitable in enumerate(pending):
                            tg.start_soon(_store, results, i, awaitable)
                except BaseException:
                    _cancel_pending(pending)
                    raise
                pending = session.submit(results)
            return session.output()
        except BaseException:
            session.cancel()
            raise
