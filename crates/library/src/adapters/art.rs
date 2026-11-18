use std::io::Read;
use vuretro_core::ports::{ArtQuery, ArtSource};

const BASE: &str = "https://thumbnails.libretro.com";
/// Box art is a few hundred KB at most.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// `Ok(None)` is a clean "not found".
pub type Get = Box<dyn Fn(&str) -> Result<Option<Vec<u8>>, String> + Send + Sync>;

/// Box art from the libretro-thumbnails server, looked up by official (No-Intro) name.
pub struct ThumbnailsArt {
    get: Get,
    base: String,
}

impl ThumbnailsArt {
    pub fn new() -> Self {
        Self::with_get(BASE.to_string(), Box::new(http_get))
    }

    pub fn with_get(base: String, get: Get) -> Self {
        ThumbnailsArt { get, base }
    }
}

impl Default for ThumbnailsArt {
    fn default() -> Self {
        Self::new()
    }
}

/// libretro-thumbnails file names replace these characters with `_`.
pub fn thumbnail_name(name: &str) -> String {
    name.chars()
        .map(|c| if "&*/:`<>?\\|\"".contains(c) { '_' } else { c })
        .collect()
}

/// Percent-encode everything but unreserved characters (spaces, parentheses, non-ASCII).
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

impl ArtSource for ThumbnailsArt {
    fn id(&self) -> &'static str {
        "libretro-thumbnails"
    }

    fn boxart(&self, q: &ArtQuery) -> Result<Option<Vec<u8>>, String> {
        if q.thumbnails.is_empty() {
            return Ok(None);
        }
        let (system_folder, name) = (q.thumbnails, q.name);
        let url = format!(
            "{}/{}/Named_Boxarts/{}.png",
            self.base,
            encode(system_folder),
            encode(&thumbnail_name(name))
        );
        let png = (self.get)(&url)?;
        // never keep something that is not a PNG (an error page, say)
        Ok(png.filter(|b| b.starts_with(b"\x89PNG")))
    }
}

fn http_get(url: &str) -> Result<Option<Vec<u8>>, String> {
    match ureq::get(url).call() {
        Ok(mut resp) => {
            let mut buf = Vec::new();
            resp.body_mut()
                .as_reader()
                .take(MAX_BYTES)
                .read_to_end(&mut buf)
                .map_err(|e| format!("download failed: {e}"))?;
            Ok(Some(buf))
        }
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(e) => Err(format!("download failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn q<'a>(folder: &'a str, name: &'a str) -> ArtQuery<'a> {
        ArtQuery {
            name,
            thumbnails: folder,
            screenscraper_id: 0,
            login: None,
        }
    }

    fn art(reply: Result<Option<Vec<u8>>, String>) -> (ThumbnailsArt, Arc<Mutex<Vec<String>>>) {
        let urls = Arc::new(Mutex::new(Vec::new()));
        let seen = urls.clone();
        let get: Get = Box::new(move |u| {
            seen.lock().unwrap().push(u.to_string());
            reply.clone()
        });
        (ThumbnailsArt::with_get("https://t.test".into(), get), urls)
    }

    #[test]
    fn names_follow_the_libretro_thumbnails_convention() {
        assert_eq!(
            thumbnail_name("Tom & Jerry: The \"Game\"/1"),
            "Tom _ Jerry_ The _Game__1"
        );
        assert_eq!(
            encode("Sega - Mega Drive - Genesis"),
            "Sega%20-%20Mega%20Drive%20-%20Genesis"
        );
        assert_eq!(encode("Pokémon (USA)"), "Pok%C3%A9mon%20%28USA%29");
    }

    #[test]
    fn builds_the_url_and_keeps_only_png_data() {
        let png = b"\x89PNG....".to_vec();
        let (a, urls) = art(Ok(Some(png.clone())));
        assert_eq!(
            a.boxart(&q("Nintendo - Game Boy", "Tetris (World)"))
                .unwrap(),
            Some(png)
        );
        assert_eq!(
            urls.lock().unwrap()[0],
            "https://t.test/Nintendo%20-%20Game%20Boy/Named_Boxarts/Tetris%20%28World%29.png"
        );
        let (a, _) = art(Ok(Some(b"<html>oops".to_vec())));
        assert_eq!(a.boxart(&q("x", "y")).unwrap(), None);
        let (a, _) = art(Ok(None));
        assert_eq!(a.boxart(&q("x", "y")).unwrap(), None);
        let (a, _) = art(Err("download failed: offline".into()));
        assert!(a.boxart(&q("x", "y")).unwrap_err().contains("offline"));
    }
}
