<script lang="ts">
  import { onMount } from "svelte";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";
  import { Markdown } from "@tiptap/markdown";
  import Image from "@tiptap/extension-image";
  import { TableKit } from "@tiptap/extension-table";
  let {
    value = $bindable(""),
    editable = true,
    onreference,
    onpickreference,
  }: {
    value: string;
    editable?: boolean;
    onpickreference?: () => void;
    onreference?: (kind: string, id: string) => void;
  } = $props();
  let element: HTMLDivElement;
  let editor: Editor | undefined = $state.raw();
  let source = $state(false);
  let last = "";
  onMount(() => {
    editor = new Editor({
      element,
      extensions: [
        StarterKit.configure({
          link: { openOnClick: false, protocols: ["cheki", "attachment"] },
        }),
        Markdown,
        Image,
        TableKit,
      ],
      content: value,
      contentType: "markdown",
      editable,
      editorProps: {
        attributes: {
          class:
            "prose prose-sm max-w-none min-h-72 outline-none text-base-content",
          role: "textbox",
          "aria-label": "文章正文",
        },
        handleClick: (_view, _pos, event) => {
          const a = (event.target as Element).closest("a");
          if (a) {
            event.preventDefault();
            const match = /^(cheki|attachment):([a-zA-Z0-9-]+)$/.exec(
              a.getAttribute("href") ?? "",
            );
            if (match) onreference?.(match[1], match[2]);
            return true;
          }
          return false;
        },
      },
      onUpdate: ({ editor }) => {
        last = editor.getMarkdown();
        value = last;
      },
    });
    last = value;
    return () => editor?.destroy();
  });
  $effect(() => {
    if (editor) editor.setEditable(editable);
  });
  $effect(() => {
    if (editor && value !== last) {
      editor.commands.setContent(value, {
        contentType: "markdown",
        emitUpdate: false,
      });
      last = value;
    }
  });
  export function insertReference(label: string, uri: string) {
    const text = `[${label.replace(/[\[\]\\]/g, "\\$&")}](${uri})`;
    if (source) value += `\n\n${text}\n`;
    else
      editor
        ?.chain()
        .focus()
        .insertContent(text, { contentType: "markdown" })
        .run();
  }
</script>

<div class="space-y-3">
  {#if editable}<div
      class="flex flex-wrap items-center gap-1 border-b border-ink/5 pb-3"
    >
      <button
        type="button"
        class="btn btn-ghost btn-xs"
        disabled={source}
        onclick={() => editor?.chain().focus().toggleBold().run()}
        ><strong>B</strong></button
      >
      <button
        type="button"
        class="btn btn-ghost btn-xs italic"
        disabled={source}
        onclick={() => editor?.chain().focus().toggleItalic().run()}>I</button
      >
      <button
        type="button"
        class="btn btn-ghost btn-xs"
        disabled={source}
        onclick={() =>
          editor?.chain().focus().toggleHeading({ level: 2 }).run()}
        >标题</button
      >
      <button
        type="button"
        class="btn btn-ghost btn-xs"
        disabled={source}
        onclick={() => editor?.chain().focus().toggleBulletList().run()}
        >列表</button
      >
      <button
        type="button"
        class="btn btn-ghost btn-xs"
        disabled={source}
        onclick={() => editor?.chain().focus().toggleBlockquote().run()}
        >引文</button
      >
      <button
        type="button"
        class="btn btn-ghost btn-xs"
        onclick={() => onpickreference?.()}>引用收藏 / 附件</button
      >
      <button
        type="button"
        class="btn btn-ghost btn-xs"
        disabled={source}
        onclick={() => editor?.chain().focus().undo().run()}>撤销</button
      >
      <button
        type="button"
        class="btn btn-ghost btn-xs ml-auto"
        aria-pressed={source}
        onclick={() => (source = !source)}
        >{source ? "富文本" : "Markdown 源码"}</button
      >
    </div>{/if}
  <div bind:this={element} class:hidden={source} class="py-2"></div>
  {#if source}<textarea
      class="textarea min-h-80 w-full border-0 bg-surface/20 font-mono text-sm"
      aria-label="Markdown 源码"
      bind:value
      readonly={!editable}></textarea>{/if}
</div>
