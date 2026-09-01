use std::sync::OnceLock;

use pyo3::prelude::*;
use pyo3::types::PyModule;

mod format;
mod intern;
mod render;
mod tag;

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

static TYPES: OnceLock<PyTypes> = OnceLock::new();

pub(crate) fn types() -> &'static PyTypes {
    TYPES.get().expect("htmy_rs.configure() was not called")
}

#[pyfunction]
fn configure(
    py: Python<'_>,
    safestr: Bound<'_, PyAny>,
    xbool: Bound<'_, PyAny>,
    formatter: Bound<'_, PyAny>,
    xml_format_string: Bound<'_, PyAny>,
) -> PyResult<()> {
    let datetime_mod = py.import("datetime")?;
    let types = PyTypes {
        xbool_true: xbool.getattr("true")?.unbind(),
        safestr: safestr.unbind(),
        xbool: xbool.unbind(),
        formatter: formatter.unbind(),
        xml_format_string: xml_format_string.unbind(),
        date: datetime_mod.getattr("date")?.unbind(),
        datetime: datetime_mod.getattr("datetime")?.unbind(),
        json_dumps: py.import("json")?.getattr("dumps")?.unbind(),
        chainmap: py.import("collections")?.getattr("ChainMap")?.unbind(),
        isawaitable: py.import("inspect")?.getattr("isawaitable")?.unbind(),
    };
    let _ = TYPES.set(types);
    Ok(())
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
fn format_attr(py: Python<'_>, name: &str, value: Bound<'_, PyAny>) -> PyResult<String> {
    match format::format_attr(py, name, &value)? {
        Some(s) => Ok(s),
        None => Ok(String::new()),
    }
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<tag::TagImpl>()?;
    m.add_class::<tag::TagWithPropsImpl>()?;
    m.add_class::<render::RenderSession>()?;
    m.add_function(wrap_pyfunction!(configure, m)?)?;
    m.add_function(wrap_pyfunction!(format_name, m)?)?;
    m.add_function(wrap_pyfunction!(xml_escape_text, m)?)?;
    m.add_function(wrap_pyfunction!(quoteattr, m)?)?;
    m.add_function(wrap_pyfunction!(format_attr, m)?)?;
    Ok(())
}
