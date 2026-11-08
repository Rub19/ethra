use crate::net::net;
use crate::store::data_dir;
use base64::Engine;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::Semaphore;

const MAX_DIM: u32 = 1280;
const MAX_BYTES: usize = 220 * 1024 * 1024;
const MAX_AGE_DAYS: u64 = 30;

enum Decoded {
    Rgba(SharedPixelBuffer<Rgba8Pixel>),
    Svg(Vec<u8>),
}

thread_local! {
    static CACHE: RefCell<HashMap<String, Image>> = RefCell::new(HashMap::new());
    static PENDING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static USED: Cell<usize> = const { Cell::new(0) };
    static DIRTY: Cell<bool> = const { Cell::new(false) };
}

fn hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn disk(url: &str) -> PathBuf {
    data_dir().join("cache").join(format!("{:016x}", hash(url)))
}

fn fresh(p: &PathBuf) -> bool {
    std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|e| e.as_secs() < MAX_AGE_DAYS * 86400)
}

pub fn parse_data_url(url: &str) -> Option<Vec<u8>> {
    let (_, b64) = url.strip_prefix("data:")?.split_once(";base64,")?;
    base64::engine::general_purpose::STANDARD.decode(b64).ok()
}

pub fn decode_bytes(bytes: &[u8]) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let im = image::load_from_memory(bytes).ok()?;
    let im = if im.width() > MAX_DIM || im.height() > MAX_DIM { im.thumbnail(MAX_DIM, MAX_DIM) } else { im };
    let rgba = im.to_rgba8();
    Some(SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height()))
}

fn looks_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(256)];
    let t = String::from_utf8_lossy(head);
    t.contains("<svg") || t.trim_start().starts_with("<?xml")
}

fn grayscale(b: &SharedPixelBuffer<Rgba8Pixel>) -> SharedPixelBuffer<Rgba8Pixel> {
    let mut out = b.clone();
    for p in out.make_mut_slice() {
        let l = (0.3 * p.r as f32 + 0.59 * p.g as f32 + 0.11 * p.b as f32) as u8;
        p.r = l;
        p.g = l;
        p.b = l;
    }
    out
}

async fn fetch(full: String) -> Option<Decoded> {
    static GATE: OnceLock<Semaphore> = OnceLock::new();
    let _permit = GATE.get_or_init(|| Semaphore::new(8)).acquire().await.ok()?;
    let (gray, url) = match full.strip_prefix("gray:") {
        Some(u) => (true, u.to_string()),
        None => (false, full),
    };
    let bytes = if url.starts_with("data:") {
        parse_data_url(&url)?
    } else if let Some(game) = url.strip_prefix("local:") {
        crate::net::game_icon(game)?
    } else {
        let p = disk(&url);
        match std::fs::read(&p).ok().filter(|_| fresh(&p)) {
            Some(b) => b,
            None => {
                let r = net().http.get(&url).send().await.ok()?;
                if !r.status().is_success() {
                    return None;
                }
                let b = r.bytes().await.ok()?.to_vec();
                if let Some(dir) = p.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(&p, &b);
                b
            }
        }
    };
    if looks_svg(&bytes) {
        return Some(Decoded::Svg(bytes));
    }
    tokio::task::spawn_blocking(move || decode_bytes(&bytes).map(|b| Decoded::Rgba(if gray { grayscale(&b) } else { b }))).await.ok().flatten()
}

pub fn img(url: &str) -> Image {
    if url.is_empty() {
        return Image::default();
    }
    if let Some(i) = CACHE.with(|c| c.borrow().get(url).cloned()) {
        return i;
    }
    if PENDING.with(|p| p.borrow_mut().insert(url.to_string())) {
        let u = url.to_string();
        crate::spawn(async move {
            let d = fetch(u.clone()).await;
            let _ = slint::invoke_from_event_loop(move || {
                let image = match d {
                    Some(Decoded::Rgba(b)) => {
                        USED.with(|x| x.set(x.get() + (b.width() * b.height() * 4) as usize));
                        Some(Image::from_rgba8(b))
                    }
                    Some(Decoded::Svg(b)) => Image::load_from_svg_data(&b).ok(),
                    None => None,
                };
                if let Some(i) = image {
                    CACHE.with(|c| c.borrow_mut().insert(u, i));
                    DIRTY.with(|d| d.set(true));
                }
            });
        });
    }
    Image::default()
}

pub fn take_dirty() -> bool {
    DIRTY.with(|d| d.replace(false))
}

pub fn trim_if_needed() {
    if USED.with(|x| x.get()) > MAX_BYTES {
        CACHE.with(|c| c.borrow_mut().clear());
        PENDING.with(|p| p.borrow_mut().clear());
        USED.with(|x| x.set(0));
        DIRTY.with(|d| d.set(true));
    }
}

pub fn clear_disk_cache() -> u64 {
    let dir = data_dir().join("cache");
    let total = std::fs::read_dir(&dir).map(|l| l.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0);
    let _ = std::fs::remove_dir_all(&dir);
    total
}

pub fn disk_cache_size() -> u64 {
    std::fs::read_dir(data_dir().join("cache")).map(|l| l.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_urls() {
        assert_eq!(parse_data_url("data:image/png;base64,QUJD").unwrap(), b"ABC");
        assert!(parse_data_url("https://x").is_none());
        assert!(looks_svg(b"<?xml version=\"1.0\"?><svg>"));
        assert!(!looks_svg(b"\x89PNG"));
    }

    #[test]
    fn decode_png() {
        let mut out = std::io::Cursor::new(vec![]);
        image::RgbaImage::new(3, 2).write_to(&mut out, image::ImageFormat::Png).unwrap();
        let b = decode_bytes(out.get_ref()).unwrap();
        assert_eq!((b.width(), b.height()), (3, 2));
    }
}
