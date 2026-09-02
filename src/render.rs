use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyString};

use crate::format::{write_props, xml_escape};
use crate::intern::{Child, Separator, is_safestr, is_sequence};
use crate::tag::{TagImpl, TagWithPropsImpl};
use crate::{PyTypes, types};

enum Part {
    Str(String),
    Slot(usize),
}

enum PendingKind {
    Component { context: Py<PyAny> },
    Context { obj: Py<PyAny>, parent: Py<PyAny> },
}

struct Pending {
    slot: usize,
    awaitable: Py<PyAny>,
    kind: PendingKind,
}

struct Session {
    root: Vec<Part>,
    slots: Vec<Vec<Part>>,
    pending: Vec<Pending>,
    string_formatter: Py<PyAny>,
    use_default_sf: bool,
    types: PyTypes,
}

fn parts_mut(sess: &mut Session, dest: Option<usize>) -> &mut Vec<Part> {
    match dest {
        None => &mut sess.root,
        Some(i) => &mut sess.slots[i],
    }
}

fn write_str(sess: &mut Session, dest: Option<usize>, s: &str) {
    if s.is_empty() {
        return;
    }
    let parts = parts_mut(sess, dest);
    if let Some(Part::Str(buf)) = parts.last_mut() {
        buf.push_str(s);
    } else {
        parts.push(Part::Str(s.to_string()));
    }
}

fn push_hole(sess: &mut Session, dest: Option<usize>) -> usize {
    let id = sess.slots.len();
    sess.slots.push(Vec::new());
    parts_mut(sess, dest).push(Part::Slot(id));
    id
}

fn flatten(parts: &[Part], slots: &[Vec<Part>], out: &mut String) {
    for p in parts {
        match p {
            Part::Str(s) => out.push_str(s),
            Part::Slot(i) => flatten(&slots[*i], slots, out),
        }
    }
}

fn is_awaitable(types: &PyTypes, obj: &Bound<'_, PyAny>) -> PyResult<bool> {
    types.isawaitable.bind(obj.py()).call1((obj,))?.extract()
}

fn python_formatter<'py>(
    types: &PyTypes,
    context: &Bound<'py, PyAny>,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let fmt = context.call_method1("get", (types.formatter.bind(context.py()),))?;
    if fmt.is_none() {
        Ok(None)
    } else {
        Ok(Some(fmt))
    }
}

fn invalid_type(obj: &Bound<'_, PyAny>) -> PyErr {
    PyValueError::new_err(format!("Invalid component type: {}", obj.get_type()))
}

fn write_text(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    s: &str,
    safe: bool,
    py_obj: &Bound<'_, PyAny>,
) -> PyResult<()> {
    if sess.use_default_sf {
        if safe {
            write_str(sess, dest, s);
        } else {
            let escaped = xml_escape(s);
            write_str(sess, dest, &escaped);
        }
    } else {
        let formatted: String = sess.string_formatter.bind(py).call1((py_obj,))?.extract()?;
        write_str(sess, dest, &formatted);
    }
    Ok(())
}

fn emit_markup(sess: &mut Session, dest: Option<usize>, py: Python<'_>, s: &str) -> PyResult<()> {
    if sess.use_default_sf {
        write_str(sess, dest, s);
        return Ok(());
    }
    let safe = sess.types.safestr.bind(py).call1((s,))?;
    let formatted: String = sess.string_formatter.bind(py).call1((safe,))?.extract()?;
    write_str(sess, dest, &formatted);
    Ok(())
}

fn emit_plain(sess: &mut Session, dest: Option<usize>, py: Python<'_>, s: &str) -> PyResult<()> {
    if sess.use_default_sf {
        write_str(sess, dest, s);
        return Ok(());
    }
    let formatted: String = sess.string_formatter.bind(py).call1((s,))?.extract()?;
    write_str(sess, dest, &formatted);
    Ok(())
}

fn write_open(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    name: &str,
    props: &[(String, Py<PyAny>)],
    void: bool,
    context: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let mut buf = String::new();
    buf.push('<');
    buf.push_str(name);
    buf.push(' ');
    let fmt = python_formatter(&sess.types, context)?;
    write_props(&mut buf, py, &sess.types, props, fmt.as_ref())?;
    if void {
        buf.push_str("/>");
    } else {
        buf.push('>');
    }
    emit_markup(sess, dest, py, &buf)
}

fn write_close(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    name: &str,
) -> PyResult<()> {
    emit_markup(sess, dest, py, &format!("</{name}>"))
}

fn write_tag(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    tag: &Bound<'_, TagImpl>,
    context: &Py<PyAny>,
) -> PyResult<()> {
    let inner = tag.borrow().inner.clone();
    let ctx = context.bind(py);
    write_open(sess, dest, py, &inner.name, &inner.props, false, ctx)?;
    write_children(sess, dest, py, &inner.children, &inner.separator, context)?;
    write_close(sess, dest, py, &inner.name)?;
    Ok(())
}

fn write_void(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    tag: &Bound<'_, TagWithPropsImpl>,
    context: &Py<PyAny>,
) -> PyResult<()> {
    let inner = tag.borrow().inner.clone();
    let ctx = context.bind(py);
    write_open(sess, dest, py, &inner.name, &inner.props, true, ctx)
}

fn write_children(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    children: &[Child],
    sep: &Separator,
    context: &Py<PyAny>,
) -> PyResult<()> {
    match sep {
        Separator::None => {
            for c in children {
                walk_child(sess, dest, py, c, context)?;
            }
        }
        Separator::NewlinePad => {
            if children.is_empty() {
                return Ok(());
            }
            for c in children {
                emit_plain(sess, dest, py, "\n")?;
                walk_child(sess, dest, py, c, context)?;
            }
            emit_plain(sess, dest, py, "\n")?;
        }
        Separator::Opaque(sep) => {
            if children.is_empty() {
                return Ok(());
            }
            let sep_b = sep.bind(py);
            walk_item(sess, dest, py, sep_b, context)?;
            for (i, c) in children.iter().enumerate() {
                if i > 0 {
                    walk_item(sess, dest, py, sep_b, context)?;
                }
                walk_child(sess, dest, py, c, context)?;
            }
            walk_item(sess, dest, py, sep_b, context)?;
        }
    }
    Ok(())
}

fn walk_child(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    child: &Child,
    context: &Py<PyAny>,
) -> PyResult<()> {
    match child {
        Child::Skip => Ok(()),
        Child::Tag(t) => write_tag(sess, dest, py, t.bind(py), context),
        Child::TagWithProps(t) => write_void(sess, dest, py, t.bind(py), context),
        Child::Text {
            s,
            safe,
            py: py_obj,
        } => write_text(sess, dest, py, s, *safe, py_obj.bind(py)),
        Child::Opaque(o) => {
            // Sequences are invalid among tag children: the Python renderer calls
            // `htmy()` on every non-string child, so only component results (which
            // go through `walk_item` directly) may be sequences.
            let obj = o.bind(py);
            if is_sequence(obj) {
                return Err(invalid_type(obj));
            }
            walk_item(sess, dest, py, obj, context)
        }
    }
}

fn walk_item(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    context: &Py<PyAny>,
) -> PyResult<()> {
    if obj.is_none() {
        return Ok(());
    }

    if let Ok(tag) = obj.cast::<TagImpl>() {
        return write_tag(sess, dest, py, tag, context);
    }
    if let Ok(tag) = obj.cast::<TagWithPropsImpl>() {
        return write_void(sess, dest, py, tag, context);
    }

    if is_safestr(&sess.types, obj)? {
        let s: String = obj.extract()?;
        return write_text(sess, dest, py, &s, true, obj);
    }
    if obj.is_instance_of::<PyString>() {
        let s: String = obj.extract()?;
        return write_text(sess, dest, py, &s, false, obj);
    }

    if is_sequence(obj) {
        for item in obj.try_iter()? {
            let item = item?;
            if item.is_none() {
                continue;
            }
            walk_item(sess, dest, py, &item, context)?;
        }
        return Ok(());
    }

    handle_component(sess, dest, py, obj, context)
}

fn handle_component(
    sess: &mut Session,
    dest: Option<usize>,
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    context: &Py<PyAny>,
) -> PyResult<()> {
    let mut ctx_owned: Option<Py<PyAny>> = None;

    if obj.hasattr("htmy_context")? {
        let extra = obj.call_method0("htmy_context")?;
        if is_awaitable(&sess.types, &extra)? {
            let slot = push_hole(sess, dest);
            sess.pending.push(Pending {
                slot,
                awaitable: extra.unbind(),
                kind: PendingKind::Context {
                    obj: obj.clone().unbind(),
                    parent: context.clone_ref(py),
                },
            });
            return Ok(());
        }
        if extra.is_truthy()? {
            ctx_owned = Some(
                sess.types
                    .chainmap
                    .bind(py)
                    .call1((extra, context.bind(py)))?
                    .unbind(),
            );
        }
    }

    let ctx_ref: Py<PyAny> = match ctx_owned {
        Some(c) => c,
        None => context.clone_ref(py),
    };

    if !obj.hasattr("htmy")? {
        return Err(invalid_type(obj));
    }

    let result = obj.call_method1("htmy", (ctx_ref.bind(py),))?;
    if is_awaitable(&sess.types, &result)? {
        let slot = push_hole(sess, dest);
        sess.pending.push(Pending {
            slot,
            awaitable: result.unbind(),
            kind: PendingKind::Component { context: ctx_ref },
        });
        return Ok(());
    }

    walk_item(sess, dest, py, &result, &ctx_ref)
}

fn cancel_pending(py: Python<'_>, pending: &[Pending]) {
    for p in pending {
        let aw = p.awaitable.bind(py);
        if let Ok(close) = aw.getattr("close") {
            let _ = close.call0();
        }
    }
}

fn pending_list(py: Python<'_>, pending: &[Pending]) -> PyResult<Py<PyAny>> {
    let items: Vec<Py<PyAny>> = pending.iter().map(|p| p.awaitable.clone_ref(py)).collect();
    Ok(PyList::new(py, items)?.unbind().into_any())
}

#[pyclass(module = "htmy_rs", name = "RenderSession")]
pub struct RenderSession {
    inner: Session,
}

#[pymethods]
impl RenderSession {
    #[new]
    fn new(
        component: Bound<'_, PyAny>,
        context: Bound<'_, PyAny>,
        string_formatter: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let py = component.py();
        let types = types(py)?;
        let use_default_sf = string_formatter.is(types.xml_format_string.bind(py));
        let ctx = context.unbind();
        let mut sess = Session {
            root: Vec::new(),
            slots: Vec::new(),
            pending: Vec::new(),
            string_formatter: string_formatter.unbind(),
            use_default_sf,
            types,
        };
        if let Err(e) = walk_item(&mut sess, None, py, &component, &ctx) {
            cancel_pending(py, &sess.pending);
            return Err(e);
        }
        Ok(Self { inner: sess })
    }

    fn pending(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        if self.inner.pending.is_empty() {
            Ok(None)
        } else {
            Ok(Some(pending_list(py, &self.inner.pending)?))
        }
    }

    fn submit(
        &mut self,
        py: Python<'_>,
        results: Bound<'_, PyList>,
    ) -> PyResult<Option<Py<PyAny>>> {
        let pending = std::mem::take(&mut self.inner.pending);
        if pending.len() != results.len() {
            cancel_pending(py, &pending);
            return Err(PyValueError::new_err("submit result count mismatch"));
        }
        for (p, result) in pending.into_iter().zip(results.iter()) {
            match p.kind {
                PendingKind::Component { context } => {
                    if let Err(e) = walk_item(&mut self.inner, Some(p.slot), py, &result, &context)
                    {
                        cancel_pending(py, &self.inner.pending);
                        return Err(e);
                    }
                }
                PendingKind::Context { obj, parent } => {
                    let extra = result;
                    let ctx_owned: Py<PyAny> = if extra.is_truthy()? {
                        self.inner
                            .types
                            .chainmap
                            .bind(py)
                            .call1((&extra, parent.bind(py)))?
                            .unbind()
                    } else {
                        parent
                    };
                    let obj_b = obj.bind(py);
                    if !obj_b.hasattr("htmy")? {
                        return Err(invalid_type(obj_b));
                    }
                    let htmy_result = obj_b.call_method1("htmy", (ctx_owned.bind(py),))?;
                    if is_awaitable(&self.inner.types, &htmy_result)? {
                        self.inner.pending.push(Pending {
                            slot: p.slot,
                            awaitable: htmy_result.unbind(),
                            kind: PendingKind::Component { context: ctx_owned },
                        });
                    } else if let Err(e) =
                        walk_item(&mut self.inner, Some(p.slot), py, &htmy_result, &ctx_owned)
                    {
                        cancel_pending(py, &self.inner.pending);
                        return Err(e);
                    }
                }
            }
        }
        if self.inner.pending.is_empty() {
            Ok(None)
        } else {
            Ok(Some(pending_list(py, &self.inner.pending)?))
        }
    }

    fn output(&self) -> String {
        let mut out = String::new();
        flatten(&self.inner.root, &self.inner.slots, &mut out);
        out
    }

    fn cancel(&mut self, py: Python<'_>) {
        cancel_pending(py, &self.inner.pending);
        self.inner.pending.clear();
    }
}
