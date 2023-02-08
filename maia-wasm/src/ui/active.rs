use web_sys::Document;

pub trait IsElementActive {
    fn is_element_active(&self, id: &str) -> bool;
}

impl IsElementActive for Document {
    fn is_element_active(&self, id: &str) -> bool {
        self.active_element()
            .map(|elem| elem.id() == id)
            .unwrap_or(false)
    }
}
