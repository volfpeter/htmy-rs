use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

use crate::intern::{Child, Separator, intern_children, intern_separator};
use crate::types;

pub struct TagInner {
    pub name: String,
    pub props: Vec<(String, Py<PyAny>)>,
    pub children: Vec<Child>,
    pub separator: Separator,
    pub py_props: Py<PyDict>,
    pub py_children: Py<PyAny>,
    pub py_separator: Py<PyAny>,
}

pub struct VoidInner {
    pub name: String,
    pub props: Vec<(String, Py<PyAny>)>,
    pub py_props: Py<PyDict>,
}

fn props_vec(props: &Bound<'_, PyDict>) -> PyResult<Vec<(String, Py<PyAny>)>> {
    let mut out = Vec::with_capacity(props.len());
    for (k, v) in props.iter() {
        out.push((k.extract()?, v.unbind()));
    }
    Ok(out)
}

fn format_attrs(
    py: Python<'_>,
    context: &Bound<'_, PyAny>,
    props: &[(String, Py<PyAny>)],
) -> PyResult<String> {
    let formatter_cls = types().formatter.bind(py);
    let default = formatter_cls.call0()?;
    let formatter = context.call_method1("get", (formatter_cls, default))?;
    let mut parts = Vec::with_capacity(props.len());
    for (name, value) in props {
        let formatted: String = formatter
            .call_method1("format", (name, value.bind(py)))?
            .extract()?;
        parts.push(formatted);
    }
    Ok(parts.join(" "))
}

fn safestr(py: Python<'_>, s: String) -> PyResult<Py<PyAny>> {
    Ok(types().safestr.bind(py).call1((s,))?.unbind())
}

#[pyclass(module = "htmy_rs", name = "TagImpl")]
pub struct TagImpl {
    pub inner: Arc<TagInner>,
}

#[pymethods]
impl TagImpl {
    #[new]
    fn new(
        name: String,
        props: Bound<'_, PyDict>,
        children: Bound<'_, PyAny>,
        child_separator: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: Arc::new(TagInner {
                name,
                props: props_vec(&props)?,
                children: intern_children(&children)?,
                separator: intern_separator(&child_separator)?,
                py_props: props.unbind(),
                py_children: children.unbind(),
                py_separator: child_separator.unbind(),
            }),
        })
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn props(&self, py: Python<'_>) -> Py<PyDict> {
        self.inner.py_props.clone_ref(py)
    }

    #[getter]
    fn children(&self, py: Python<'_>) -> Py<PyAny> {
        self.inner.py_children.clone_ref(py)
    }

    #[getter]
    fn child_separator(&self, py: Python<'_>) -> Py<PyAny> {
        self.inner.py_separator.clone_ref(py)
    }

    fn htmy(&self, py: Python<'_>, context: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let inner = &self.inner;
        let attrs = format_attrs(py, &context, &inner.props)?;
        let open = safestr(py, format!("<{} {}>", inner.name, attrs))?;
        let close = safestr(py, format!("</{}>", inner.name))?;

        let mut result: Vec<Py<PyAny>> = Vec::new();
        result.push(open);

        match &inner.separator {
            Separator::None => {
                for item in inner.py_children.bind(py).try_iter()? {
                    result.push(item?.unbind());
                }
            }
            _ => {
                let children = inner.py_children.bind(py);
                let mut iter = children.try_iter()?;
                if let Some(first) = iter.next() {
                    let sep = inner.py_separator.clone_ref(py);
                    result.push(sep.clone_ref(py));
                    result.push(first?.unbind());
                    for item in iter {
                        result.push(sep.clone_ref(py));
                        result.push(item?.unbind());
                    }
                    result.push(sep);
                }
            }
        }

        result.push(close);
        Ok(PyTuple::new(py, result)?.unbind().into_any())
    }
}

#[pyclass(module = "htmy_rs", name = "TagWithPropsImpl")]
pub struct TagWithPropsImpl {
    pub inner: Arc<VoidInner>,
}

#[pymethods]
impl TagWithPropsImpl {
    #[new]
    fn new(name: String, props: Bound<'_, PyDict>) -> PyResult<Self> {
        Ok(Self {
            inner: Arc::new(VoidInner {
                name,
                props: props_vec(&props)?,
                py_props: props.unbind(),
            }),
        })
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn props(&self, py: Python<'_>) -> Py<PyDict> {
        self.inner.py_props.clone_ref(py)
    }

    fn htmy(&self, py: Python<'_>, context: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let inner = &self.inner;
        let attrs = format_attrs(py, &context, &inner.props)?;
        safestr(py, format!("<{} {}/>", inner.name, attrs))
    }
}
