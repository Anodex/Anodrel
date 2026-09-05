//! UI-thread state and owned-worker handoffs for one product update action.

use std::{
    fmt, io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use anodrel_application::is_valid_application_id;
use anodrel_windows_update_consent::UpdateConsent;
use anodrel_windows_update_handoff::UpdateHandoffError;
use anodrel_windows_updater::{
    AvailableUpdate, UpdateImagePreparationError, UpdateLaunchError, UpdateOfferError,
    discover_current_update,
};

/// One product-update controller held only by a verified product window.
///
/// The application identity was previously read from signed machine policy;
/// callers must never derive it from an application request or rendered value.
pub struct ProductUpdateController {
    application_id: String,
    attempt: Option<Attempt>,
    cancelled: Arc<AtomicBool>,
}

enum Attempt {
    Discovering(thread::JoinHandle<Result<AvailableUpdate, UpdateOfferError>>),
    Installing(thread::JoinHandle<Result<InstallResult, InstallFailure>>),
}

enum InstallResult {
    Installed,
    ElevationDeclined,
    CancelledBeforeElevation,
}

#[derive(Debug)]
enum InstallFailure {
    Preparation(UpdateImagePreparationError),
    Launch(UpdateLaunchError),
    Observation(UpdateHandoffError),
    Postcondition(anodrel_windows_updater::UpdateCompletionError),
}

/// A safe terminal result from one native product-update action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductUpdateOutcome {
    /// The person declined Anodrel's confirmation before download.
    ConsentDeclined,
    /// The person declined the separate Windows UAC confirmation.
    ElevationDeclined,
    /// Policy independently proved the selected signed update.
    Installed,
    /// Discovery, transfer, handoff, observation, or proof did not complete.
    Failed,
}

/// One result from a non-blocking product-update poll on its owner UI thread.
pub enum ProductUpdatePoll {
    /// A worker is still running, or the controller has no active action.
    Pending,
    /// The signed candidate is ready for the existing native consent prompt.
    ConsentRequired(AvailableUpdate),
    /// One update action has reached a safe terminal state.
    Complete(ProductUpdateOutcome),
}

impl fmt::Debug for ProductUpdatePoll {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => formatter.write_str("ProductUpdatePoll::Pending"),
            Self::ConsentRequired(_) => {
                formatter.write_str("ProductUpdatePoll::ConsentRequired(..)")
            }
            Self::Complete(outcome) => formatter
                .debug_tuple("ProductUpdatePoll::Complete")
                .field(outcome)
                .finish(),
        }
    }
}

/// A product-update action could not start its owned discovery worker.
#[derive(Debug)]
pub enum ProductUpdateStartError {
    /// The host attempted to build a controller from an invalid application ID.
    InvalidApplicationId,
    /// Windows-host worker creation failed before discovery began.
    WorkerStart(io::Error),
}

impl fmt::Display for ProductUpdateStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidApplicationId => "the product update identity is invalid",
            Self::WorkerStart(_) => "the product update check could not start",
        })
    }
}

impl std::error::Error for ProductUpdateStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidApplicationId => None,
            Self::WorkerStart(error) => Some(error),
        }
    }
}

impl fmt::Display for InstallFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the product update installation did not complete")
    }
}

impl std::error::Error for InstallFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preparation(error) => Some(error),
            Self::Launch(error) => Some(error),
            Self::Observation(error) => Some(error),
            Self::Postcondition(error) => Some(error),
        }
    }
}

impl ProductUpdateController {
    /// Creates the controller from one host-validated installed application ID.
    pub fn new(application_id: &str) -> Result<Self, ProductUpdateStartError> {
        if !is_valid_application_id(application_id) {
            return Err(ProductUpdateStartError::InvalidApplicationId);
        }
        Ok(Self {
            application_id: application_id.to_owned(),
            attempt: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Starts signed discovery after the person chose the native system-menu item.
    ///
    /// A concurrent action stays active; it does not begin another network
    /// request, prompt, download, UAC handoff, or installation attempt.
    pub fn begin(&mut self) -> Result<bool, ProductUpdateStartError> {
        if self.attempt.is_some() {
            return Ok(false);
        }
        let application_id = self.application_id.clone();
        let worker = thread::Builder::new()
            .name("anodrel-product-update-discovery".to_owned())
            .spawn(move || discover_current_update(&application_id))
            .map_err(ProductUpdateStartError::WorkerStart)?;
        self.attempt = Some(Attempt::Discovering(worker));
        Ok(true)
    }

    /// Returns whether this window still owns an active update attempt.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.attempt.is_some()
    }

    /// Polls one owned worker without blocking the native UI thread.
    ///
    /// A [`ProductUpdatePoll::ConsentRequired`] value must be passed only to
    /// the existing native consent adapter on this same UI thread, then returned
    /// through [`Self::submit_consent`].
    pub fn poll(&mut self) -> ProductUpdatePoll {
        let Some(attempt) = self.attempt.take() else {
            return ProductUpdatePoll::Pending;
        };
        match attempt {
            Attempt::Discovering(worker) if !worker.is_finished() => {
                self.attempt = Some(Attempt::Discovering(worker));
                ProductUpdatePoll::Pending
            }
            Attempt::Discovering(worker) => match worker.join() {
                Ok(Ok(offer)) => ProductUpdatePoll::ConsentRequired(offer),
                Ok(Err(_)) | Err(_) => ProductUpdatePoll::Complete(ProductUpdateOutcome::Failed),
            },
            Attempt::Installing(worker) if !worker.is_finished() => {
                self.attempt = Some(Attempt::Installing(worker));
                ProductUpdatePoll::Pending
            }
            Attempt::Installing(worker) => match worker.join() {
                Ok(Ok(InstallResult::Installed)) => {
                    ProductUpdatePoll::Complete(ProductUpdateOutcome::Installed)
                }
                Ok(Ok(InstallResult::ElevationDeclined)) => {
                    ProductUpdatePoll::Complete(ProductUpdateOutcome::ElevationDeclined)
                }
                Ok(Ok(InstallResult::CancelledBeforeElevation)) => {
                    ProductUpdatePoll::Complete(ProductUpdateOutcome::Failed)
                }
                Ok(Err(_)) | Err(_) => ProductUpdatePoll::Complete(ProductUpdateOutcome::Failed),
            },
        }
    }

    /// Continues a discovered offer after native UI-thread consent.
    ///
    /// The action stays terminal after a decline. Approval starts all transfer,
    /// UAC, observation, and policy proof work on one owned worker.
    pub fn submit_consent(
        &mut self,
        consent: UpdateConsent,
    ) -> Result<ProductUpdatePoll, ProductUpdateStartError> {
        match consent {
            UpdateConsent::Declined => Ok(ProductUpdatePoll::Complete(
                ProductUpdateOutcome::ConsentDeclined,
            )),
            UpdateConsent::Approved(offer) => {
                let cancelled = Arc::clone(&self.cancelled);
                let worker = thread::Builder::new()
                    .name("anodrel-product-update-install".to_owned())
                    .spawn(move || install(offer, cancelled))
                    .map_err(ProductUpdateStartError::WorkerStart)?;
                self.attempt = Some(Attempt::Installing(worker));
                Ok(ProductUpdatePoll::Pending)
            }
        }
    }
}

impl fmt::Debug for ProductUpdateController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProductUpdateController")
            .field("active", &self.is_active())
            .finish_non_exhaustive()
    }
}

impl Drop for ProductUpdateController {
    fn drop(&mut self) {
        // A product window may end while transfer is in flight. The direct
        // downloader cannot interrupt a synchronous WinHTTP read, but this
        // signal guarantees a completed download cannot reach a new UAC
        // handoff after its owning window has gone away.
        self.cancelled.store(true, Ordering::Release);
    }
}

fn install(
    offer: AvailableUpdate,
    cancelled: Arc<AtomicBool>,
) -> Result<InstallResult, InstallFailure> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(InstallResult::CancelledBeforeElevation);
    }
    let ready = offer.download().map_err(InstallFailure::Preparation)?;
    if cancelled.load(Ordering::Acquire) {
        return Ok(InstallResult::CancelledBeforeElevation);
    }
    let process = match ready.begin_elevation() {
        Ok(process) => process,
        Err(UpdateLaunchError::HandoffInvalid(UpdateHandoffError::UserDeclined)) => {
            return Ok(InstallResult::ElevationDeclined);
        }
        Err(error) => return Err(InstallFailure::Launch(error)),
    };
    let completed = process.wait().map_err(InstallFailure::Observation)?;
    completed
        .verify_selection()
        .map_err(InstallFailure::Postcondition)?;
    Ok(InstallResult::Installed)
}

#[cfg(test)]
mod tests {
    use super::{
        Ordering, ProductUpdateController, ProductUpdateOutcome, ProductUpdatePoll,
        ProductUpdateStartError,
    };

    #[test]
    fn invalid_identity_never_starts_a_worker() {
        assert!(matches!(
            ProductUpdateController::new("org.anodrel/escape"),
            Err(ProductUpdateStartError::InvalidApplicationId)
        ));
    }

    #[test]
    fn terminal_outcomes_keep_success_distinct_from_declines_and_failure() {
        assert_ne!(
            ProductUpdateOutcome::Installed,
            ProductUpdateOutcome::ConsentDeclined
        );
        assert_ne!(
            ProductUpdateOutcome::Installed,
            ProductUpdateOutcome::ElevationDeclined
        );
        assert_ne!(
            ProductUpdateOutcome::Installed,
            ProductUpdateOutcome::Failed
        );
        assert!(matches!(
            ProductUpdatePoll::Pending,
            ProductUpdatePoll::Pending
        ));
    }

    #[test]
    fn debug_output_does_not_disclose_the_host_held_application_identity() {
        let controller = ProductUpdateController::new("org.anodrel.product-update-test")
            .expect("valid fixture identity");
        assert_eq!(
            format!("{controller:?}"),
            "ProductUpdateController { active: false, .. }"
        );
    }

    #[test]
    fn dropping_a_window_controller_blocks_a_later_elevation_handoff() {
        let controller = ProductUpdateController::new("org.anodrel.product-update-test")
            .expect("valid fixture identity");
        let cancelled = controller.cancelled.clone();
        drop(controller);
        assert!(cancelled.load(Ordering::Acquire));
    }
}
