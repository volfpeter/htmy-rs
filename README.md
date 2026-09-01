# htmy-rs

Native accelerator for [htmy](https://github.com/volfpeter/htmy).

When installed, this library provides replacements for the renderer and tag internals of `htmy`, which are automatically picked up and used by `htmy`, without any code changes in user code.

Important: this is not a Rust rewrite of `htmy`, just an optional accelerator.

## Installation

```bash
pip install htmy-rs
```

## Async compatibility

Async features stay in Python and are handled via `anyio`, so both `asyncio` and `trio` are supported.

## License - MIT

The package is open-sourced under the conditions of the [MIT license](https://choosealicense.com/licenses/mit/).
