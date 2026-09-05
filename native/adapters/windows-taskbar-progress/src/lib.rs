#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Direct best-effort Windows taskbar progress for one host-owned window.
//!
//! The caller must wait until Windows has delivered that window's
//! `TaskbarButtonCreated` message. This adapter has no protocol, window
//! creation, scheduling, application data, or retained COM object. See
//! `docs/PRODUCT_UPDATE_PROGRESS.md` and Decision 0200.

mod raw;

/// One bounded taskbar presentation state for a host-owned operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskbarProgress {
    /// Removes any earlier operation indicator from the button.
    Clear,
    /// Reports activity whose completion fraction is not yet meaningful.
    Activity,
    /// Reports one already bounded completed/total pair.
    Determinate {
        /// The completed portion of the operation.
        completed: u64,
        /// The nonzero portion that represents completion.
        total: u64,
    },
}

impl TaskbarProgress {
    const fn is_valid(self) -> bool {
        match self {
            Self::Clear | Self::Activity => true,
            Self::Determinate { completed, total } => total != 0 && completed <= total,
        }
    }
}

/// Applies one best-effort taskbar progress state to an already-ready window.
///
/// A false result deliberately exposes no Windows or COM detail. The host's
/// independent native caption remains the authoritative visual representation.
/// This function creates and releases its direct COM object synchronously on
/// the caller's UI thread; it never stores a pointer across a worker boundary.
#[must_use]
pub fn set_taskbar_progress(window: isize, progress: TaskbarProgress) -> bool {
    if window == 0 || !progress.is_valid() {
        return false;
    }
    raw::set(window, progress)
}

#[cfg(test)]
mod tests {
    use super::TaskbarProgress;

    #[test]
    fn determinate_values_are_closed_and_nonzero() {
        assert!(TaskbarProgress::Clear.is_valid());
        assert!(TaskbarProgress::Activity.is_valid());
        assert!(
            TaskbarProgress::Determinate {
                completed: 0,
                total: 1
            }
            .is_valid()
        );
        assert!(
            !TaskbarProgress::Determinate {
                completed: 1,
                total: 0
            }
            .is_valid()
        );
        assert!(
            !TaskbarProgress::Determinate {
                completed: 3,
                total: 2
            }
            .is_valid()
        );
    }
}
