//! Active elements.
//!
//! This module defines the [`IsElementActive`] trait for [`Document`], which is
//! used to check if a given UI element is the active element.

use web_sys::Document;

/// Trait to check if an element is active.
///
/// This trait is used to implement `is_element_active` as a method on
/// [`Document`] instead of a function of two arguments.
pub trait IsElementActive {
    /// Returns `true` if the element is active.
    ///
    /// The `id` argument indicates the ID of the element.
    fn is_element_active(&self, id: &str) -> bool;
}

impl IsElementActive for Document {
    fn is_element_active(&self, id: &str) -> bool {
        self.active_element()
            .map(|elem| elem.id() == id)
            .unwrap_or(false)
    }
}
