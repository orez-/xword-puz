use std::collections::HashMap;
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub struct MultiError<E> {
    errors: HashMap<&'static str, E>,
}

impl<E> MultiError<E> {
    pub(crate) fn new() -> Self {
        Self { errors: HashMap::default() }
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

impl<E: serde::Serialize> Into<JsValue> for MultiError<E> {
    fn into(self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.errors)
            .expect("map of strings to strings should be serializable")
    }
}
