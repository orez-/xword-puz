use std::collections::HashMap;
use wasm_bindgen::JsValue;

#[derive(Default)]
pub struct MultiError {
    errors: HashMap<String, String>,
}

impl MultiError {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(crate) fn push(&mut self, section: &str, msg: String) {
        self.errors.insert(section.into(), msg);
    }

    pub fn into_error_map(self) -> HashMap<String, String> {
        self.errors
    }
}

impl Into<JsValue> for MultiError {
    fn into(self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.errors)
            .expect("map of strings to strings should be serializable")
    }
}
