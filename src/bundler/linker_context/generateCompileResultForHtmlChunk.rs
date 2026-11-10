use crate::mal_prelude::*;

use bstr::BStr;

use bun_ast::Log;
use bun_ast::{ImportKind, ImportRecord, ImportRecordFlags};
use bun_core::strings;
use bun_threading::thread_pool::Task as ThreadPoolLibTask;
use lol_html::HandlerResult;
use lol_html::html_content::{ContentType, Element, EndTag};

use crate::HTMLScanner::{HTMLProcessor, HTMLProcessorHandler};
use crate::chunk;
use crate::linker_context_mod::{GenerateChunkCtx, LinkerContext, debug};
use crate::options::Loader;
use crate::{Chunk, CompileResult};

/// Rrewrite the HTML with the following transforms:
/// 1. Remove all <script> and <link> tags which were not marked as
///    external. This is defined by the source_index on the ImportRecord,
///    when it's not Index.invalid then we update it accordingly. This will
///    need to be a reference to the chunk or asset.
/// 2. For all other non-external URLs, update the "src" or "href"
///    attribute to point to the asset's unique key. Later, when joining
///    chunks, we will rewrite these to their final URL or pathname,
///    including the public_path.
/// 3. If a JavaScript chunk exists, add a <script type="module" crossorigin> tag that contains
///    the JavaScript for the entry point which uses the "src" attribute
///    to point to the JavaScript chunk's unique key.
/// 4. If a CSS chunk exists, add a <link rel="stylesheet" href="..." crossorigin> tag that contains
///    the CSS for the entry point which uses the "href" attribute to point to the
///    CSS chunk's unique key. A page that has a stylesheet <link> with `media` or
///    `disabled` has more than one CSS chunk, and the tag of the chunk of such a
///    <link> has the attributes of that <link> (`JavaScriptChunk::html_css_tags`).
/// 5. For each imported module or chunk within the JavaScript code, add
///    a <link rel="modulepreload" href="..." crossorigin> tag that
///    points to the module or chunk's unique key so that we tell the
///    browser to preload the user's code.
// CONCURRENCY: thread-pool callback — runs on worker threads, one task per
// HTML `PendingPartRange` (exactly one per HTML chunk). Writes:
// `chunk.compile_results_for_chunk[0]` (per-chunk disjoint). Reads
// `c.parse_graph.input_files` / `c.graph` / `ctx.chunks` shared. Never forms
// `&mut LinkerContext`. See `generate_compile_result_for_js_chunk` for the
// `PendingPartRange: Send` justification.
//
/// # Safety
///
/// `task` must be the intrusive `task` field of a live `PendingPartRange`
/// scheduled by `generate_chunks_in_parallel`; see
/// [`pending_part_range_prologue`](crate::linker_context_mod::pending_part_range_prologue)
/// for the full contract. Matches the `Task::callback: unsafe fn(*mut Task)`
/// contract.
pub(crate) unsafe fn generate_compile_result_for_html_chunk(task: *mut ThreadPoolLibTask) {
    // SAFETY: `task` is the intrusive `task` field of a `PendingPartRange`
    // scheduled by `generate_chunks_in_parallel`; see the helper's contract.
    let (part_range, chunk_ptr, _worker) =
        unsafe { crate::linker_context_mod::pending_part_range_prologue(task) };
    let i = part_range.i as usize;
    let ctx: &GenerateChunkCtx = part_range.ctx;

    // `ctx.c` is a `ParentRef` and `ctx.chunk` / `ctx.chunks` are `BackRef`s;
    // all yield safe shared borrows via `.get()`. `chunk` is this task's
    // exclusively-owned HTML chunk for the duration of the compile step.
    let c_ref: &LinkerContext = ctx.c.get();
    let chunk_ref: &Chunk = ctx.chunk.get();
    let chunks: &[Chunk] = ctx.chunks.get();
    let result = generate_compile_result_for_html_chunk_impl(c_ref, chunk_ref, chunks);
    // SAFETY: HTML chunks have exactly one part-range (i == 0); see
    // `Chunk::write_compile_result_slot` for the disjoint-slot contract.
    unsafe { Chunk::write_compile_result_slot(chunk_ptr, i, result) };
}

#[derive(Default)]
struct EndTagIndices {
    head: Option<u32>,
    body: Option<u32>,
    html: Option<u32>,
}

struct HTMLLoader<'a> {
    linker: &'a LinkerContext<'a>,
    import_records: &'a [ImportRecord<'a>],
    current_import_record_index: u32,
    /// This task's HTML chunk, an element of `chunks`.
    chunk: &'a Chunk<'a>,
    chunks: &'a [Chunk<'a>],
    /// This page's `JavaScriptChunk::html_css_tags` and the `css_chunks` that
    /// they index. Empty for the dev server, which links each stylesheet.
    css_tags: &'a [chunk::HtmlCssTag],
    css_chunks: &'a [u32],
    /// `(import record, attributes)` of each stylesheet `<link>` with `media`
    /// or `disabled` that this pass removed.
    conditional_links: Vec<(u32, String)>,
    /// Where the head tags go, for a page with `css_tags`. They are added
    /// after the pass, because a `<link>` that one of them needs can follow
    /// this point.
    head_tags_offset: Option<usize>,
    compile_to_standalone_html: bool,
    output: Vec<u8>,
    end_tag_indices: EndTagIndices,
    added_head_tags: bool,
    added_body_script: bool,
}

/// The attributes of a stylesheet `<link>`, for the tag that links the CSS
/// chunk of that stylesheet in its place: all of them but `href`, and but
/// `integrity`, which the author made for the stylesheet and not for the chunk.
fn link_attributes(link: &Element<'_, '_>, add_crossorigin: bool) -> String {
    let mut attributes = String::new();
    let mut has_crossorigin = false;
    for attribute in link.attributes() {
        let name = attribute.name();
        match name.as_str() {
            "href" | "integrity" => continue,
            "crossorigin" => has_crossorigin = true,
            _ => {}
        }
        attributes.push(' ');
        attributes.push_str(&name);
        // The value is the text of the source, character references included.
        let value = attribute.value();
        if !value.is_empty() {
            attributes.push_str("=\"");
            for c in value.chars() {
                match c {
                    '"' => attributes.push_str("&quot;"),
                    c => attributes.push(c),
                }
            }
            attributes.push('"');
        }
    }
    if add_crossorigin && !has_crossorigin {
        attributes.push_str(" crossorigin");
    }
    attributes
}

/// `Element::set_attribute` takes `&str`, so non-UTF-8 `name`/`value` bytes
/// fail the same way an invalid attribute name does.
fn set_attribute(element: &mut Element<'_, '_>, name: &[u8], value: &[u8]) {
    let ok = match (core::str::from_utf8(name), core::str::from_utf8(value)) {
        (Ok(name), Ok(value)) => element.set_attribute(name, value).is_ok(),
        _ => false,
    };
    if !ok {
        panic!("unexpected error from Element.setAttribute");
    }
}

/// `record` is a stylesheet `<link>` with `media` or `disabled`, and its
/// stylesheet is in the bundle.
fn is_conditional_stylesheet_link(record: &ImportRecord, loader: Loader) -> bool {
    record
        .flags
        .contains(ImportRecordFlags::HTML_LINK_CONDITIONAL)
        // The dev server does not load again a stylesheet that did not change.
        && (loader.is_css() || record.path.is_disabled)
}

impl<'a> HTMLProcessorHandler for HTMLLoader<'a> {
    fn on_write_html(&mut self, bytes: &[u8]) {
        self.output.extend_from_slice(bytes);
    }

    fn on_html_parse_error(&mut self, err: &[u8]) {
        bun_core::Output::panic(format_args!(
            "Parsing HTML during replacement phase errored, which should never happen since the first pass succeeded: {}",
            BStr::new(err)
        ));
    }

    fn on_tag(
        &mut self,
        element: &mut Element<'_, '_>,
        _path: &[u8],
        url_attribute: &[u8],
        _kind: ImportKind,
    ) {
        if self.current_import_record_index as usize >= self.import_records.len() {
            bun_core::Output::panic(format_args!(
                "Assertion failure in HTMLLoader.onTag: current_import_record_index ({}) >= import_records.len ({})",
                self.current_import_record_index,
                self.import_records.len()
            ));
        }

        let import_record_index = self.current_import_record_index;
        let import_record: &ImportRecord = &self.import_records[import_record_index as usize];
        self.current_import_record_index += 1;

        let parse_graph = self.linker.parse_graph();
        let unique_key_for_additional_files: &[u8] = if import_record.source_index.is_valid() {
            &parse_graph
                .input_files
                .items_unique_key_for_additional_file()[import_record.source_index.get() as usize]
        } else {
            b""
        };
        let loader: Loader = if import_record.source_index.is_valid() {
            parse_graph.input_files.items_loader()[import_record.source_index.get() as usize]
        } else {
            Loader::File
        };

        if import_record
            .flags
            .contains(ImportRecordFlags::IS_EXTERNAL_WITHOUT_SIDE_EFFECTS)
        {
            debug!(
                "Leaving external import: {}",
                BStr::new(import_record.path.text)
            );
            return;
        }

        if self.linker.dev_server.is_some() {
            if !unique_key_for_additional_files.is_empty() {
                set_attribute(element, url_attribute, unique_key_for_additional_files);
            } else if import_record.path.is_disabled
                || loader.is_javascript_like()
                || loader.is_css()
            {
                if is_conditional_stylesheet_link(import_record, loader) {
                    // The dev server makes the tag: see `CompileResult::Html::css_links`.
                    self.conditional_links
                        .push((import_record_index, link_attributes(element, false)));
                }
                element.remove();
            } else {
                set_attribute(element, url_attribute, import_record.path.pretty);
            }
            return;
        }

        if import_record.source_index.is_invalid() {
            debug!(
                "Leaving import with invalid source index: {}",
                BStr::new(import_record.path.text)
            );
            return;
        }

        if loader.is_javascript_like() || loader.is_css() {
            if is_conditional_stylesheet_link(import_record, loader) {
                self.conditional_links.push((
                    import_record_index,
                    link_attributes(element, !self.compile_to_standalone_html),
                ));
            }
            // Remove the original non-external tags
            element.remove();
            return;
        }

        if self.compile_to_standalone_html && import_record.source_index.is_valid() {
            // In standalone HTML mode, inline assets as data: URIs
            let url_for_css =
                parse_graph.ast.items_url_for_css()[import_record.source_index.get() as usize];
            if !url_for_css.is_empty() {
                set_attribute(element, url_attribute, url_for_css);
                return;
            }
        }

        if !unique_key_for_additional_files.is_empty() {
            // Replace the external href/src with the unique key so that we later will rewrite it to the final URL or pathname
            set_attribute(element, url_attribute, unique_key_for_additional_files);
            return;
        }
    }

    fn on_head_tag(&mut self, element: &mut Element<'_, '_>) -> bool {
        self.register_end_tag_handler(element, Self::end_head_tag_handler)
    }

    fn on_html_tag(&mut self, element: &mut Element<'_, '_>) -> bool {
        self.register_end_tag_handler(element, Self::end_html_tag_handler)
    }

    fn on_body_tag(&mut self, element: &mut Element<'_, '_>) -> bool {
        self.register_end_tag_handler(element, Self::end_body_tag_handler)
    }
}

/// An `HTMLLoader` end-tag callback. It receives the loader as an erased
/// `*mut ()` so the closure boxed into `Element::end_tag_handlers` captures
/// only `'static` data — `HTMLLoader<'a>` itself is not `'static`.
type EndTagHandlerFn = fn(*mut (), &mut EndTag<'_>) -> HandlerResult;

impl<'a> HTMLLoader<'a> {
    /// Arranges for `handler(self, end_tag)` to run when `element`'s end tag
    /// is reached. Returns `true` (stop the rewriter) if `element` cannot
    /// have an end tag.
    fn register_end_tag_handler(
        &mut self,
        element: &mut Element<'_, '_>,
        handler: EndTagHandlerFn,
    ) -> bool {
        let Some(handlers) = element.end_tag_handlers() else {
            return true;
        };
        // `self` points at the `HTMLLoader` that `HTMLProcessor::run` holds
        // for the whole rewriting pass, so the erased pointer is still live
        // when lol-html invokes `handler` at the end tag.
        let opaque_this = std::ptr::from_mut::<Self>(self).cast::<()>();
        handlers.push(Box::new(move |end| handler(opaque_this, end)));
        false
    }

    /// This is called for head, body, and html; whichever ends up coming first.
    fn add_head_tags(&mut self, end_tag: &mut EndTag<'_>) {
        if self.added_head_tags {
            return;
        }
        self.added_head_tags = true;

        if !self.css_tags.is_empty() {
            self.head_tags_offset = Some(self.output.len());
            return;
        }

        let tags = self.get_head_tags();
        for tag in &tags {
            end_tag.before(tag, ContentType::Html);
        }
    }

    /// Insert inline script before </body> so DOM elements are available.
    fn add_body_tags(&mut self, end_tag: &mut EndTag<'_>) {
        if self.added_body_script {
            return;
        }
        self.added_body_script = true;

        if let Some(script) = self.standalone_body_script() {
            end_tag.before(&script, ContentType::Html);
        }
    }

    /// The inline `<script type="module">…</script>` holding this HTML
    /// chunk's JavaScript chunk, if it has one.
    fn standalone_body_script(&self) -> Option<String> {
        let js_chunk = self.chunk.get_js_chunk_for_html(self.chunks)?;
        Some(format!(
            "<script type=\"module\">{}</script>",
            BStr::new(js_chunk.unique_key)
        ))
    }

    /// The tag that links `css_chunk`, or that holds it in standalone HTML.
    /// `link_attributes`: the chunk is the stylesheet of a `<link>` with
    /// `media` or `disabled`, and these are the attributes of that `<link>`.
    fn css_tag(&self, css_chunk: &Chunk, link_attributes: Option<&str>) -> String {
        let key = BStr::new(css_chunk.unique_key);
        match link_attributes {
            // In standalone HTML, the key of such a chunk becomes a `data:` URL.
            Some(attributes) => format!("<link{attributes} href=\"{key}\">"),
            None if self.compile_to_standalone_html => format!("<style>{key}</style>"),
            None => format!("<link rel=\"stylesheet\" crossorigin href=\"{key}\">"),
        }
    }

    fn add_css_tags(&self, array: &mut Vec<String>) {
        if self.css_tags.is_empty() {
            if let Some(css_chunk) = self.chunk.get_css_chunk_for_html(self.chunks) {
                array.push(self.css_tag(css_chunk, None));
            }
            return;
        }
        for tag in self.css_tags {
            let link_attributes = tag.link.map(|link| {
                let (_, attributes) = (self.conditional_links.iter())
                    .find(|(record, _)| *record == link)
                    .expect("the pass removed the <link> of each CSS chunk");
                attributes.as_str()
            });
            let css_chunk = &self.chunks[self.css_chunks[tag.css_chunk as usize] as usize];
            let tag = self.css_tag(css_chunk, link_attributes);
            // One `<link>` can match two rows of `TAG_HANDLERS`. It then has
            // two import records, and one tag is enough.
            if link_attributes.is_none() || array.last() != Some(&tag) {
                array.push(tag);
            }
        }
    }

    fn get_head_tags(&self) -> Vec<String> {
        let mut array: Vec<String> = Vec::with_capacity(2);
        if self.compile_to_standalone_html {
            // In standalone HTML mode, only put CSS in <head>; JS goes before </body>
            self.add_css_tags(&mut array);
        } else {
            // Put CSS before JS to reduce chances of flash of unstyled content
            self.add_css_tags(&mut array);
            if let Some(js_chunk_index) = self.chunk.get_js_chunk_index_for_html(self.chunks) {
                // type="module" scripts do not block rendering, so it is okay to put them in head
                array.push(format!(
                    "<script type=\"module\" crossorigin src=\"{}\"></script>",
                    BStr::new(self.chunks[js_chunk_index].unique_key)
                ));
                if self.linker.module_preload() {
                    let closure = bun_core::handle_oom(Chunk::reachable_chunks(
                        self.chunks,
                        js_chunk_index as u32,
                        &[bun_ast::ImportKind::Stmt],
                    ));
                    for &other in &closure[1..] {
                        array.push(format!(
                            "<link rel=\"modulepreload\" crossorigin href=\"{}\">",
                            BStr::new(self.chunks[other as usize].unique_key)
                        ));
                    }
                }
            }
        }
        array
    }

    fn end_head_tag_handler(opaque_this: *mut (), end: &mut EndTag<'_>) -> HandlerResult {
        // SAFETY: `opaque_this` is the erased `&mut HTMLLoader` from `register_end_tag_handler`.
        let this: &mut Self = unsafe { &mut *opaque_this.cast::<Self>() };
        if this.linker.dev_server.is_none() {
            this.add_head_tags(end);
        } else {
            this.end_tag_indices.head = Some(u32::try_from(this.output.len()).expect("int cast"));
        }
        Ok(())
    }

    fn end_body_tag_handler(opaque_this: *mut (), end: &mut EndTag<'_>) -> HandlerResult {
        // SAFETY: `opaque_this` is the erased `&mut HTMLLoader` from `register_end_tag_handler`.
        let this: &mut Self = unsafe { &mut *opaque_this.cast::<Self>() };
        if this.linker.dev_server.is_none() {
            if this.compile_to_standalone_html {
                // In standalone mode, insert JS before </body> so DOM is available
                this.add_body_tags(end);
            } else {
                this.add_head_tags(end);
            }
        } else {
            this.end_tag_indices.body = Some(u32::try_from(this.output.len()).expect("int cast"));
        }
        Ok(())
    }

    fn end_html_tag_handler(opaque_this: *mut (), end: &mut EndTag<'_>) -> HandlerResult {
        // SAFETY: `opaque_this` is the erased `&mut HTMLLoader` from `register_end_tag_handler`.
        let this: &mut Self = unsafe { &mut *opaque_this.cast::<Self>() };
        if this.linker.dev_server.is_none() {
            if this.compile_to_standalone_html {
                // Fallback: if no </body> was found, insert both CSS and JS before </html>
                this.add_head_tags(end);
                this.add_body_tags(end);
            } else {
                this.add_head_tags(end);
            }
        } else {
            this.end_tag_indices.html = Some(u32::try_from(this.output.len()).expect("int cast"));
        }
        Ok(())
    }
}

fn generate_compile_result_for_html_chunk_impl<'a>(
    c: &'a LinkerContext<'a>,
    chunk: &'a Chunk<'a>,
    chunks: &'a [Chunk<'a>],
) -> CompileResult {
    let parse_graph = c.parse_graph();
    let sources = parse_graph.input_files.items_source();
    let import_records = c.graph.ast.items_import_records();
    let source_index = chunk.entry_point.source_index();

    // HTML bundles for the dev server must outlive the bundle task (see
    // `DevServer.RouteBundle.HTML.bundled_html_text`). The output is built in a
    // plain `Vec<u8>` and returned as an owned `Box<[u8]>` inside
    // `CompileResult::Html`, so it lives as long as whoever holds the
    // `CompileResult` — no arena-lifetime distinction needed.

    // `c.log` is now `*mut Log` (raw backref); copy directly. The HTMLLoader.log
    // field is currently dead_code, so no write actually occurs through this
    // pointer today.
    let log: *mut Log = c.log;
    let minify_whitespace = c.options.minify_whitespace;
    let compile_to_standalone_html = c.options.compile_mode.is_standalone_html();
    let has_dev_server = c.dev_server.is_some();
    let contents: &[u8] = &sources[source_index as usize].contents;
    let records = import_records[source_index as usize].as_slice();

    let _ = (source_index, log, minify_whitespace);
    let mut css_tags: &[chunk::HtmlCssTag] = &[];
    let mut css_chunks: &[u32] = &[];
    if !has_dev_server
        && (records.iter()).any(|record| {
            record
                .flags
                .contains(ImportRecordFlags::HTML_LINK_CONDITIONAL)
        })
        && let Some(js_chunk) = chunk.get_js_chunk_for_html(chunks)
        && let chunk::Content::Javascript(js) = &js_chunk.content
    {
        css_tags = &js.html_css_tags;
        css_chunks = &js.css_chunks;
    }
    let mut html_loader = HTMLLoader {
        linker: c,
        import_records: records,
        current_import_record_index: 0,
        compile_to_standalone_html,
        chunk,
        chunks,
        css_tags,
        css_chunks,
        conditional_links: Vec::new(),
        head_tags_offset: None,
        output: Vec::new(),
        end_tag_indices: EndTagIndices {
            html: None,
            body: None,
            head: None,
        },
        added_head_tags: false,
        added_body_script: false,
    };

    HTMLProcessor::<HTMLLoader, true>::run(&mut html_loader, contents)
        .unwrap_or_else(|_| panic!("unexpected error from HTMLProcessor.run"));

    if let Some(offset) = html_loader.head_tags_offset {
        let tags = html_loader.get_head_tags().concat().into_bytes();
        drop(html_loader.output.splice(offset..offset, tags));
    }

    // There are some cases where invalid HTML will make it so </head> is
    // never emitted, even if the literal text DOES appear. These cases are
    // along the lines of having a self-closing tag for a non-self closing
    // element. In this case, head_end_tag_index will be 0, and a simple
    // search through the page is done to find the "</head>"
    // See https://github.com/oven-sh/bun/issues/17554
    let script_injection_offset: u32 = if has_dev_server {
        'brk: {
            if let Some(head) = html_loader.end_tag_indices.head {
                break 'brk head;
            }
            if let Some(head) = strings::index_of(&html_loader.output, b"</head>") {
                break 'brk u32::try_from(head).expect("int cast");
            }
            if let Some(body) = html_loader.end_tag_indices.body {
                break 'brk body;
            }
            if let Some(html) = html_loader.end_tag_indices.html {
                break 'brk html;
            }
            u32::try_from(html_loader.output.len()).expect("int cast") // inject at end of file.
        }
    } else {
        'brk: {
            if !html_loader.added_head_tags || !html_loader.added_body_script {
                // Cold path: the document is missing all of the head, body, and html elements.
                if !html_loader.added_head_tags {
                    let tags = html_loader.get_head_tags();
                    for tag in &tags {
                        html_loader.output.extend_from_slice(tag.as_bytes());
                    }
                    html_loader.added_head_tags = true;
                }
                if !html_loader.added_body_script {
                    if html_loader.compile_to_standalone_html {
                        if let Some(script) = html_loader.standalone_body_script() {
                            html_loader.output.extend_from_slice(script.as_bytes());
                        }
                    }
                    html_loader.added_body_script = true;
                }
            }
            // value is ignored
            break 'brk 0;
        }
    };

    // The dev server links each stylesheet of the page where `script_injection_offset`
    // is. It needs the attributes for the ones that a `<link>` with `media` or
    // `disabled` loads.
    let mut css_links: Vec<(u64, Box<[u8]>)> = Vec::new();
    if has_dev_server && !html_loader.conditional_links.is_empty() {
        let asset_of = |record: &ImportRecord| {
            crate::bake_types::css_asset_id(if record.source_index.is_valid() {
                &sources[record.source_index.get() as usize].path
            } else {
                &record.path
            })
        };
        for (record, attributes) in &html_loader.conditional_links {
            let link: (u64, Box<[u8]>) = (
                asset_of(&records[*record as usize]),
                attributes.as_bytes().into(),
            );
            // One `<link>` can match two rows of `TAG_HANDLERS`.
            if css_links.last() != Some(&link) {
                css_links.push(link);
            }
        }
        // A stylesheet that another `<link>` of the page loads too, without
        // `media` and `disabled`, applies whatever the first `<link>` says.
        let conditional = css_links.len();
        for record in records {
            if record.kind != ImportKind::At
                || (record.flags).contains(ImportRecordFlags::HTML_LINK_CONDITIONAL)
                || (record.flags).contains(ImportRecordFlags::IS_EXTERNAL_WITHOUT_SIDE_EFFECTS)
            {
                continue;
            }
            let asset = asset_of(record);
            if css_links[..conditional].iter().any(|(id, _)| *id == asset)
                && !css_links[conditional..].iter().any(|(id, _)| *id == asset)
            {
                css_links.push((asset, Box::default()));
            }
        }
    }

    CompileResult::Html {
        code: html_loader.output.into_boxed_slice(),
        source_index,
        script_injection_offset,
        css_links: css_links.into_boxed_slice(),
    }
}
