//! Bounded worker-to-UI secondary-window creation handoffs.

use super::*;

impl<T: Clone> UiWindowGroup<T> {
    /// Sends one validated secondary-view creation request to the UI thread.
    ///
    /// A returned identity means the UI thread created and registered the
    /// native view, then committed the portable view. A native caller whose
    /// [`Self::complete_open`] returns `false` must discard any just-created
    /// window, because the worker timed out and the group rolled the pending
    /// view back.
    pub fn open_secondary(
        &self,
        context: T,
        encoded_document: &str,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_inner(
            context,
            encoded_document,
            UI_WINDOW_OPEN_RESPONSE_TIMEOUT,
            SecondaryDocumentFormat::V1,
        )
    }

    /// Sends one validated version-2 secondary-view creation request to the UI
    /// thread. Its lifecycle is identical to [`Self::open_secondary`], but the
    /// document is decoded through the explicit scroll-aware format.
    pub fn open_secondary_v2(
        &self,
        context: T,
        encoded_document: &str,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_inner(
            context,
            encoded_document,
            UI_WINDOW_OPEN_RESPONSE_TIMEOUT,
            SecondaryDocumentFormat::V2,
        )
    }

    /// Sends one validated version-3 secondary-view creation request to the UI
    /// thread. Its lifecycle is identical to [`Self::open_secondary`], but the
    /// document is decoded through the explicit status-aware format.
    pub fn open_secondary_v3(
        &self,
        context: T,
        encoded_document: &str,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_inner(
            context,
            encoded_document,
            UI_WINDOW_OPEN_RESPONSE_TIMEOUT,
            SecondaryDocumentFormat::V3,
        )
    }

    #[cfg(test)]
    pub(super) fn open_secondary_within(
        &self,
        context: T,
        encoded_document: &str,
        timeout: Duration,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        self.open_secondary_inner(
            context,
            encoded_document,
            timeout,
            SecondaryDocumentFormat::V1,
        )
    }

    fn open_secondary_inner(
        &self,
        context: T,
        encoded_document: &str,
        timeout: Duration,
        format: SecondaryDocumentFormat,
    ) -> Result<UiWindowId, UiWindowGroupError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let mut state = lock(&self.state);
            if state.active.is_some() {
                return Err(UiWindowGroupError::Busy);
            }
            let pending = match format {
                SecondaryDocumentFormat::V1 => state.windows.prepare_secondary(encoded_document),
                SecondaryDocumentFormat::V2 => state.windows.prepare_secondary_v2(encoded_document),
                SecondaryDocumentFormat::V3 => state.windows.prepare_secondary_v3(encoded_document),
            }
            .map_err(map_window_error)?;
            state.next_request_id = state.next_request_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_request_id;
            state.active = Some(ActiveOpen {
                id: request_id,
                context,
                pending,
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        let result = wait_for_response(&response, timeout);
        if result.is_none() {
            let mut state = lock(&self.state);
            let Some(active) = state.active.take() else {
                // `complete_open` publishes its response before it releases
                // this state lock. It won the deadline race, so return that
                // already-committed outcome instead of reporting a false
                // timeout to the caller.
                drop(state);
                return take_completed_response(&response)
                    .unwrap_or(Err(UiWindowGroupError::Unavailable));
            };
            if active.id != request_id {
                state.active = Some(active);
                return Err(UiWindowGroupError::Unavailable);
            }
            let _ = state.windows.abort_secondary(active.pending);
            return Err(UiWindowGroupError::Unavailable);
        }
        result.expect("response exists when the wait did not time out")
    }

    /// Takes one native-view creation request exactly once on the UI thread.
    #[must_use]
    pub fn take_open_request(&self) -> Option<UiWindowOpenRequest<T>> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            return None;
        }
        active.taken = true;
        Some(UiWindowOpenRequest {
            id: active.id,
            context: active.context.clone(),
            resources: active.pending.resources(),
            snapshot: UiWindowSnapshot::new(
                active.pending.id().clone(),
                active.pending.snapshot().clone(),
            ),
        })
    }

    /// Completes one native creation request.
    ///
    /// `true` commits and publishes the prepared view. `false` aborts it. The
    /// boolean return is false for an expired or unknown request, telling a
    /// native host to destroy a late-created window rather than leave it bound
    /// to a rolled-back session view.
    pub fn complete_open(&self, request_id: u64, created: bool) -> bool {
        {
            let mut state = lock(&self.state);
            let Some(active) = state.active.as_ref() else {
                return false;
            };
            if active.id != request_id || !active.taken {
                return false;
            }
            let active = state.active.take().expect("active request was present");
            let response = Arc::clone(&active.response);
            let result = if created {
                match state.windows.commit_secondary(active.pending) {
                    Ok(snapshot) => Ok(snapshot.id().clone()),
                    Err(_) => Err(UiWindowGroupError::Unavailable),
                }
            } else {
                let _ = state.windows.abort_secondary(active.pending);
                Err(UiWindowGroupError::Unavailable)
            };
            // Keep the state lock while publishing the outcome. A timed-out
            // worker that observes no active request can then only observe a
            // completed response, never a transient empty slot.
            let value = &mut *lock(&response.value);
            *value = Some(result);
            response.ready.notify_one();
        }
        true
    }

    /// Fails one native creation request without exposing the native cause.
    pub fn fail_open(&self, request_id: u64) -> bool {
        self.complete_open(request_id, false)
    }
}
