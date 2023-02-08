//! Pointer device handling.
//!
//! This module implements handling of pointer devices, including mouse dragging
//! and two-finger touchscreen gestures.

use web_sys::PointerEvent;

const NUM_POINTERS: usize = 2;

/// Pointer tracker.
///
/// A pointer tracker receives [`PointerEvent`]'s from the web browser, maintains
/// state information about the pointers that are active, and generates
/// [`PointerGesture`]'s.
pub struct PointerTracker {
    slots: [Option<PointerEvent>; NUM_POINTERS],
}

/// Pointer gesture.
///
/// Pointer gestures are higher level descriptions of the actions performed by
/// pointer devices. These are easier to interpret than the information in a
/// [`PointerEvent`], so the applicaton can implement actions according to
/// pointer gestures.
pub enum PointerGesture {
    /// A drag gesture.
    ///
    /// Drag gestures are performed by clicking and dragging with a mouse or by
    /// dragging one finger on a touchscreen.
    Drag {
        /// Displacement in the X dimension (in pixels).
        dx: i32,
        /// Displacement in the Y dimension (in pixels).
        dy: i32,
    },
    /// A pinch gesture.
    ///
    /// Pinch gestures are performed by touching two fingers on a touchscreen
    /// and moving them closer together or futher apart.
    Pinch {
        /// Center of the pinch gesture.
        ///
        /// The pinch gesture center is the mean point between the locations of
        /// the two fingers involved in the pinch.
        center: (i32, i32),
        /// Dilation factor.
        ///
        /// The dilation factor indicates the zoom factor that the relative
        /// movement of the fingers has caused.
        dilation: (f32, f32),
    },
}

impl PointerTracker {
    /// Creates a new pointer tracker.
    pub fn new() -> PointerTracker {
        PointerTracker {
            slots: Default::default(),
        }
    }

    /// Handler for the pointer down event.
    ///
    /// This function should be used as the handler for pointer down events.
    pub fn on_pointer_down(&mut self, event: PointerEvent) {
        self.record_event(event);
    }

    #[allow(clippy::needless_return)]
    fn record_event(&mut self, event: PointerEvent) {
        let pointer_id = event.pointer_id();
        // Search previous event with same pointer ID.
        if let Some(slot) = self.slots.iter_mut().find_map(|x| {
            x.as_mut().and_then(|x| {
                if x.pointer_id() == pointer_id {
                    Some(x)
                } else {
                    None
                }
            })
        }) {
            // Replace event with the new one.
            *slot = event;
            return;
        }
        // Search for an empty slot.
        if let Some(slot) = self.slots.iter_mut().find(|x| x.is_none()) {
            slot.replace(event);
            return;
        }
        // We found no empty slots, so we cannot handle this pointer (this
        // typically should not happen).
    }

    /// Handler for the pointer up event.
    ///
    /// This function should be used as the handler for pointer up events.
    #[allow(clippy::needless_return)]
    pub fn on_pointer_up(&mut self, event: PointerEvent) {
        let pointer_id = event.pointer_id();
        // Search previous event with the same pointer (this typically should be
        // found).
        if let Some(slot) = self
            .slots
            .iter_mut()
            .find(|x| x.as_ref().map_or(false, |x| x.pointer_id() == pointer_id))
        {
            // Remove event.
            slot.take();
            return;
        }
        // The pointer event was not found, so there is nothing to remove (this
        // typically should not happen).
    }

    fn get_event(&self, pointer_id: i32) -> Option<&PointerEvent> {
        self.slots.iter().find_map(|x| {
            x.as_ref().and_then(|x| {
                if x.pointer_id() == pointer_id {
                    Some(x)
                } else {
                    None
                }
            })
        })
    }

    /// Handler for the pointer move event.
    ///
    /// This functions should be used as the hanlder for pointer move events.
    ///
    /// If the event produces a corresponding pointer gesture, it is returned.
    pub fn on_pointer_move(&mut self, event: PointerEvent) -> Option<PointerGesture> {
        let ret = match self.num_active_pointers() {
            1 => self
                .get_event(event.pointer_id())
                .map(|old_event| self.drag(&event, old_event)),
            2 => self.pinch(&event),
            _ => None,
        };
        if ret.is_some() {
            self.record_event(event);
        }
        ret
    }

    /// Checks if there are any active pointers.
    ///
    /// This function returns `true` if there are any active pointers
    /// currently. An active pointer is one for which a pointer down event has
    /// been received, and the corresponding pointer up event has not been
    /// received yet.
    pub fn has_active_pointers(&self) -> bool {
        self.slots.iter().any(|x| x.is_some())
    }

    fn num_active_pointers(&self) -> usize {
        self.slots.iter().filter(|x| x.is_some()).count()
    }

    fn drag(&self, new: &PointerEvent, old: &PointerEvent) -> PointerGesture {
        PointerGesture::Drag {
            dx: new.client_x() - old.client_x(),
            dy: new.client_y() - old.client_y(),
        }
    }

    fn pinch(&self, event: &PointerEvent) -> Option<PointerGesture> {
        let pointer_id = event.pointer_id();
        // This event might not be present in the slots. In that case the pinch
        // is invalid.
        let same = self.get_event(pointer_id)?;
        // There must be another event in the slots, since this is only called
        // when there are 2 events in the slots.
        let other = self
            .slots
            .iter()
            .find_map(|x| {
                x.as_ref().and_then(|x| {
                    if x.pointer_id() != pointer_id {
                        Some(x)
                    } else {
                        None
                    }
                })
            })
            .unwrap();
        let same_x = same.client_x() as f32;
        let same_y = same.client_y() as f32;
        let other_x = other.client_x() as f32;
        let other_y = other.client_y() as f32;
        let new_x = event.client_x() as f32;
        let new_y = event.client_y() as f32;
        let min_dilation = 0.5;
        let max_dilation = 2.0;
        let min_distance = 10.0;
        let dist_x = (same_x - other_x).abs();
        let dist_y = (same_y - other_y).abs();
        let dilation_x = if dist_x >= min_distance {
            ((new_x - other_x) / (same_x - other_x))
                .abs()
                .clamp(min_dilation, max_dilation)
        } else {
            1.0
        };
        let dilation_y = if dist_y >= min_distance {
            ((new_y - other_y) / (same_y - other_y))
                .abs()
                .clamp(min_dilation, max_dilation)
        } else {
            1.0
        };
        Some(PointerGesture::Pinch {
            center: (other.client_x(), other.client_y()),
            dilation: (dilation_x, dilation_y),
        })
    }
}

impl Default for PointerTracker {
    fn default() -> PointerTracker {
        PointerTracker::new()
    }
}
