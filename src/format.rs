use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PySet, PyString, PyTuple};

use crate::types;

/// Default property-name conversion. Copied from `htmy.core.Formatter._format_name`.
pub fn format_name(name: &str) -> String {
    let Some(first) = name.chars().next() else {
        return String::new();
    };
    let last = name.chars().next_back().unwrap_or(first);
    let no_replacement = first == '_' || last == '_';
    if no_replacement {
        name.trim_matches('_').to_string()
    } else {
        name.replace('_', "-")
    }
}

/// Escape `&`, `>`, `<` — same order as `xml.sax.saxutils.escape`.
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '>' => out.push_str("&gt;"),
            '<' => out.push_str("&lt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Same quoting as `xml.sax.saxutils.quoteattr`.
pub fn quoteattr(s: &str) -> String {
    let escaped = xml_escape(s)
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
        .replace('\t', "&#9;");
    if escaped.contains('"') {
        if escaped.contains('\'') {
            format!("\"{}\"", escaped.replace('"', "&quot;"))
        } else {
            format!("'{escaped}'")
        }
    } else {
        format!("\"{escaped}\"")
    }
}

/// Format one attribute. `None` means skip (emit empty string in the join).
pub fn format_attr(
    py: Python<'_>,
    name: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<String>> {
    match format_value(py, value)? {
        None => Ok(None),
        Some(v) => Ok(Some(format!("{}={}", format_name(name), quoteattr(&v)))),
    }
}

/// Default value rules. Exact type only, same as Python `Formatter`.
/// `None` means skip the property.
pub fn format_value(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
    if value.is_none() {
        return Ok(None);
    }

    let t = types();

    if value.get_type().is(t.xbool.bind(py)) {
        if value.is(t.xbool_true.bind(py)) {
            return Ok(Some(String::new()));
        }
        return Ok(None);
    }

    if value.is_exact_instance_of::<PyBool>() {
        let b = value.cast::<PyBool>()?;
        return Ok(Some(if b.is_true() { "true" } else { "false" }.to_string()));
    }

    if value.get_type().is(t.datetime.bind(py)) || value.get_type().is(t.date.bind(py)) {
        let s: String = value.call_method0("isoformat")?.extract()?;
        return Ok(Some(s));
    }

    if value.is_exact_instance_of::<PyDict>()
        || value.is_exact_instance_of::<PyList>()
        || value.is_exact_instance_of::<PyTuple>()
    {
        let s: String = t.json_dumps.bind(py).call1((value,))?.extract()?;
        return Ok(Some(s));
    }

    if value.is_exact_instance_of::<PySet>() {
        let set = value.cast::<PySet>()?;
        let tup = PyTuple::new(py, set.iter())?;
        let s: String = t.json_dumps.bind(py).call1((tup,))?.extract()?;
        return Ok(Some(s));
    }

    if value.is_exact_instance_of::<PyString>() {
        return Ok(Some(value.extract()?));
    }

    Ok(Some(value.str()?.to_string()))
}

pub fn write_props(
    buf: &mut String,
    py: Python<'_>,
    props: &[(String, Py<PyAny>)],
    python_formatter: Option<&Bound<'_, PyAny>>,
) -> PyResult<()> {
    for (i, (name, value)) in props.iter().enumerate() {
        if i > 0 {
            buf.push(' ');
        }
        let value = value.bind(py);
        if let Some(fmt) = python_formatter {
            let formatted: String = fmt.call_method1("format", (name, value))?.extract()?;
            buf.push_str(&formatted);
        } else if let Some(formatted) = format_attr(py, name, value)? {
            buf.push_str(&formatted);
        }
    }
    Ok(())
}
