use pyo3::prelude::*;
use pyo3::types::PyModule;

mod format;
mod intern;
mod render;
mod tag;

/// The Python objects this module interoperates with.
#[derive(Clone)]
pub(crate) struct PyTypes {
    pub safestr: Py<PyAny>,
    pub xbool: Py<PyAny>,
    pub xbool_true: Py<PyAny>,
    pub formatter: Py<PyAny>,
    pub xml_format_string: Py<PyAny>,
    pub date: Py<PyAny>,
    pub datetime: Py<PyAny>,
    pub json_dumps: Py<PyAny>,
    pub chainmap: Py<PyAny>,
    pub isawaitable: Py<PyAny>,
}

/// Name of the module attribute that stores the resolved [`PyTypes`].
const TYPES_ATTR: &str = "_types";

/// Opaque capsule that exposes [`PyTypes`] to Python as a single module attribute.
#[pyclass(module = "htmy_rs", name = "_Types")]
struct TypesCell {
    inner: PyTypes,
}

fn resolve_types(py: Python<'_>) -> PyResult<PyTypes> {
    let htmy = py.import("htmy.core")?;
    let xbool_true = htmy.getattr("XBool")?.getattr("true")?.unbind();
    let datetime_mod = py.import("datetime")?;
    Ok(PyTypes {
        safestr: htmy.getattr("SafeStr")?.unbind(),
        xbool: htmy.getattr("XBool")?.unbind(),
        xbool_true,
        formatter: htmy.getattr("Formatter")?.unbind(),
        xml_format_string: htmy.getattr("xml_format_string")?.unbind(),
        date: datetime_mod.getattr("date")?.unbind(),
        datetime: datetime_mod.getattr("datetime")?.unbind(),
        json_dumps: py.import("json")?.getattr("dumps")?.unbind(),
        chainmap: py.import("collections")?.getattr("ChainMap")?.unbind(),
        isawaitable: py.import("inspect")?.getattr("isawaitable")?.unbind(),
    })
}

fn module_types(m: &Bound<'_, PyModule>) -> PyResult<PyTypes> {
    let cell = m.getattr(TYPES_ATTR)?.cast_into::<TypesCell>()?;
    Ok(cell.borrow().inner.clone())
}

pub(crate) fn types(py: Python<'_>) -> PyResult<PyTypes> {
    module_types(&py.import("htmy_rs._native")?)
}

#[pyfunction]
fn format_name(name: &str) -> String {
    format::format_name(name)
}

#[pyfunction]
fn xml_escape_text(s: &str) -> String {
    format::xml_escape(s)
}

#[pyfunction]
fn quoteattr(s: &str) -> String {
    format::quoteattr(s)
}

#[pyfunction]
#[pyo3(pass_module)]
fn format_attr(m: &Bound<'_, PyModule>, name: &str, value: Bound<'_, PyAny>) -> PyResult<String> {
    match format::format_attr(m.py(), &module_types(m)?, name, &value)? {
        Some(s) => Ok(s),
        None => Ok(String::new()),
    }
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let t = resolve_types(py)?;
    let cell = Py::new(py, TypesCell { inner: t })?;
    m.add(TYPES_ATTR, cell)?;
    m.add_class::<tag::TagImpl>()?;
    m.add_class::<tag::TagWithPropsImpl>()?;
    m.add_class::<render::RenderSession>()?;
    m.add_function(wrap_pyfunction!(format_name, m)?)?;
    m.add_function(wrap_pyfunction!(xml_escape_text, m)?)?;
    m.add_function(wrap_pyfunction!(quoteattr, m)?)?;
    m.add_function(wrap_pyfunction!(format_attr, m)?)?;
    Ok(())
}
