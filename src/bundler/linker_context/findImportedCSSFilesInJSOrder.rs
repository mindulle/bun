use crate::mal_prelude::*;
use bun_alloc::Arena;
use bun_ast::{ImportRecord, ImportRecordFlags};
use bun_collections::VecExt;

use crate::{Index, LinkerContext};
use bun_collections::DynamicBitSet as BitSet;

/// JavaScript modules are traversed in depth-first postorder. This is the
/// order that JavaScript modules were evaluated in before the top-level await
/// feature was introduced.
///
/// ```text
///   A
///  / \
/// B   C
///  \ /
///   D
/// ```
///
/// If A imports B and then C, B imports D, and C imports D, then the JavaScript
/// traversal order is D B C A.
///
/// This function may deviate from ESM import order for dynamic imports (both
/// "require()" and "import()"). This is because the import order is impossible
/// to determine since the imports happen at run-time instead of compile-time.
/// In this case we just pick an arbitrary but consistent order.
pub(crate) fn find_imported_css_files_in_js_order(
    this: &LinkerContext,
    _temp: &Arena,
    entry_point: Index,
) -> Vec<Index> {
    if !(this.parse_graph().input_files.items_loader().iter()).any(|loader| loader.is_css()) {
        return Vec::new();
    }
    let mut visited = BitSet::init_empty(this.graph.files.len()).expect("oom");
    let mut order: Vec<Index> = Vec::new();
    visit_in_js_order(this, entry_point, entry_point, &mut visited, &mut order);
    order
}

/// The traversal of `find_imported_css_files_in_js_order`, from `start`. It
/// adds to `order` each CSS file that it reaches and that is not `visited`.
fn visit_in_js_order(
    this: &LinkerContext,
    entry_point: Index,
    start: Index,
    visited: &mut BitSet,
    order: &mut Vec<Index>,
) {
    let all_import_records = this.graph.ast.items_import_records();
    let all_loaders = this.parse_graph().input_files.items_loader();
    let all_parts = this.graph.ast.items_parts();

    // Explicit-stack DFS (was per-edge recursive). `Enter` pushes
    // successors in discovery order then reverses the tail so `Leave`
    // fires in the same depth-first postorder the recursion produced.
    #[derive(Copy, Clone)]
    enum Frame {
        Enter(Index),
        Leave(Index),
    }
    let mut stack: Vec<Frame> = vec![Frame::Enter(start)];

    while let Some(frame) = stack.pop() {
        let source_index = match frame {
            Frame::Leave(source_index) => {
                order.push(source_index);
                continue;
            }
            Frame::Enter(source_index) => source_index,
        };

        if visited.is_set(source_index.get() as usize) {
            continue;
        }
        visited.set(source_index.get() as usize);

        let records: &[ImportRecord] = all_import_records[source_index.get() as usize].as_slice();
        let p = &all_parts[source_index.get() as usize];

        let mark = stack.len();
        // Iterate over each part in the file in order
        for part in p.as_slice() {
            // Traverse any files imported by this part. Note that CommonJS calls
            // to "require()" count as imports too, sort of as if the part has an
            // ESM "import" statement in it. This may seem weird because ESM imports
            // are a compile-time concept while CommonJS imports are a run-time
            // concept. But we don't want to manipulate <style> tags at run-time so
            // this is the only way to do it.
            for &import_record_index in part.import_record_indices.slice() {
                let record = &records[import_record_index as usize];
                if record.source_index.is_valid()
                    && !visited.is_set(record.source_index.get() as usize)
                {
                    stack.push(Frame::Enter(record.source_index));
                }
            }
        }
        // Only CSS files contribute to `order`; skip the `Leave` for
        // everything else. The entry point was always visited with
        // `is_css = false` in the recursive form.
        if source_index != entry_point && all_loaders[source_index.get() as usize].is_css() {
            stack.push(Frame::Leave(source_index));
        }
        stack[mark..].reverse();
    }
}

/// The CSS chunks of an HTML page that has a stylesheet `<link>` with `media`
/// or `disabled` (`ImportRecordFlags::HTML_LINK_CONDITIONAL`), in cascade
/// order. `None` for every other page.
///
/// The stylesheet of such a link gets a chunk of its own: `(Some(the import
/// record of the link), [the stylesheet])`. That chunk cuts the order of
/// `find_imported_css_files_in_js_order` in two: `(None, stylesheets)` is the
/// CSS that the page's other links and its scripts load between two cuts.
pub(crate) fn find_css_chunks_of_html_page(
    this: &LinkerContext,
    page: Index,
) -> Option<Vec<(Option<u32>, Vec<Index>)>> {
    let records: &[ImportRecord] =
        this.graph.ast.items_import_records()[page.get() as usize].as_slice();
    let loaders = this.parse_graph().input_files.items_loader();
    let is_conditional = |record: &ImportRecord| {
        record
            .flags
            .contains(ImportRecordFlags::HTML_LINK_CONDITIONAL)
            && record.source_index.is_valid()
            && loaders[record.source_index.get() as usize].is_css()
    };
    if !records.iter().any(is_conditional) {
        return None;
    }

    let mut chunks: Vec<(Option<u32>, Vec<Index>)> = Vec::new();
    let mut between: Vec<Index> = Vec::new();
    let mut visited = BitSet::init_empty(this.graph.files.len()).expect("oom");
    visited.set(page.get() as usize);
    for (record_index, record) in records.iter().enumerate() {
        if is_conditional(record) {
            if !between.is_empty() {
                chunks.push((None, core::mem::take(&mut between)));
            }
            // Not `visited`: another link or a script that loads this
            // stylesheet applies it whatever this link says.
            chunks.push((Some(record_index as u32), vec![record.source_index]));
        } else if record.source_index.is_valid() {
            visit_in_js_order(this, page, record.source_index, &mut visited, &mut between);
        }
    }
    if !between.is_empty() {
        chunks.push((None, between));
    }
    Some(chunks)
}
