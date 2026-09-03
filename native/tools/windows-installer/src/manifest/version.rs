/// One release directory version, distinct from protocol compatibility.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackageVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

impl PackageVersion {
    /// Creates one exact three-component release version.
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns the release version's major component.
    #[must_use]
    pub const fn major(self) -> u16 {
        self.major
    }

    /// Returns the release version's minor component.
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }

    /// Returns the release version's patch component.
    #[must_use]
    pub const fn patch(self) -> u16 {
        self.patch
    }

    /// Parses the exact canonical name of a release directory.
    ///
    /// Release promotion creates only `major.minor.patch` names with ordinary
    /// decimal components. Rejecting alternate spellings keeps a version
    /// directory from having more than one textual identity.
    pub fn from_canonical_directory_name(name: &str) -> Option<Self> {
        let mut components = name.split('.');
        let major = parse_directory_component(components.next()?)?;
        let minor = parse_directory_component(components.next()?)?;
        let patch = parse_directory_component(components.next()?)?;
        components.next().is_none().then_some(Self {
            major,
            minor,
            patch,
        })
    }
}

fn parse_directory_component(component: &str) -> Option<u16> {
    (!component.is_empty()
        && component.bytes().all(|byte| byte.is_ascii_digit())
        && (component.len() == 1 || !component.starts_with('0')))
    .then(|| component.parse().ok())?
}
