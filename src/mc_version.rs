//! Minecraft version parsing and comparison.
//!
//! Two naming schemes are supported:
//! * Old style: `1.minor[.patch]` (e.g. `1.12.2`, `1.20.1`, `1.21.1`).
//! * New style (26.x onward): `[two-digit-year].minor[.patch]` (e.g. `26.3`, `26.3.1`).
//!
//! Because the new-style major version (the year) is always greater than the old-style
//! major `1`, plain numeric `(major, minor, patch)` comparison covers both schemes without
//! any branching.

/// A parsed version number (major, minor, patch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct McVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl McVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parse a version number. Returns `None` for unparseable versions (e.g. dotless
    /// snapshots such as `24w05a`).
    ///
    /// Suffixes such as `-rc1`, `-pre1`, and whitespace are stripped, so `1.21-rc1`
    /// parses as `(1, 21, 0)`.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('.');
        let major = parse_leading_u32(parts.next()?)?;
        let minor = parse_leading_u32(parts.next()?)?;
        let patch = parts.next().and_then(parse_leading_u32).unwrap_or(0);
        Some(Self { major, minor, patch })
    }
}

/// Take the leading run of ASCII digits and parse it as a `u32`; `None` if empty.
fn parse_leading_u32(s: &str) -> Option<u32> {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}
