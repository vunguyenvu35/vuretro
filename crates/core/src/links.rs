//! The project's own links, and the rule for which ones the app may open in the browser.
//! Pure, so the rule is unit-tested.

pub const REPO: &str = "https://github.com/vunguyenvu35/vuretro";
pub const KOFI: &str = "https://ko-fi.com/vunguyenvu";

/// The only places the app opens from its own pages (About, Support, Help).
const ALLOWED: &[&str] = &[REPO, KOFI];

/// Whether `url` is one of the project's links: exactly an allowed address, or a path, query
/// or fragment below it. Look-alikes (`…/vuretro-evil`, `…/vuretro.evil.com`, other schemes)
/// are refused, so a page can never use the app to open an arbitrary address.
pub fn url_allowed(url: &str) -> bool {
    ALLOWED.iter().any(|base| {
        url == *base
            || ["/", "?", "#"].iter().any(|sep| {
                url.strip_prefix(base)
                    .is_some_and(|rest| rest.starts_with(sep))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_projects_own_pages_are_allowed() {
        for ok in [
            REPO,
            "https://github.com/vunguyenvu35/vuretro/issues/new",
            "https://github.com/vunguyenvu35/vuretro/blob/main/LICENSE",
            "https://github.com/vunguyenvu35/vuretro#support-vuretro",
            "https://github.com/vunguyenvu35/vuretro?tab=readme-ov-file",
            KOFI,
            "https://ko-fi.com/vunguyenvu/",
        ] {
            assert!(url_allowed(ok), "{ok}");
        }
    }

    #[test]
    fn look_alikes_other_schemes_and_junk_are_refused() {
        for bad in [
            "",
            "https://github.com/vunguyenvu35/vuretro-evil",
            "https://github.com/vunguyenvu35/vuretro.evil.com",
            "https://github.com/vunguyenvu35/other",
            "http://github.com/vunguyenvu35/vuretro",
            "https://evil.example/https://github.com/vunguyenvu35/vuretro",
            "https://ko-fi.com/vunguyenvu.evil.com",
            "https://ko-fi.com/someoneelse",
            "javascript:alert(1)",
            "file:///etc/passwd",
            " https://github.com/vunguyenvu35/vuretro",
        ] {
            assert!(!url_allowed(bad), "{bad:?}");
        }
    }
}
