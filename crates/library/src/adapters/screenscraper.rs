use serde_json::Value;
use std::io::Read;
use std::time::Duration;
use vuretro_core::ports::{ArtQuery, ArtSource};

const API: &str = "https://api.screenscraper.fr/api2/jeuInfos.php";
const MAX_BYTES: u64 = 8 * 1024 * 1024;
const SOFT_NAME: &str = "vuretro";
/// ScreenScraper limits how fast one account may ask; stay well under it.
const PAUSE: Duration = Duration::from_millis(1500);

/// `Ok(None)` is a clean "not found".
pub type Get = Box<dyn Fn(&str) -> Result<Option<Vec<u8>>, String> + Send + Sync>;

/// The developer credentials ScreenScraper issues to an application (not the user's own
/// account). The project owner registers vuretro with them and passes them in at runtime.
#[derive(Clone)]
pub struct DevLogin {
    pub id: String,
    pub password: String,
}

impl DevLogin {
    /// From `VURETRO_SS_DEVID` and `VURETRO_SS_DEVPASSWORD`.
    pub fn from_env() -> Option<Self> {
        Some(DevLogin {
            id: std::env::var("VURETRO_SS_DEVID")
                .ok()
                .filter(|v| !v.is_empty())?,
            password: std::env::var("VURETRO_SS_DEVPASSWORD")
                .ok()
                .filter(|v| !v.is_empty())?,
        })
    }
}

/// Box art from ScreenScraper, found by game name and system, with the user's account.
pub struct ScreenScraperArt {
    dev: DevLogin,
    get: Get,
    pause: Duration,
}

impl ScreenScraperArt {
    pub fn new(dev: DevLogin) -> Self {
        ScreenScraperArt {
            dev,
            get: Box::new(http_get),
            pause: PAUSE,
        }
    }

    pub fn with_get(dev: DevLogin, get: Get) -> Self {
        ScreenScraperArt {
            dev,
            get,
            pause: Duration::ZERO,
        }
    }
}

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

/// The URL of the best 2D box picture in a `jeuInfos` reply: world or US first, then
/// Europe, Japan, any other region.
pub fn pick_box_url(reply: &Value) -> Option<String> {
    let medias = reply.pointer("/response/jeu/medias")?.as_array()?;
    let boxes: Vec<&Value> = medias.iter().filter(|m| m["type"] == "box-2D").collect();
    let url_of = |m: &Value| m["url"].as_str().map(str::to_string);
    for region in ["wor", "us", "eu", "jp"] {
        if let Some(u) = boxes
            .iter()
            .find(|m| m["region"] == region)
            .and_then(|m| url_of(m))
        {
            return Some(u);
        }
    }
    boxes.iter().find_map(|m| url_of(m))
}

impl ArtSource for ScreenScraperArt {
    fn id(&self) -> &'static str {
        "screenscraper"
    }

    fn boxart(&self, q: &ArtQuery) -> Result<Option<Vec<u8>>, String> {
        let (Some((user, password)), true) = (q.login, q.screenscraper_id != 0) else {
            return Ok(None); // needs the user's account and a system it knows
        };
        std::thread::sleep(self.pause);
        let url = format!(
            "{API}?devid={}&devpassword={}&softname={SOFT_NAME}&output=json&ssid={}&sspassword={}&systemeid={}&romtype=rom&romnom={}",
            encode(&self.dev.id),
            encode(&self.dev.password),
            encode(user),
            encode(password),
            q.screenscraper_id,
            encode(q.name),
        );
        let Some(body) = (self.get)(&url)? else {
            return Ok(None);
        };
        // ScreenScraper answers errors as plain text, not JSON
        let Ok(reply) = serde_json::from_slice::<Value>(&body) else {
            return Err(format!(
                "ScreenScraper: {}",
                String::from_utf8_lossy(&body)
                    .chars()
                    .take(120)
                    .collect::<String>()
            ));
        };
        let Some(media) = pick_box_url(&reply) else {
            return Ok(None);
        };
        std::thread::sleep(self.pause);
        (self.get)(&media)
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

    const REPLY: &str = r#"{"response":{"jeu":{"medias":[
        {"type":"ss","region":"wor","url":"https://x/ss.png"},
        {"type":"box-2D","region":"jp","url":"https://x/jp.png"},
        {"type":"box-2D","region":"us","url":"https://x/us.png"},
        {"type":"box-3D","region":"wor","url":"https://x/3d.png"}]}}}"#;

    fn dev() -> DevLogin {
        DevLogin {
            id: "dev id".into(),
            password: "dp".into(),
        }
    }

    fn query<'a>(login: Option<(&'a str, &'a str)>, id: u32) -> ArtQuery<'a> {
        ArtQuery {
            name: "Super A (USA)",
            thumbnails: "",
            screenscraper_id: id,
            login,
        }
    }

    #[test]
    fn prefers_world_then_us_box_art_over_other_media() {
        let v: Value = serde_json::from_str(REPLY).unwrap();
        assert_eq!(pick_box_url(&v).as_deref(), Some("https://x/us.png"));
        let none: Value = serde_json::from_str(r#"{"response":{"jeu":{"medias":[]}}}"#).unwrap();
        assert_eq!(pick_box_url(&none), None);
        assert_eq!(pick_box_url(&Value::Null), None);
    }

    #[test]
    fn asks_with_the_account_then_downloads_the_picture() {
        let urls = Arc::new(Mutex::new(Vec::new()));
        let seen = urls.clone();
        let get: Get = Box::new(move |u| {
            seen.lock().unwrap().push(u.to_string());
            Ok(Some(if u.starts_with(API) {
                REPLY.as_bytes().to_vec()
            } else {
                b"\x89PNG-data".to_vec()
            }))
        });
        let art = ScreenScraperArt::with_get(dev(), get);
        let got = art.boxart(&query(Some(("me", "p&w")), 4)).unwrap();
        assert_eq!(got, Some(b"\x89PNG-data".to_vec()));
        let urls = urls.lock().unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls[0].contains("devid=dev%20id") && urls[0].contains("ssid=me"));
        assert!(urls[0].contains("sspassword=p%26w") && urls[0].contains("systemeid=4"));
        assert!(urls[0].contains("romnom=Super%20A%20%28USA%29"));
        assert_eq!(urls[1], "https://x/us.png");
    }

    #[test]
    fn without_an_account_or_a_known_system_nothing_is_sent() {
        let get: Get = Box::new(|_| panic!("no request expected"));
        let art = ScreenScraperArt::with_get(dev(), get);
        assert_eq!(art.boxart(&query(None, 4)).unwrap(), None);
        assert_eq!(art.boxart(&query(Some(("u", "p")), 0)).unwrap(), None);
    }

    #[test]
    fn plain_text_errors_from_the_server_are_reported() {
        let get: Get = Box::new(|_| {
            Ok(Some(
                b"Erreur de login : Verifiez vos identifiants".to_vec(),
            ))
        });
        let art = ScreenScraperArt::with_get(dev(), get);
        let err = art.boxart(&query(Some(("u", "p")), 4)).unwrap_err();
        assert!(err.contains("ScreenScraper") && err.contains("login"));
    }
}
