use std::collections::HashMap;
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub struct MultiError<E> {
    errors: HashMap<&'static str, E>,
}

impl<E> MultiError<E> {
    pub(crate) fn new() -> Self {
        Self {
            errors: HashMap::default(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(crate) fn insert(&mut self, section: &'static str, err: E) {
        self.errors.insert(section, err);
    }

    pub fn into_error_map(self) -> HashMap<&'static str, E> {
        self.errors
    }
}

impl<E, const N: usize> From<[(&'static str, E); N]> for MultiError<E> {
    fn from(it: [(&'static str, E); N]) -> Self {
        let mut this = Self::new();
        for (k, v) in it {
            this.insert(k, v);
        }
        this
    }
}

impl<E: serde::Serialize> From<MultiError<E>> for JsValue {
    fn from(err: MultiError<E>) -> JsValue {
        serde_wasm_bindgen::to_value(&err.errors)
            .expect("map of strings to strings should be serializable")
    }
}
