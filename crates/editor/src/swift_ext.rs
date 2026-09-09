use crate::{
    Addon, Editor,
    display_map::{BlockPlacement, BlockProperties, BlockStyle, CustomBlockId},
};
use gpui::{AppContext, Context, Entity, Window};
use language::Buffer;
use multi_buffer::{Anchor, MultiBuffer, MultiBufferOffset, MultiBufferSnapshot};
use settings::SoftWrap;
use std::{mem, ops::Range, sync::Arc};
use text::{BufferId, ToOffset};
use ui::{InteractiveElement, IntoElement, ParentElement, Styled, div};

pub struct InlineExpansions {
    blocks: Vec<CustomBlockId>,
}

impl Addon for InlineExpansions {
    fn to_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

pub fn show_inline_expansions(
    editor: &mut Editor,
    buffer_id: BufferId,
    expansions: &[(Range<lsp::Position>, String)],
    window: &mut Window,
    cx: &mut Context<Editor>,
) {
    let Some(buffer) = editor.buffer.read(cx).as_singleton() else {
        return;
    };
    let snapshot = buffer.read(cx).snapshot();
    if snapshot.remote_id() != buffer_id {
        return;
    }
    let Some(project) = editor.project.clone() else {
        return;
    };
    let language = buffer.read(cx).language().cloned();

    if clear_inline_expansions(editor, cx) {
        return;
    }

    let multi_snapshot = editor.buffer.read(cx).snapshot(cx);
    let mut blocks = Vec::new();
    for (range, content) in expansions {
        show_expansion(
            window,
            cx,
            &snapshot,
            &project,
            language.clone(),
            &multi_snapshot,
            &mut blocks,
            range,
            content,
        );
    }

    let ids = editor.insert_blocks(blocks, None, cx);
    editor.register_addon(InlineExpansions { blocks: ids });
}

fn show_expansion(
    window: &mut Window,
    cx: &mut Context<'_, Editor>,
    snapshot: &language::BufferSnapshot,
    project: &Entity<project::Project>,
    language: Option<Arc<language::Language>>,
    multi_snapshot: &MultiBufferSnapshot,
    blocks: &mut Vec<BlockProperties<Anchor>>,
    range: &Range<lsp::Position>,
    content: &String,
) {
    let expansion_buffer = project.update(cx, |project, cx| {
        project.create_local_buffer(content, language, false, cx)
    });

    let expansion_editor = create_expansion_editor(cx, expansion_buffer, window);

    let height = expansion_editor.update(cx, |e, cx| e.max_point(cx).row().0 + 1);
    let offset = language::point_from_lsp(range.start).to_offset(snapshot);

    blocks.push(create_block_properties(
        multi_snapshot,
        expansion_editor,
        height,
        offset,
    ));
}

fn create_block_properties(
    multi_snapshot: &MultiBufferSnapshot,
    expansion_editor: Entity<Editor>,
    height: u32,
    offset: usize,
) -> BlockProperties<Anchor> {
    BlockProperties {
        placement: BlockPlacement::Below(multi_snapshot.anchor_before(MultiBufferOffset(offset))),
        height: Some(height),
        style: BlockStyle::Flex,
        render: Arc::new(move |cx| {
            div()
                .block_mouse_except_scroll()
                .size_full()
                .h(height as f32 * cx.window.line_height())
                .pl(cx.margins.gutter.full_width())
                .child(expansion_editor.clone())
                .into_any_element()
        }),
        priority: 0,
    }
}

pub(super) fn clear_inline_expansions(editor: &mut Editor, cx: &mut Context<Editor>) -> bool {
    let blocks = editor
        .addon_mut::<InlineExpansions>()
        .map(|addon| mem::take(&mut addon.blocks))
        .unwrap_or_default();
    if blocks.is_empty() {
        return false;
    }
    editor.remove_blocks(blocks.into_iter().collect(), None, cx);
    true
}

fn create_expansion_editor(
    cx: &mut Context<'_, Editor>,
    expansion_buffer: Entity<Buffer>,
    window: &mut Window,
) -> Entity<Editor> {
    cx.new(|cx| {
        let multi_buffer = cx.new(|cx| MultiBuffer::singleton(expansion_buffer, cx));
        let mut editor = Editor::for_multibuffer(multi_buffer, None, window, cx);
        editor.disable_scrollbars_and_minimap(window, cx);
        editor.set_soft_wrap_mode(SoftWrap::None, cx);
        editor.set_show_wrap_guides(false, cx);
        editor.set_show_gutter(true, cx);
        editor.set_offset_content(true, cx);
        editor.disable_mouse_wheel_zoom();
        editor.set_forbid_vertical_scroll(true);
        editor.set_read_only(true);
        editor.set_show_edit_predictions(Some(false), window, cx);
        editor
    })
}
