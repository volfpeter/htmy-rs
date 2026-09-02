use pyo3::prelude::*;
use pyo3::types::{PyList, PyString, PyTuple};

use crate::PyTypes;
use crate::tag::{TagImpl, TagWithPropsImpl};

pub enum Child {
    Tag(Py<TagImpl>),
    TagWithProps(Py<TagWithPropsImpl>),
    Text {
        s: String,
        safe: bool,
        py: Py<PyAny>,
    },
    Skip,
    Opaque(Py<PyAny>),
}

pub enum Separator {
    NewlinePad,
    None,
    Opaque(Py<PyAny>),
}

pub fn is_safestr(types: &PyTypes, obj: &Bound<'_, PyAny>) -> PyResult<bool> {
    obj.is_instance(types.safestr.bind(obj.py()))
}

pub fn intern_one(types: &PyTypes, obj: &Bound<'_, PyAny>) -> PyResult<Child> {
    if obj.is_none() {
        return Ok(Child::Skip);
    }
    if let Ok(tag) = obj.cast::<TagImpl>() {
        return Ok(Child::Tag(tag.clone().unbind()));
    }
    if let Ok(tag) = obj.cast::<TagWithPropsImpl>() {
        return Ok(Child::TagWithProps(tag.clone().unbind()));
    }
    if is_safestr(types, obj)? {
        let s: String = obj.extract()?;
        return Ok(Child::Text {
            s,
            safe: true,
            py: obj.clone().unbind(),
        });
    }
    if obj.is_instance_of::<PyString>() {
        let s: String = obj.extract()?;
        return Ok(Child::Text {
            s,
            safe: false,
            py: obj.clone().unbind(),
        });
    }
    Ok(Child::Opaque(obj.clone().unbind()))
}

pub fn intern_children(types: &PyTypes, children: &Bound<'_, PyAny>) -> PyResult<Vec<Child>> {
    let mut out = Vec::new();
    for item in children.try_iter()? {
        out.push(intern_one(types, &item?)?);
    }
    Ok(out)
}

pub fn intern_separator(types: &PyTypes, sep: &Bound<'_, PyAny>) -> PyResult<Separator> {
    if sep.is_none() {
        return Ok(Separator::None);
    }
    if sep.is_instance_of::<PyString>() && !is_safestr(types, sep)? {
        let s = sep.cast::<PyString>()?.to_str()?;
        if s == "\n" {
            return Ok(Separator::NewlinePad);
        }
    }
    Ok(Separator::Opaque(sep.clone().unbind()))
}

pub fn is_sequence(obj: &Bound<'_, PyAny>) -> bool {
    obj.is_instance_of::<PyList>() || obj.is_instance_of::<PyTuple>()
}
