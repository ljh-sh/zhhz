//! CPython extension module via PyO3 (zhhz roadmap; built via maturin).
//!
//! Enabled with `--features python`. Compiles to a `cdylib` shared
//! object (e.g. `zhhz.cpython-312-darwin.so`) that the Python
//! interpreter loads as `import zhhz`.
//!
//! Mirrors the npm `Converter` surface (zhhz#40) so consumers can pick
//! the binding that fits their runtime:
//!
//! | Rust / CLI / npm                 | Python             |
//! |----------------------------------|--------------------|
//! | `Converter::new(Config::S2t)`    | `zhhz.Converter("s2t")` |
//! | `c.convert(text) -> String`     | `c.convert(text)` |
//! | `c.convert_with_custom(...)`    | `c.convert_with_custom(text, entries)` |
//! | `detect::detect_text(text)`      | `zhhz.detect(text)` |
//! | `Config::ALL`                   | `zhhz.configs()` |
//! | `Region::ALL`                   | `zhhz.locales()` |
//!
//! Custom-word entries on the Python side are accepted as either:
//! - A list of `[key, value]` tuples (preferred, mirrors npm array form).
//! - A `dict[str, str]` (opencc-py compat).
//! - A string of `"key value"` pairs separated by `|` (opencc-js DictLike compat).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

use crate::detect;
use crate::engine::{Config as EngineConfig, Converter as EngineConverter, Region as EngineRegion};

#[pyclass(name = "Converter")]
struct PyConverter {
    inner: EngineConverter,
    config_name: String,
}

#[pymethods]
impl PyConverter {
    #[new]
    fn new(config: &str) -> PyResult<Self> {
        let cfg = EngineConfig::parse(config)
            .map_err(|e| PyValueError::new_err(e))?;
        Ok(PyConverter {
            inner: EngineConverter::new(cfg),
            config_name: config.to_string(),
        })
    }

    #[staticmethod]
    fn for_region(from: &str, to: &str) -> PyResult<Self> {
        let f = EngineRegion::parse(from).map_err(PyValueError::new_err)?;
        let t = EngineRegion::parse(to).map_err(PyValueError::new_err)?;
        let cfg = crate::engine::region_pair_config(f, t).map_err(PyValueError::new_err)?;
        Ok(PyConverter {
            inner: EngineConverter::new(cfg),
            config_name: cfg.name().to_string(),
        })
    }

    /// The config name this converter was built for (e.g. `"s2twp"`).
    #[getter]
    fn config(&self) -> &str {
        &self.config_name
    }

    /// Convert `text`. Pure (no I/O, no allocations beyond the result).
    fn convert(&self, text: &str) -> String {
        self.inner.convert(text)
    }

    /// Convert `text` with custom words injected at the highest priority.
    /// `entries` accepts list-of-pairs / dict / DictLike-string.
    fn convert_with_custom(&self, text: &str, entries: &Bound<'_, PyAny>) -> PyResult<String> {
        let pairs = parse_entries(entries)?;
        let cfg = EngineConfig::parse(&self.config_name)
            .expect("config_name was validated at construction");
        Ok(EngineConverter::with_custom(cfg, &pairs).convert(text))
    }

    /// Return a new Converter with these custom words baked in.
    fn with_custom(&self, entries: &Bound<'_, PyAny>) -> PyResult<Self> {
        let pairs = parse_entries(entries)?;
        let cfg = EngineConfig::parse(&self.config_name)
            .expect("config_name was validated at construction");
        Ok(PyConverter {
            inner: EngineConverter::with_custom(cfg, &pairs),
            config_name: self.config_name.clone(),
        })
    }

    fn __repr__(&self) -> String {
        format!("<zhhz.Converter config={}>", self.config_name)
    }
}

#[pyclass(name = "Detection")]
struct PyDetection {
    region: String,
    confidence: u8,
}

#[pymethods]
impl PyDetection {
    #[getter]
    fn region(&self) -> &str {
        &self.region
    }
    #[getter]
    fn confidence(&self) -> u8 {
        self.confidence
    }
    fn __repr__(&self) -> String {
        format!("<zhhz.Detection region={} confidence={}>", self.region, self.confidence)
    }
}

/// Identify the script variant of `text`. Returns `None` when there are no
/// CJK characters or kana.
#[pyfunction]
#[pyo3(name = "detect")]
fn detect_text(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    match detect::detect_text(text) {
        Some(d) => {
            let py_det = Bound::new(
                py,
                PyDetection {
                    region: d.region.code().to_string(),
                    confidence: d.confidence,
                },
            )?;
            Ok(py_det.into_any().unbind())
        }
        None => Ok(py.None()),
    }
}

/// All 16 built-in OpenCC config names in canonical order.
#[pyfunction]
fn configs() -> Vec<String> {
    EngineConfig::ALL.iter().map(|c| c.name().to_string()).collect()
}

/// All 6 region codes in canonical order (`cn-s`, `cn-t`, `cn-tw`,
/// `cn-hk`, `jp-n`, `jp-t`).
#[pyfunction]
fn locales() -> Vec<String> {
    EngineRegion::ALL.iter().map(|r| r.code().to_string()).collect()
}

/// Convert `text` using one of the 16 built-in OpenCC configs.
#[pyfunction]
fn convert(text: &str, config: &str) -> PyResult<String> {
    let cfg = EngineConfig::parse(config).map_err(PyValueError::new_err)?;
    Ok(EngineConverter::new(cfg).convert(text))
}

/// Convert `text` using a `(from, to)` semantic-region pair.
#[pyfunction]
fn convert_region(text: &str, from: &str, to: &str) -> PyResult<String> {
    let c = PyConverter::for_region(from, to)?;
    Ok(c.inner.convert(text))
}

/// Convert with custom words, one-shot. Same `entries` shapes as
/// `Converter.convert_with_custom`.
#[pyfunction]
#[pyo3(signature = (text, config, entries))]
fn convert_with_custom(
    text: &str,
    config: &str,
    entries: &Bound<'_, PyAny>,
) -> PyResult<String> {
    let cfg = EngineConfig::parse(config).map_err(PyValueError::new_err)?;
    let pairs = parse_entries(entries)?;
    Ok(EngineConverter::with_custom(cfg, &pairs).convert(text))
}

// =========================================================================
// helpers
// =========================================================================

/// Accept list-of-pairs / dict / DictLike-string; return Vec<(String, String)>.
fn parse_entries(entries: &Bound<'_, PyAny>) -> PyResult<Vec<(String, String)>> {
    // Dict[str, str]
    if let Ok(d) = entries.downcast::<PyDict>() {
        let mut out = Vec::with_capacity(d.len());
        for (k, v) in d.iter() {
            let key = k
                .downcast::<PyString>()
                .map_err(|_| PyValueError::new_err("dict keys must be strings"))?
                .to_str()?
                .to_string();
            let value = v
                .downcast::<PyString>()
                .map_err(|_| PyValueError::new_err("dict values must be strings"))?
                .to_str()?
                .to_string();
            out.push((key, value));
        }
        if out.is_empty() {
            return Err(PyValueError::new_err("custom dict is empty"));
        }
        return Ok(out);
    }

    // str  —  "key value|key value"
    if let Ok(s) = entries.downcast::<PyString>() {
        let s = s.to_str()?;
        let mut out = Vec::new();
        for (i, raw) in s.split('|').enumerate() {
            let piece = raw.trim();
            if piece.is_empty() {
                continue;
            }
            let (key, value) = piece.split_once(' ').ok_or_else(|| {
                PyValueError::new_err(format!(
                    "dict entry {}: expected 'key value' separated by a space, got {:?}",
                    i, piece
                ))
            })?;
            let key = key.trim();
            if key.is_empty() {
                return Err(PyValueError::new_err(format!("dict entry {}: empty key", i)));
            }
            out.push((key.to_string(), value.trim().to_string()));
        }
        if out.is_empty() {
            return Err(PyValueError::new_err("custom dict string parsed to zero entries"));
        }
        return Ok(out);
    }

    // list / tuple of [key, value]
    if entries.is_instance_of::<PyList>() || entries.is_instance_of::<PyTuple>() {
        return parse_pair_seq(entries);
    }

    // Generic iterable — accept generators, etc.
    if entries.hasattr("__iter__")? {
        return parse_pair_iter(entries);
    }

    Err(PyValueError::new_err(
        "entries must be a list of [key, value] pairs, a dict, or a 'key value|...' string",
    ))
}

fn parse_pair_seq<'py>(
    seq: &Bound<'py, PyAny>,
) -> PyResult<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut iter = seq.iter()?;
    while let Some(item) = iter.next() {
        let item = item?;
        out.push(parse_pair(&item, i)?);
        i += 1;
    }
    if out.is_empty() {
        return Err(PyValueError::new_err("custom entries list is empty"));
    }
    Ok(out)
}

fn parse_pair_iter<'py>(
    iter_obj: &Bound<'py, PyAny>,
) -> PyResult<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut iter = iter_obj.iter()?;
    while let Some(item) = iter.next() {
        let item = item?;
        out.push(parse_pair(&item, i)?);
        i += 1;
    }
    if out.is_empty() {
        return Err(PyValueError::new_err("custom entries iterator is empty"));
    }
    Ok(out)
}

fn parse_pair(item: &Bound<'_, PyAny>, i: usize) -> PyResult<(String, String)> {
    if let Ok(t) = item.downcast::<PyTuple>() {
        if t.len() != 2 {
            return Err(PyValueError::new_err(format!(
                "entries[{}] must be a 2-element [key, value]",
                i
            )));
        }
        let key = t.get_item(0)?.extract::<String>()?;
        let value = t.get_item(1)?.extract::<String>()?;
        if key.is_empty() {
            return Err(PyValueError::new_err(format!("entries[{}] has an empty key", i)));
        }
        return Ok((key, value));
    }
    if let Ok(l) = item.downcast::<PyList>() {
        if l.len() != 2 {
            return Err(PyValueError::new_err(format!(
                "entries[{}] must be a 2-element [key, value]",
                i
            )));
        }
        let key = l.get_item(0)?.extract::<String>()?;
        let value = l.get_item(1)?.extract::<String>()?;
        if key.is_empty() {
            return Err(PyValueError::new_err(format!("entries[{}] has an empty key", i)));
        }
        return Ok((key, value));
    }
    Err(PyValueError::new_err(format!(
        "entries[{}] must be a [key, value] pair",
        i
    )))
}

/// Register the `zhhz` Python module.
#[pymodule]
fn zhhz(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<PyConverter>()?;
    m.add_class::<PyDetection>()?;
    m.add_function(wrap_pyfunction!(detect_text, m)?)?;
    m.add_function(wrap_pyfunction!(configs, m)?)?;
    m.add_function(wrap_pyfunction!(locales, m)?)?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(convert_region, m)?)?;
    m.add_function(wrap_pyfunction!(convert_with_custom, m)?)?;
    Ok(())
}