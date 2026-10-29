// burrow's editor, the parts of it (index.html puts them together in a file's tile): CodeMirror 6 with the languages
// its sessions are likely to touch, and marked + DOMPurify for a markdown file shown rendered. build.sh bundles this into
// ../editor.js as window.Editor, so the page has no build step and needs no CDN for it.
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, highlightSpecialChars, drawSelection, dropCursor, rectangularSelection, crosshairCursor } from "@codemirror/view";
import { EditorState, Compartment } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { syntaxHighlighting, HighlightStyle, indentOnInput, bracketMatching, foldGutter, foldKeymap, StreamLanguage } from "@codemirror/language";
import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { tags } from "@lezer/highlight";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { markdown } from "@codemirror/lang-markdown";
import { json } from "@codemirror/lang-json";
import { css } from "@codemirror/lang-css";
import { html } from "@codemirror/lang-html";
import { rust } from "@codemirror/lang-rust";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { yaml } from "@codemirror/legacy-modes/mode/yaml";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { marked } from "marked";
import DOMPurify from "dompurify";

// a language by the file's extension (none: plain text)
const LANG = {
  js: () => javascript(), mjs: () => javascript(), cjs: () => javascript(), jsx: () => javascript({ jsx: true }),
  ts: () => javascript({ typescript: true }), tsx: () => javascript({ typescript: true, jsx: true }),
  py: () => python(), md: () => markdown(), markdown: () => markdown(), json: () => json(), css: () => css(),
  html: () => html(), htm: () => html(), rs: () => rust(),
  sh: () => StreamLanguage.define(shell), bash: () => StreamLanguage.define(shell), zsh: () => StreamLanguage.define(shell),
  yml: () => StreamLanguage.define(yaml), yaml: () => StreamLanguage.define(yaml), toml: () => StreamLanguage.define(toml),
};
const language = (name) => { const e = (name.split(".").pop() || "").toLowerCase(), f = LANG[e]; return f ? f() : []; };

window.Editor = {
  EditorView, EditorState, Compartment, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, highlightSpecialChars,
  drawSelection, dropCursor, rectangularSelection, crosshairCursor, defaultKeymap, history, historyKeymap, indentWithTab,
  searchKeymap, highlightSelectionMatches, syntaxHighlighting, HighlightStyle, indentOnInput, bracketMatching, foldGutter, foldKeymap,
  closeBrackets, closeBracketsKeymap, tags, language,
  // a markdown file as a page: its HTML cleaned of anything that runs (the page can open shells on your machines)
  render: (md) => DOMPurify.sanitize(marked.parse(md, { gfm: true }), { USE_PROFILES: { html: true }, FORBID_TAGS: ["style", "form", "input", "button", "iframe"], FORBID_ATTR: ["style"] }),
};
