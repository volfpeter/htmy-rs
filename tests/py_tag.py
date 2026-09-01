"""Python tag implementations copied from htmy, for htmy() fallback comparison."""

from __future__ import annotations

from typing import TYPE_CHECKING

from htmy.core import Formatter, SafeStr
from htmy.utils import join_components

if TYPE_CHECKING:
    from htmy.typing import Component, ComponentSequence, ComponentType, Context, Properties


class TagWithPropsImpl:
    __slots__ = ("name", "props")

    def __init__(self, name: str, props: Properties) -> None:
        self.name = name
        self.props = props

    def htmy(self, context: Context) -> ComponentType:
        formatter: Formatter = context.get(Formatter, _default_formatter)
        return SafeStr(f"<{self.name} {' '.join(formatter.format(n, v) for n, v in self.props.items())}/>")


class TagImpl:
    __slots__ = ("child_separator", "children", "name", "props")

    def __init__(
        self,
        name: str,
        props: Properties,
        children: ComponentSequence,
        child_separator: ComponentType,
    ) -> None:
        self.name = name
        self.props = props
        self.children = children
        self.child_separator = child_separator

    def htmy(self, context: Context) -> Component:
        formatter: Formatter = context.get(Formatter, _default_formatter)
        name = self.name
        return (
            SafeStr(f"<{name} {' '.join(formatter.format(n, v) for n, v in self.props.items())}>"),
            *(
                self.children
                if self.child_separator is None
                else join_components(self.children, separator=self.child_separator, pad=True)
            ),
            SafeStr(f"</{name}>"),
        )


_default_formatter = Formatter()
