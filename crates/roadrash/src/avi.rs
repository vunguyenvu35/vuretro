//! `VIDEO/*.AVI`: RIFF files with a Cinepak video stream (`vids`/`cvid`) and a sound stream
//! (`auds`), the sound being IMA ADPCM in the files seen. Only what is needed to take the
//! frames and the sound blocks out is read; the rest of the file is skipped.

/// What the headers say about the sound.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundFormat {
    /// 0x0001 is PCM, 0x0011 is IMA ADPCM.
    pub tag: u16,
    pub channels: u16,
    pub rate: u32,
    pub block_bytes: u16,
}

/// A movie taken apart: nothing is decoded yet.
#[derive(Debug, Clone, PartialEq)]
pub struct Movie {
    pub width: u32,
    pub height: u32,
    /// Microseconds from one frame to the next.
    pub frame_us: u32,
    pub sound: Option<SoundFormat>,
    /// The compressed frames, in order.
    pub frames: Vec<Vec<u8>>,
    /// The sound chunks, in order.
    pub sound_chunks: Vec<Vec<u8>>,
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
}
fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|w| u16::from_le_bytes([w[0], w[1]]))
}

/// Walk the chunks in `b` (RIFF layout: a four letter name, a length, the data, padded to an
/// even size) and hand each to `visit` with its name and data. A LIST is entered.
fn walk(b: &[u8], visit: &mut dyn FnMut(&[u8], &[u8])) -> Result<(), String> {
    let mut at = 0;
    while at + 8 <= b.len() {
        let name = &b[at..at + 4];
        let len = u32_at(b, at + 4).unwrap_or(0) as usize;
        let end = (at + 8 + len).min(b.len()); // a cut last chunk is read as far as it goes
        let data = &b[at + 8..end];
        if name == b"LIST" && data.len() >= 4 {
            visit(&b"LIST"[..], &data[..4]);
            walk(&data[4..], visit)?;
        } else {
            visit(name, data);
        }
        at = end + (len & 1);
    }
    Ok(())
}

pub fn parse(file: &[u8]) -> Result<Movie, String> {
    if file.get(..4) != Some(b"RIFF") || file.get(8..12) != Some(b"AVI ") {
        return Err("not an AVI file".into());
    }
    let mut movie = Movie {
        width: 0,
        height: 0,
        frame_us: 0,
        sound: None,
        frames: Vec::new(),
        sound_chunks: Vec::new(),
    };
    // stream kinds in the order the headers list them: the chunk "00dc" belongs to stream 0
    let mut kinds: Vec<[u8; 4]> = Vec::new();
    let mut sound_stream = None;
    let mut video_stream = None;
    let mut error = None;
    walk(&file[12..], &mut |name, data| match name {
        b"avih" => movie.frame_us = u32_at(data, 0).unwrap_or(0),
        b"strh" => {
            if let Some(kind) = data.get(..4) {
                let k = [kind[0], kind[1], kind[2], kind[3]];
                match &k {
                    b"vids" => video_stream.get_or_insert(kinds.len()),
                    b"auds" => sound_stream.get_or_insert(kinds.len()),
                    _ => &mut 0,
                };
                kinds.push(k);
            }
        }
        b"strf" => match kinds.last() {
            Some(b"vids") => {
                movie.width = u32_at(data, 4).unwrap_or(0);
                movie.height = u32_at(data, 8).unwrap_or(0);
                if data.get(16..20) != Some(&b"cvid"[..]) {
                    error = Some("the video is not Cinepak".to_string());
                }
            }
            Some(b"auds") => {
                movie.sound = Some(SoundFormat {
                    tag: u16_at(data, 0).unwrap_or(0),
                    channels: u16_at(data, 2).unwrap_or(0),
                    rate: u32_at(data, 4).unwrap_or(0),
                    block_bytes: u16_at(data, 12).unwrap_or(0),
                });
            }
            _ => {}
        },
        n if n.len() == 4 && n[0].is_ascii_digit() && n[1].is_ascii_digit() => {
            let stream = ((n[0] - b'0') * 10 + (n[1] - b'0')) as usize;
            if Some(stream) == video_stream && (n.ends_with(b"dc") || n.ends_with(b"db")) {
                movie.frames.push(data.to_vec());
            } else if Some(stream) == sound_stream && n.ends_with(b"wb") {
                movie.sound_chunks.push(data.to_vec());
            }
        }
        _ => {}
    })?;
    if let Some(e) = error {
        return Err(e);
    }
    if movie.width == 0 || movie.height == 0 {
        return Err("the file has no video header".into());
    }
    Ok(movie)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut c = name.to_vec();
        c.extend_from_slice(&(data.len() as u32).to_le_bytes());
        c.extend_from_slice(data);
        if data.len() % 2 == 1 {
            c.push(0);
        }
        c
    }
    fn list(kind: &[u8; 4], inner: &[u8]) -> Vec<u8> {
        let mut d = kind.to_vec();
        d.extend_from_slice(inner);
        chunk(b"LIST", &d)
    }

    fn sample() -> Vec<u8> {
        let mut strh = b"vids".to_vec();
        strh.resize(56, 0);
        let mut strf = vec![0u8; 40];
        strf[4..8].copy_from_slice(&320u32.to_le_bytes());
        strf[8..12].copy_from_slice(&192u32.to_le_bytes());
        strf[16..20].copy_from_slice(b"cvid");
        let mut headers = chunk(b"avih", &66_667u32.to_le_bytes());
        headers.extend(list(
            b"strl",
            &[chunk(b"strh", &strh), chunk(b"strf", &strf)].concat(),
        ));
        let movi = [
            chunk(b"00dc", &[1, 2, 3]),
            chunk(b"01wb", &[9]),
            chunk(b"00dc", &[4]),
        ]
        .concat();
        let body = [list(b"hdrl", &headers), list(b"movi", &movi)].concat();
        let mut file = b"RIFF".to_vec();
        file.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
        file.extend_from_slice(b"AVI ");
        file.extend(body);
        file
    }

    #[test]
    fn the_size_the_rate_and_the_frames_are_taken_out() {
        let m = parse(&sample()).unwrap();
        assert_eq!((m.width, m.height, m.frame_us), (320, 192, 66_667));
        assert_eq!(m.frames, vec![vec![1, 2, 3], vec![4]]);
        assert!(m.sound.is_none());
    }

    #[test]
    fn other_files_and_cut_files_are_errors_not_panics() {
        assert!(parse(b"RIFF....WAVE").is_err());
        let s = sample();
        for cut in [3, 11, 40, s.len() - 5] {
            let _ = parse(&s[..cut]); // must not panic; whether it is an error depends on the cut
        }
        assert!(parse(&s[..40]).is_err());
    }
}
