use super::rendering::prepared_source_image;
use super::{
    Arc, ColorMode, Document, EngineError, JobCtx, Layer, LayerContent, LayerId, Metadata, Mutex, PixelFormat, Rect, Result, SampleType, Session, SmartObject,
    SmartSource, Surface, base_name, json, other,
};

/// A linked file's bytes; `None` when it is missing or its path is not followed
/// ([`crate::doc_path::resolve`]).
///
/// The path is document data, not a request parameter, so every read rechecks it with the gate
/// of the session the work is done for: a filter, transform or refresh re-renders from here,
/// not only the contents commands that judge it up front. A refused path is handled like a
/// missing file: the layer keeps its last rendering.
fn read_file(path: &str) -> Option<Vec<u8>> {
    if path.is_empty() {
        return None;
    }
    crate::jobs::authorize_render_command("layer.smartObjects.updateModifiedContent", &json!({"path": path})).ok()?;
    photocraft_format::read_file(&crate::doc_path::resolve(path).ok()?).ok()
}

/// The source file of a smart object: embedded bytes, the PSD's embedded linked-layer data (PSD
/// import keeps the placed layer's uuid as the path), or a linked file on disk.
pub fn source_bytes(meta: &Metadata, src: &SmartSource) -> Option<(String, Arc<Vec<u8>>)> {
    embedded_source_bytes(meta, src).or_else(|| match src {
        SmartSource::Linked { path } => read_file(path).map(|b| (base_name(path), Arc::new(b))),
        SmartSource::Embedded { .. } => None,
    })
}

/// Memory-only resolution also covers Photoshop's embedded files addressed by a `Linked` UUID.
pub(crate) fn embedded_source_bytes(meta: &Metadata, src: &SmartSource) -> Option<(String, Arc<Vec<u8>>)> {
    match src {
        SmartSource::Embedded { file_name, bytes } => Some((file_name.clone(), bytes.clone())),
        SmartSource::Linked { path } => photocraft_io::linked::find_linked_file(meta, path).map(|f| (f.file_name, Arc::new(f.bytes))),
    }
}

/// The actual path is stored in the document, not in the command's params. Recheck it before
/// reading, reusing embedded PSD bytes rather than parsing their global blocks a second time.
pub(super) fn authorized_source_bytes(s: &Session, cmd: &str, meta: &Metadata, src: &SmartSource) -> Result<Option<(String, Arc<Vec<u8>>)>> {
    if let Some(source) = embedded_source_bytes(meta, src) {
        return Ok(Some(source));
    }
    let SmartSource::Linked { path } = src else { return Ok(None) };
    if let Some(gate) = s.authorize {
        gate(cmd, &json!({"path": path}))?;
    }
    Ok(read_file(path).map(|b| (base_name(path), Arc::new(b))))
}

/// Recheck a source's actual path before an untrusted command reads it: the path lives in the
/// document, not in the command's params. Embedded PSD UUIDs never reach the filesystem.
pub(super) fn authorize_source(s: &Session, cmd: &str, doc: &Document, src: &SmartSource) -> Result<()> {
    if let (Some(gate), SmartSource::Linked { path }) = (s.authorize, src)
        && embedded_source_bytes(&doc.metadata, src).is_none()
    {
        gate(cmd, &json!({"path": path}))?;
    }
    Ok(())
}

/// A decoded, composited source in its own pixel space (`bounds` starts at the origin).
#[derive(Clone, Debug)]
pub struct SourceImage {
    pub surface: Arc<Surface>,
    pub bounds: Rect,
}

type CacheKey = ([u8; 32], PixelFormat);

struct CacheEntry {
    key: CacheKey,
    image: SourceImage,
    decodes: u32,
    reusable_without_preparation: bool,
}

/// Decoded source composites by content hash, most recent last. Sources proven free of Smart
/// content reuse decoded pixels; other sources revalidate stored commands under the current authority.
static SOURCE_CACHE: Mutex<Vec<CacheEntry>> = Mutex::new(Vec::new());

const CACHE_ENTRIES: usize = 32;

const CACHE_BYTES: usize = 1 << 30;

pub(super) fn cache_key(bytes: &[u8], fmt: PixelFormat) -> CacheKey {
    (*blake3::hash(bytes).as_bytes(), fmt)
}

pub(super) fn cache_get(key: &CacheKey) -> Option<SourceImage> {
    cache_get_counted(key, false)
}

pub(super) fn cache_put(key: CacheKey, image: SourceImage, reusable_without_preparation: bool, decoded: bool) {
    let Ok(mut c) = SOURCE_CACHE.lock() else { return };
    let decodes = match c.iter().position(|e| e.key == key) {
        Some(i) => c.remove(i).decodes.saturating_add(u32::from(decoded)),
        None => u32::from(decoded),
    };
    c.push(CacheEntry { key, image, decodes, reusable_without_preparation });
    let size = |e: &CacheEntry| e.image.bounds.width() as usize * e.image.bounds.height() as usize * e.key.1.channels() * 4;
    while c.len() > CACHE_ENTRIES || (c.len() > 1 && c.iter().map(size).sum::<usize>() > CACHE_BYTES) {
        c.remove(0);
    }
}

/// Successful source-image decodes since this cache entry was created (zero for seeded composites or evicted entries).
/// Direct uncached [`decode_source`] calls are excluded; the diagnostic saturates at `u32::MAX`.
pub fn source_decode_count(bytes: &[u8], fmt: PixelFormat) -> u32 {
    let key = cache_key(bytes, fmt);
    SOURCE_CACHE.lock().ok().and_then(|c| c.iter().find(|e| e.key == key).map(|e| e.decodes)).unwrap_or(0)
}

pub(crate) fn take_caches(layers: &mut [Layer]) -> Vec<(LayerId, Surface)> {
    let mut out = Vec::new();
    for l in layers {
        match &mut l.content {
            LayerContent::Smart(sm) => out.extend(sm.cache.take().map(|c| (l.id, c))),
            LayerContent::Group(g) => out.extend(take_caches(&mut g.children)),
            _ => {}
        }
    }
    out
}

pub(crate) fn restore_unrendered<E>(
    doc: &mut Document,
    taken: Vec<(LayerId, Surface)>,
    mut fallback: impl FnMut(&Surface) -> std::result::Result<Surface, E>,
) -> std::result::Result<(), E> {
    for (id, cache) in taken {
        if let Some(LayerContent::Smart(sm)) = doc.layer_mut(id).map(|l| &mut l.content)
            && sm.cache.is_none()
        {
            sm.cache = Some(fallback(&cache)?);
        }
    }
    Ok(())
}

pub(super) fn buffer_to_surface(buf: &photocraft_compose::Buffer, fmt: PixelFormat) -> Surface {
    let n = fmt.channels();
    let mut data = vec![0.0f32; buf.px.len() * n];
    for (p, out) in buf.px.iter().zip(data.chunks_exact_mut(n)) {
        photocraft_raster::from_rgba_into(&fmt, *p, out);
    }
    let mut s = Surface::new(fmt);
    if !buf.rect.is_empty() {
        s.write_region(buf.rect, &data);
    }
    s.prune();
    s
}

pub(super) fn source_surface(src: &Document, buf: &photocraft_compose::Buffer, fmt: PixelFormat) -> Result<Surface> {
    let gray = fmt.mode == ColorMode::Grayscale && src.pixel_format().mode != ColorMode::Grayscale;
    let linear = fmt.sample == SampleType::F32 && src.depth != SampleType::F32;
    if !gray && !linear {
        return Ok(buffer_to_surface(buf, fmt));
    }
    let mut doc = Document::new(src.name.clone(), src.size, src.mode, src.depth);
    doc.icc_profile = src.icc_profile.clone();
    doc.layers = vec![Layer::new("source", LayerContent::Raster(buffer_to_surface(buf, src.pixel_format())))];
    if gray {
        let gray_profile = crate::color_cmds::working_profile(ColorMode::Grayscale);
        crate::color_cmds::convert_document(&mut doc, &gray_profile, photocraft_cms::Intent::RelativeColorimetric, true)?;
    }
    if linear {
        crate::image_cmds::change_depth(&mut doc, SampleType::F32);
    }
    let surface = doc.layers.pop().and_then(|l| match l.content {
        LayerContent::Raster(s) => Some(s),
        _ => None,
    });
    let surface = surface.ok_or_else(|| other("the smart object's source could not be converted"))?;
    Ok(if surface.format() == fmt { surface } else { surface.convert(fmt) })
}

/// An SVG source rendered at `scale` times its own size (cached per scale), so a vector smart
/// object stays sharp however it is placed.
pub fn svg_source_image(bytes: &[u8], fmt: PixelFormat, scale: f64) -> Result<SourceImage> {
    let mut h = blake3::Hasher::new();
    h.update(bytes);
    h.update(&scale.to_bits().to_le_bytes());
    let key = (*h.finalize().as_bytes(), fmt);
    if let Some(img) = cache_get(&key) {
        return Ok(img);
    }
    let buf = photocraft_io::svg::rasterize(bytes, scale as f32).map_err(|e| other(format!("can't render the SVG smart object: {e}")))?;
    let img = SourceImage { surface: Arc::new(buffer_to_surface(&buf, fmt)), bounds: buf.rect };
    cache_put(key, img.clone(), true, true);
    Ok(img)
}

/// The scale a vector source is rasterised at for its placement: the larger axis scale of the
/// transform in 1/16 steps (so nudges reuse the cache), between 1/16 and 64.
pub(super) fn vector_scale(sm: &SmartObject) -> f64 {
    let [a, b, c, d, ..] = sm.transform.m;
    let k = (a * a + b * b).sqrt().max((c * c + d * d).sqrt());
    if !k.is_finite() {
        return 1.0;
    }
    ((k * 16.0).ceil() / 16.0).clamp(1.0 / 16.0, 64.0)
}

/// The composited source image in `fmt`; raw Smart-free sources reuse caches and other sources recheck authority.
pub fn source_image(file_name: &str, bytes: &[u8], fmt: PixelFormat) -> Result<SourceImage> {
    source_image_in(file_name, bytes, fmt, &JobCtx::new())
}

/// Prepare one externally supplied source; nested contents share the preparer's ledger rather than re-entering here.
pub(super) fn decode_source_in(file_name: &str, bytes: &[u8], ctx: &JobCtx) -> Result<Document> {
    decode_source_admitted_in(file_name, bytes, ctx).map(|(document, _)| document)
}

/// Establish cache reuse eligibility from raw native structure before preparation can materialize stored actions.
pub(super) fn decode_source_admitted_in(file_name: &str, bytes: &[u8], ctx: &JobCtx) -> Result<(Document, bool)> {
    ctx.check()?;
    crate::allocation::checkpoint("importing document")?;
    let imported = ctx
        .stage(0.0, 0.5, "Decoding Smart contents", |ctl| photocraft_io::import_with_options_and_interrupt(file_name, bytes, &Default::default(), ctl))
        .map_err(|e| if ctx.cancelled() { EngineError::Cancelled } else { other(format!("can't read smart object contents \"{file_name}\": {e}")) })?;
    if imported.preview_only {
        return Err(other(format!(
            "{file_name}: only this Affinity file's embedded preview could be read; open it with File › Open to see the warning, or export PSD or PNG from Affinity before using it here"
        )));
    }
    let mut doc = imported.document;
    let reusable = source_without_stored_commands(&doc, ctx)?;
    crate::import_prepare::prepare_imported_document(&mut doc, ctx)?;
    Ok((doc, reusable))
}

/// A bounded iterative raw-layer walk proves cache reuse cannot bypass Smart source or filter replay.
pub(super) fn source_without_stored_commands(doc: &Document, ctx: &JobCtx) -> Result<bool> {
    let mut groups = vec![doc.layers.iter()];
    while let Some(children) = groups.last_mut() {
        ctx.check()?;
        let Some(layer) = children.next() else {
            groups.pop();
            continue;
        };
        if groups.len() > photocraft_doc::MAX_GROUP_DEPTH + 1 {
            return Ok(false);
        }
        if matches!(layer.content, LayerContent::Smart(_)) {
            return Ok(false);
        }
        if let Some(children) = layer.children() {
            groups.push(children.iter());
        }
    }
    Ok(true)
}

/// Reuse only a raw-decoder proof, never an extension or a post-preparation document that hides pending actions.
pub(super) fn cache_get_without_preparation(key: &CacheKey) -> Option<SourceImage> {
    let mut cache = SOURCE_CACHE.lock().ok()?;
    let index = cache.iter().position(|entry| entry.key == *key && entry.reusable_without_preparation)?;
    let entry = cache.remove(index);
    let image = entry.image.clone();
    cache.push(entry);
    Some(image)
}

/// Count successful source redecodes even when the already composed image remains reusable.
pub(super) fn cache_get_counted(key: &CacheKey, decoded: bool) -> Option<SourceImage> {
    let mut c = SOURCE_CACHE.lock().ok()?;
    let i = c.iter().position(|e| e.key == *key)?;
    let mut e = c.remove(i);
    e.decodes = e.decodes.saturating_add(u32::from(decoded));
    let img = e.image.clone();
    c.push(e);
    Some(img)
}

/// Check stored source authority before consulting a process-wide rendered-source cache.
pub(super) fn source_image_in(file_name: &str, bytes: &[u8], fmt: PixelFormat, ctx: &JobCtx) -> Result<SourceImage> {
    ctx.check()?;
    let key = cache_key(bytes, fmt);
    if let Some(image) = cache_get_without_preparation(&key) {
        return Ok(image);
    }
    let (doc, reusable) = decode_source_admitted_in(file_name, bytes, ctx)?;
    if let Some(img) = cache_get_counted(&key, true) {
        return Ok(img);
    }
    let img = prepared_source_image(&doc, fmt, None, ctx)?;
    cache_put(key, img.clone(), reusable, true);
    Ok(img)
}
