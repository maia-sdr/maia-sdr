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
    slots: [Option<Pointer>; NUM_POINTERS],
    new_series_id: u8,
}

struct Pointer {
    event: PointerEvent,
    series_id: u8,
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
        /// X position (in pixels) of the previous pointer location
        x0: i32,
        /// Y position (in pixels) of the previous pointer location
        y0: i32,
        /// ID for the series of gestures.
        ///
        /// Gestures that are generated as part of the same action have the same
        /// series ID.
        series_id: u8,
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
        /// ID for the series of gestures.
        ///
        /// Gestures that are generated as part of the same action have the same
        /// series ID.
        series_id: u8,
    },
}

impl PointerTracker {
    /// Creates a new pointer tracker.
    pub fn new() -> PointerTracker {
        PointerTracker {
            slots: Default::default(),
            new_series_id: 0,
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
                if x.event.pointer_id() == pointer_id {
                    Some(x)
                } else {
                    None
                }
            })
        }) {
            // Replace event with the new one.
            slot.event = event;
            return;
        }
        // Search for an empty slot.
        if let Some(slot) = self.slots.iter_mut().find(|x| x.is_none()) {
            // create new series of pointer
            slot.replace(Pointer {
                event,
                series_id: self.new_series_id,
            });
            self.new_series_id = self.new_series_id.wrapping_add(1);
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
        if let Some(slot) = self.slots.iter_mut().find(|x| {
            x.as_ref()
                .is_some_and(|x| x.event.pointer_id() == pointer_id)
        }) {
            // Remove event.
            slot.take();
            return;
        }
        // The pointer event was not found, so there is nothing to remove (this
        // typically should not happen).
    }

    fn get_pointer(&self, pointer_id: i32) -> Option<&Pointer> {
        self.slots.iter().find_map(|x| {
            x.as_ref().and_then(|x| {
                if x.event.pointer_id() == pointer_id {
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
                .get_pointer(event.pointer_id())
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

    fn drag(&self, new: &PointerEvent, old: &Pointer) -> PointerGesture {
        let x0 = old.event.client_x();
        let y0 = old.event.client_y();
        PointerGesture::Drag {
            dx: new.client_x() - x0,
            dy: new.client_y() - y0,
            x0,
            y0,
            series_id: old.series_id,
        }
    }

    fn pinch(&self, event: &PointerEvent) -> Option<PointerGesture> {
        let pointer_id = event.pointer_id();
        // This event might not be present in the slots. In that case the pinch
        // is invalid.
        let same = self.get_pointer(pointer_id)?;
        // There must be another event in the slots, since this is only called
        // when there are 2 events in the slots.
        let other = self
            .slots
            .iter()
            .find_map(|x| {
                x.as_ref().and_then(|x| {
                    if x.event.pointer_id() != pointer_id {
                        Some(x)
                    } else {
                        None
                    }
                })
            })
            .unwrap();
        let same_x = same.event.client_x() as f32;
        let same_y = same.event.client_y() as f32;
        let other_x = other.event.client_x() as f32;
        let other_y = other.event.client_y() as f32;
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
            center: (other.event.client_x(), other.event.client_y()),
            dilation: (dilation_x, dilation_y),
            // to assign a consistent series ID regardless of which pointer
            // generated the event, we take the minimum
            series_id: same.series_id.min(other.series_id),
        })
    }
}

impl Default for PointerTracker {
    fn default() -> PointerTracker {
        PointerTracker::new()
    }
}
