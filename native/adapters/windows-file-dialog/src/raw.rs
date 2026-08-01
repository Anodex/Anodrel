//! Narrow Comdlg32 binding for one bounded host-owned open-file dialog.
use anodrel_file_dialog::{FileDialogFilter, SelectedFilePath};
use std::{mem, ptr};
const MAX_PATH_UNITS: usize = 32_768;
const OFN_EXPLORER: u32 = 0x0008_0000;
const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
#[repr(C)]
struct OpenFileNameW {
    l_struct_size: u32,
    hwnd_owner: isize,
    h_instance: isize,
    filter: *const u16,
    custom_filter: *mut u16,
    custom_filter_max: u32,
    filter_index: u32,
    file: *mut u16,
    file_max: u32,
    file_title: *mut u16,
    file_title_max: u32,
    initial_dir: *const u16,
    title: *const u16,
    flags: u32,
    file_offset: u16,
    file_extension: u16,
    def_ext: *const u16,
    cust_data: isize,
    hook: *const core::ffi::c_void,
    template_name: *const u16,
    reserved: *mut core::ffi::c_void,
    reserved_max: u32,
    flags_ex: u32,
}
#[link(name = "comdlg32")]
unsafe extern "system" {
    fn GetOpenFileNameW(value: *mut OpenFileNameW) -> i32;
    fn CommDlgExtendedError() -> u32;
}
pub(super) fn open_file(
    owner_window: isize,
    filters: &[FileDialogFilter],
) -> Result<Option<SelectedFilePath>, ()> {
    let filter = filter_string(filters);
    let mut file = vec![0_u16; MAX_PATH_UNITS];
    let mut value = OpenFileNameW {
        l_struct_size: mem::size_of::<OpenFileNameW>() as u32,
        hwnd_owner: owner_window,
        h_instance: 0,
        filter: filter.as_ptr(),
        custom_filter: ptr::null_mut(),
        custom_filter_max: 0,
        filter_index: 1,
        file: file.as_mut_ptr(),
        file_max: MAX_PATH_UNITS as u32,
        file_title: ptr::null_mut(),
        file_title_max: 0,
        initial_dir: ptr::null(),
        title: ptr::null(),
        flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
        file_offset: 0,
        file_extension: 0,
        def_ext: ptr::null(),
        cust_data: 0,
        hook: ptr::null(),
        template_name: ptr::null(),
        reserved: ptr::null_mut(),
        reserved_max: 0,
        flags_ex: 0,
    };
    let selected = unsafe { GetOpenFileNameW(&mut value) };
    if selected == 0 {
        let error = unsafe { CommDlgExtendedError() };
        return if error == 0 { Ok(None) } else { Err(()) };
    }
    let Some(end) = file.iter().position(|unit| *unit == 0) else {
        return Err(());
    };
    let path = String::from_utf16(&file[..end]).map_err(|_| ())?;
    SelectedFilePath::new(path).map(Some).map_err(|_| ())
}
fn filter_string(filters: &[FileDialogFilter]) -> Vec<u16> {
    let mut result = Vec::new();
    for filter in filters {
        result.extend(filter.label().encode_utf16());
        result.push(0);
        let pattern = filter
            .extensions()
            .iter()
            .map(|extension| format!("*.{extension}"))
            .collect::<Vec<_>>()
            .join(";");
        result.extend(pattern.encode_utf16());
        result.push(0);
    }
    result.push(0);
    result
}
#[cfg(test)]
mod tests {
    use super::filter_string;
    use anodrel_file_dialog::FileDialogFilter;
    #[test]
    fn filter_has_double_terminal_nul() {
        let f = FileDialogFilter::new("Text", vec!["txt".to_owned()]).unwrap();
        let e = filter_string(&[f]);
        assert_eq!(&e[e.len() - 2..], [0, 0]);
    }
}
