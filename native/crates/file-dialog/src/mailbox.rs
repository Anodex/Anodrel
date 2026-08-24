//! One bounded, host-UI-thread dialog request bridge.

use std::{
    fmt,
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crate::{
    FileDialogFilter, SaveFilePath, SaveReference, SelectedFilePath, SelectedFolderPath,
    SelectionReference,
};

/// Maximum time a protocol worker may wait for its host UI thread to respond.
pub const FILE_DIALOG_RESPONSE_TIMEOUT: Duration = Duration::from_secs(120);

/// A host service that can open one file selected by the user.
///
/// An implementation must never treat a selected path as filesystem authority.
pub trait FileDialogService: fmt::Debug + Send {
    /// Requests one file from a host-owned picker.
    fn open_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError>;

    /// Requests one save destination from a host-owned picker.
    fn save_file(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }

    /// Requests one open-file selection that must carry a retained reference.
    fn open_file_with_reference(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }

    /// Requests one folder from a host-owned picker.
    fn open_folder(&self) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }

    /// Requests one save destination that must carry a retained output reference.
    fn save_file_with_reference(
        &self,
        _filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        Err(FileDialogServiceError::Unavailable)
    }
}

/// The outcome of a completed file-picker request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileDialogSelection {
    /// The user selected one absolute path.
    Selected(SelectedFilePath),
    /// The user selected one absolute save destination.
    Saved(SaveFilePath),
    /// The user selected one absolute filesystem folder.
    Folder(SelectedFolderPath),
    /// The UI thread captured one selected file and its opaque reference.
    Captured(SelectedFilePath, SelectionReference),
    /// The UI thread captured one selected output object and its opaque reference.
    CapturedSave(SaveFilePath, SaveReference),
    /// The user dismissed the host-owned picker.
    Cancelled,
}

/// The host-owned operation represented by one pending dialog request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogRequestKind {
    /// Select one existing file.
    Open,
    /// Select one save destination.
    Save,
    /// Select one existing filesystem folder.
    OpenFolder,
    /// Select one existing file while retaining its native identity.
    OpenWithReference,
    /// Select one save destination while retaining its native output object.
    SaveWithReference,
}

/// A safe host-service failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogServiceError {
    /// The host could not route the request to, or complete it on, its UI thread.
    Unavailable,
}

impl fmt::Display for FileDialogServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("file dialog is unavailable")
    }
}

impl std::error::Error for FileDialogServiceError {}

/// A bounded pending request taken exactly once by the host UI thread.
#[derive(Clone, Debug)]
pub struct FileDialogRequest {
    id: u64,
    kind: FileDialogRequestKind,
    filters: Vec<FileDialogFilter>,
}

impl FileDialogRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns whether the host must show an open or a save picker.
    #[must_use]
    pub const fn kind(&self) -> FileDialogRequestKind {
        self.kind
    }

    /// Returns the strict filters requested by the authenticated application.
    #[must_use]
    pub fn filters(&self) -> &[FileDialogFilter] {
        &self.filters
    }
}

/// A one-request bridge from a protocol worker to one host UI thread.
///
/// At most one request may be pending or displayed at a time. A duplicate or a
/// request that cannot be completed within the fixed timeout fails safely.
#[derive(Clone, Debug, Default)]
pub struct FileDialogMailbox {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: Option<ActiveRequest>,
}

#[derive(Debug)]
struct ActiveRequest {
    request: FileDialogRequest,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    value: Mutex<Option<Result<FileDialogSelection, FileDialogServiceError>>>,
    ready: Condvar,
}

impl FileDialogMailbox {
    /// Creates an empty UI-thread dialog mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the current request once so the owning native UI thread can show it.
    #[must_use]
    pub fn take(&self) -> Option<FileDialogRequest> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request.clone())
        }
    }

    /// Completes the matching request after the UI thread closes its picker.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn complete(&self, request_id: u64, selection: FileDialogSelection) -> bool {
        self.respond(request_id, Ok(selection))
    }

    /// Completes the matching request with a safe unavailable result.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn fail(&self, request_id: u64) -> bool {
        self.respond(request_id, Err(FileDialogServiceError::Unavailable))
    }

    fn request(
        &self,
        kind: FileDialogRequestKind,
        filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let state = &mut *lock(&self.state);
            if state.active.is_some() {
                return Err(FileDialogServiceError::Unavailable);
            }
            state.next_id = state.next_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_id;
            state.active = Some(ActiveRequest {
                request: FileDialogRequest {
                    id: request_id,
                    kind,
                    filters: filters.to_vec(),
                },
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        let result = wait_for_response(&response);
        if result.is_none() {
            let state = &mut *lock(&self.state);
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.request.id == request_id)
            {
                state.active = None;
            }
        }
        result.unwrap_or(Err(FileDialogServiceError::Unavailable))
    }

    fn respond(
        &self,
        request_id: u64,
        result: Result<FileDialogSelection, FileDialogServiceError>,
    ) -> bool {
        let response = {
            let state = &mut *lock(&self.state);
            let Some(active) = state.active.as_ref() else {
                return false;
            };
            if active.request.id != request_id
                || !active.taken
                || !selection_matches_kind(active.request.kind, &result)
            {
                return false;
            }
            let response = Arc::clone(&active.response);
            state.active = None;
            response
        };
        let value = &mut *lock(&response.value);
        *value = Some(result);
        response.ready.notify_one();
        true
    }
}

impl FileDialogService for FileDialogMailbox {
    fn open_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        self.request(FileDialogRequestKind::Open, filters)
    }

    fn save_file(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        self.request(FileDialogRequestKind::Save, filters)
    }

    fn open_file_with_reference(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        self.request(FileDialogRequestKind::OpenWithReference, filters)
    }

    fn open_folder(&self) -> Result<FileDialogSelection, FileDialogServiceError> {
        self.request(FileDialogRequestKind::OpenFolder, &[])
    }

    fn save_file_with_reference(
        &self,
        filters: &[FileDialogFilter],
    ) -> Result<FileDialogSelection, FileDialogServiceError> {
        self.request(FileDialogRequestKind::SaveWithReference, filters)
    }
}

fn selection_matches_kind(
    kind: FileDialogRequestKind,
    result: &Result<FileDialogSelection, FileDialogServiceError>,
) -> bool {
    result.is_err()
        || matches!(
            (kind, result),
            (_, Ok(FileDialogSelection::Cancelled))
                | (
                    FileDialogRequestKind::Open,
                    Ok(FileDialogSelection::Selected(_))
                )
                | (
                    FileDialogRequestKind::Save,
                    Ok(FileDialogSelection::Saved(_))
                )
                | (
                    FileDialogRequestKind::OpenFolder,
                    Ok(FileDialogSelection::Folder(_))
                )
                | (
                    FileDialogRequestKind::OpenWithReference,
                    Ok(FileDialogSelection::Captured(_, _))
                )
                | (
                    FileDialogRequestKind::SaveWithReference,
                    Ok(FileDialogSelection::CapturedSave(_, _))
                )
        )
}

fn wait_for_response(
    response: &ResponseSlot,
) -> Option<Result<FileDialogSelection, FileDialogServiceError>> {
    let value = lock(&response.value);
    let (mut value, timeout) = response
        .ready
        .wait_timeout_while(value, FILE_DIALOG_RESPONSE_TIMEOUT, |value| value.is_none())
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if timeout.timed_out() && value.is_none() {
        None
    } else {
        value.take()
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, thread};

    use super::{
        FileDialogMailbox, FileDialogRequestKind, FileDialogSelection, FileDialogService,
        FileDialogServiceError,
    };
    use crate::{
        FileDialogFilter, SaveFilePath, SaveReference, SelectedFilePath, SelectedFolderPath,
        SelectionReference,
    };

    fn filter() -> FileDialogFilter {
        FileDialogFilter::new("Text", vec!["txt".to_owned()]).expect("test filter is valid")
    }

    #[test]
    fn transfers_one_request_to_the_ui_thread_and_returns_its_selection() {
        let mailbox = FileDialogMailbox::new();
        let worker = mailbox.clone();
        let (sent, received) = mpsc::channel();
        let worker_thread = thread::spawn(move || {
            sent.send(
                worker
                    .open_file(&[filter()])
                    .expect("host UI completes dialog"),
            )
        });

        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.filters(), &[filter()]);
        assert_eq!(request.kind(), FileDialogRequestKind::Open);
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(
            request.id(),
            FileDialogSelection::Selected(
                SelectedFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid"),
            ),
        ));
        assert_eq!(
            received.recv().expect("worker responded"),
            FileDialogSelection::Selected(
                SelectedFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid"),
            )
        );
        worker_thread
            .join()
            .expect("worker did not panic")
            .expect("receiver remained available");
    }

    #[test]
    fn rejects_a_second_modal_request_while_one_is_pending() {
        let mailbox = FileDialogMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.open_file(&[filter()]));
        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(
            mailbox.open_file(&[filter()]),
            Err(FileDialogServiceError::Unavailable)
        );
        assert!(mailbox.complete(request.id(), FileDialogSelection::Cancelled));
        assert_eq!(
            waiting.join().expect("worker did not panic"),
            Ok(FileDialogSelection::Cancelled)
        );
    }

    #[test]
    fn does_not_accept_an_untaken_or_stale_completion() {
        let mailbox = FileDialogMailbox::new();
        assert!(!mailbox.complete(1, FileDialogSelection::Cancelled));
        assert!(!mailbox.fail(1));
    }

    #[test]
    fn keeps_save_destinations_distinct_from_open_file_selections() {
        let mailbox = FileDialogMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.save_file(&[filter()]));
        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.kind(), FileDialogRequestKind::Save);
        assert!(!mailbox.complete(
            request.id(),
            FileDialogSelection::Selected(
                SelectedFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid"),
            ),
        ));
        assert!(mailbox.complete(
            request.id(),
            FileDialogSelection::Saved(
                SaveFilePath::new(r"C:\\Users\\Owner\\draft.txt").expect("path is valid"),
            ),
        ));
        assert_eq!(
            waiting.join().expect("worker did not panic"),
            Ok(FileDialogSelection::Saved(
                SaveFilePath::new(r"C:\\Users\\Owner\\draft.txt").expect("path is valid"),
            ))
        );
    }

    #[test]
    fn captures_a_selection_only_for_the_reference_request_kind() {
        let mailbox = FileDialogMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.open_file_with_reference(&[filter()]));
        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.kind(), FileDialogRequestKind::OpenWithReference);
        let path = SelectedFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid");
        let reference =
            SelectionReference::new("AbCdEfGhIjKlMnOpQrStUv").expect("reference is valid");
        assert!(mailbox.complete(
            request.id(),
            FileDialogSelection::Captured(path.clone(), reference.clone()),
        ));
        assert_eq!(
            waiting.join().expect("worker did not panic"),
            Ok(FileDialogSelection::Captured(path, reference))
        );
    }

    #[test]
    fn keeps_folder_selections_distinct_from_file_and_save_results() {
        let mailbox = FileDialogMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.open_folder());
        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.kind(), FileDialogRequestKind::OpenFolder);
        assert!(request.filters().is_empty());
        assert!(!mailbox.complete(
            request.id(),
            FileDialogSelection::Selected(
                SelectedFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid"),
            ),
        ));
        let folder =
            SelectedFolderPath::new(r"C:\\Users\\Owner\\Documents").expect("folder is valid");
        assert!(mailbox.complete(request.id(), FileDialogSelection::Folder(folder.clone())));
        assert_eq!(
            waiting.join().expect("worker did not panic"),
            Ok(FileDialogSelection::Folder(folder))
        );
    }

    #[test]
    fn captures_a_save_destination_only_for_the_save_reference_request_kind() {
        let mailbox = FileDialogMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.save_file_with_reference(&[filter()]));
        let request = loop {
            if let Some(request) = mailbox.take() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.kind(), FileDialogRequestKind::SaveWithReference);
        let path = SaveFilePath::new(r"C:\\Users\\Owner\\note.txt").expect("path is valid");
        let reference = SaveReference::new("AbCdEfGhIjKlMnOpQrStUv").expect("reference is valid");
        assert!(mailbox.complete(
            request.id(),
            FileDialogSelection::CapturedSave(path.clone(), reference.clone()),
        ));
        assert_eq!(
            waiting.join().expect("worker did not panic"),
            Ok(FileDialogSelection::CapturedSave(path, reference))
        );
    }
}
