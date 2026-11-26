import { byId } from "./dom.js";

const MAX_BODY = 2000;
const MAX_IMAGE_BYTES = 1024 * 1024;
const EMOJI_ICON =
  '<svg viewBox="0 0 18 18" aria-hidden="true">' +
  '<circle class="ql-stroke" cx="9" cy="9" r="7"></circle>' +
  '<circle class="ql-fill" cx="6.5" cy="7.5" r="1.1"></circle>' +
  '<circle class="ql-fill" cx="11.5" cy="7.5" r="1.1"></circle>' +
  '<path class="ql-stroke" d="M6.2,11.2c.9,1.1,1.9,1.6,2.8,1.6s1.9-.5,2.8-1.6"></path>' +
  "</svg>";

let quill = null;

export function hasComposerDraft() {
  return quill !== null && quill.getText().trim() !== "";
}

function openComposer() {
  const panel = byId("composer-panel");
  if (!panel || typeof bootstrap === "undefined") {
    return;
  }
  bootstrap.Collapse.getOrCreateInstance(panel, { toggle: false }).show();
  window.setTimeout(function () {
    ensureQuill();
    if (quill) {
      quill.focus();
    }
  }, 180);
}

function closeComposer() {
  const panel = byId("composer-panel");
  if (!panel || typeof bootstrap === "undefined") {
    return;
  }
  bootstrap.Collapse.getOrCreateInstance(panel, { toggle: false }).hide();
  const trigger = byId("composer-trigger");
  if (trigger) {
    trigger.value = "";
  }
  hideEmojiPanel();
}

function syncEditor() {
  const hidden = byId("body");
  const counter = byId("composer-count");
  if (!quill || !hidden) {
    return;
  }
  const text = quill.getText().replace(/\n$/, "");
  if (text.length > MAX_BODY) {
    quill.deleteText(MAX_BODY, text.length);
  }
  hidden.value = quill.root.innerHTML;
  if (counter) {
    const len = quill.getText().replace(/\n$/, "").length;
    counter.textContent = String(len) + "/" + String(MAX_BODY);
  }
}

function insertEmoji(glyph) {
  ensureQuill();
  if (!quill || !glyph) {
    return;
  }
  const range = quill.getSelection(true);
  const index = range ? range.index : Math.max(0, quill.getLength() - 1);
  quill.insertText(index, glyph, "user");
  quill.setSelection(index + glyph.length, 0, "user");
  syncEditor();
  hideEmojiPanel();
}

function emojiPanel() {
  return document.querySelector("[data-clitorine-emoji-panel]");
}

function hideEmojiPanel() {
  const panel = emojiPanel();
  if (panel) {
    panel.hidden = true;
  }
}

function toggleEmojiPanel() {
  const panel = emojiPanel();
  if (panel) {
    panel.hidden = !panel.hidden;
  }
}

function onImagePick(input) {
  const preview = byId("media-preview");
  const file = input.files && input.files[0];
  if (!file) {
    return;
  }
  if (file.size > MAX_IMAGE_BYTES) {
    window.alert("Image must be 1 MB or smaller for this demo.");
    input.value = "";
    return;
  }
  if (!file.type.startsWith("image/")) {
    window.alert("Please choose an image file.");
    input.value = "";
    return;
  }
  if (preview) {
    const url = URL.createObjectURL(file);
    preview.innerHTML =
      '<img class="img-fluid rounded border" alt="preview" src="' +
      url +
      '" />';
  }
}

function ensureQuill() {
  const host = byId("composer-quill");
  if (!host || quill || typeof Quill === "undefined") {
    return;
  }
  const icons = Quill.import("ui/icons");
  icons.emoji = EMOJI_ICON;
  quill = new Quill("#composer-quill", {
    theme: "snow",
    placeholder: "Share a story, a thought, a clip…",
    modules: {
      toolbar: {
        container: [
          ["bold", "italic", "underline"],
          [{ list: "ordered" }, { list: "bullet" }],
          ["blockquote", "link"],
          ["emoji"],
        ],
        handlers: {
          emoji: function () {
            toggleEmojiPanel();
          },
        },
      },
    },
  });
  const initial = host.getAttribute("data-initial-html");
  if (initial) {
    quill.clipboard.dangerouslyPasteHTML(initial);
  }
  quill.on("text-change", syncEditor);
  syncEditor();
}

export function bindComposer() {
  const trigger = byId("composer-trigger");
  if (trigger) {
    trigger.addEventListener("focus", openComposer);
    trigger.addEventListener("click", openComposer);
  }
  const cancel = byId("composer-cancel");
  if (cancel) {
    cancel.addEventListener("click", function (event) {
      event.preventDefault();
      closeComposer();
    });
  }
  document.querySelectorAll("[data-clitorine-emoji]").forEach(function (btn) {
    btn.addEventListener("click", function (event) {
      event.preventDefault();
      insertEmoji(btn.getAttribute("data-clitorine-emoji") || "");
    });
  });
  const imageInput = byId("image");
  if (imageInput) {
    imageInput.addEventListener("change", function () {
      onImagePick(imageInput);
    });
  }
  const form = document.querySelector("form.composer-form");
  if (form) {
    form.addEventListener("submit", function () {
      ensureQuill();
      syncEditor();
    });
  }
  if (byId("composer-quill") && !byId("composer-panel")) {
    ensureQuill();
  } else if (document.body.dataset.composerOpen === "1") {
    openComposer();
  }
}
